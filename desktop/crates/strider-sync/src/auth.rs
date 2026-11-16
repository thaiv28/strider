use crate::{Client, Result, SyncError};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::TryRngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};
use url::Url;
#[derive(Deserialize)]
pub struct User {
    pub email: String,
    pub name: Option<String>,
}
/// Contains a credential; deliberately has no Debug implementation.
#[derive(Deserialize)]
pub struct Session {
    pub token: String,
    pub user: User,
}
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
fn random() -> Result<String> {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| SyncError::Protocol("Random source unavailable".into()))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
pub fn callback(request: &str, state: &str) -> Result<String> {
    let mut line = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    if line.next() != Some("GET") {
        return Err(SyncError::Protocol("Expected callback GET".into()));
    }
    let target = line
        .next()
        .filter(|s| s.starts_with("/callback?"))
        .ok_or_else(|| SyncError::Protocol("Invalid callback path".into()))?;
    let url = Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| SyncError::Protocol("Invalid callback".into()))?;
    let pairs = url.query_pairs().collect::<Vec<_>>();
    let get = |key: &str| -> Result<String> {
        let values = pairs.iter().filter(|(k, _)| k == key).collect::<Vec<_>>();
        if values.len() != 1 {
            return Err(SyncError::Protocol(format!(
                "Missing or duplicate callback {key}"
            )));
        }
        Ok(values[0].1.to_string())
    };
    if get("state")? != state {
        return Err(SyncError::Protocol("Sign-in state did not match".into()));
    }
    if pairs.iter().any(|(k, _)| k == "error") {
        return Err(SyncError::Protocol("Sign-in was declined".into()));
    }
    let code = get("code")?;
    if code.len() != 43
        || !code
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err(SyncError::Protocol("Invalid authorization code".into()));
    }
    Ok(code)
}
pub fn sign_in(base_url: &str, device_name: &str, open_url: impl Fn(&Url)) -> Result<Session> {
    if device_name.trim().is_empty() || device_name.chars().count() > 60 {
        return Err(SyncError::Protocol(
            "Device name must contain 1–60 characters".into(),
        ));
    }
    let client = Client::new(base_url, None)?;
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|_| SyncError::Protocol("Could not open sign-in callback".into()))?;
    listener
        .set_nonblocking(true)
        .map_err(|_| SyncError::Protocol("Could not configure callback".into()))?;
    let port = listener
        .local_addr()
        .map_err(|_| SyncError::Protocol("Could not read callback port".into()))?
        .port();
    let verifier = random()?;
    let state = random()?;
    let mut url = client
        .base_url
        .join("/desktop/authorize")
        .map_err(|_| SyncError::Protocol("Invalid server URL".into()))?;
    url.query_pairs_mut()
        .append_pair("code_challenge", &challenge(&verifier))
        .append_pair("code_challenge_method", "S256")
        .append_pair("redirect_uri", &format!("http://127.0.0.1:{port}/callback"))
        .append_pair("device_name", device_name)
        .append_pair("state", &state);
    open_url(&url);
    let deadline = Instant::now() + Duration::from_secs(300);
    let mut socket = loop {
        match listener.accept() {
            Ok((s, _)) => break s,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(50))
            }
            Err(_) => return Err(SyncError::Protocol("Sign-in callback timed out".into())),
        }
    };
    socket
        .set_read_timeout(Some(
            deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(10)),
        ))
        .map_err(|_| SyncError::Offline)?;
    socket
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| SyncError::Offline)?;
    let mut request = Vec::new();
    let mut byte = [0u8; 1];
    while request.len() < 8192 {
        if socket
            .read(&mut byte)
            .map_err(|_| SyncError::Protocol("Could not read callback".into()))?
            == 0
        {
            break;
        }
        request.push(byte[0]);
        if request.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let code = std::str::from_utf8(&request)
        .map_err(|_| SyncError::Protocol("Invalid callback encoding".into()))
        .and_then(|r| callback(r, &state));
    let message = if code.is_ok() {
        "Strider is signed in. You can close this tab."
    } else {
        "Strider sign-in failed. Close this tab and try again in the app."
    };
    let status = if code.is_ok() {
        "200 OK"
    } else {
        "400 Bad Request"
    };
    let _ = write!(
        socket,
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{message}",
        message.len()
    );
    drop(socket);
    code.and_then(|code| client.request("POST","/api/v1/auth/token",Some(&serde_json::json!({"code":code,"code_verifier":verifier,"device_name":device_name})))).and_then(|v| serde_json::from_value(v).map_err(|_| SyncError::Protocol("Invalid session response".into())))
}
pub fn sign_out(client: &Client) -> Result<()> {
    client
        .request("DELETE", "/api/v1/auth/token", None)
        .map(|_| ())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pkce() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mJ92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "ngF5GsXcbwljx6u133FFr3Xht9xooA_DuaX_3QwODtc"
        );
    }
    #[test]
    fn callbacks() {
        let r = format!(
            "GET /callback?state=abc&code={} HTTP/1.1\r\n\r\n",
            "x".repeat(43)
        );
        assert!(callback(&r, "abc").is_ok());
        assert!(callback(&r, "wrong").is_err());
        assert!(callback(&r.replace("GET", "POST"), "abc").is_err());
        assert!(callback(&r.replace("state=abc", "state=abc&state=abc"), "abc").is_err());
        assert!(callback("GET /callback?state=abc&error=denied HTTP/1.1", "abc").is_err());
    }
}
