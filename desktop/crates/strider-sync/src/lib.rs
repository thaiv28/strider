//! Blocking device authentication and durable push/pull synchronization.
pub mod auth;
pub mod engine;
pub mod keychain;
use serde_json::Value;
use std::{fmt, time::Duration};
use url::Url;
#[derive(Debug, PartialEq, Eq)]
pub enum SyncError {
    Offline,
    Unauthorized,
    UpgradeRequired,
    Rejected { index: usize, reason: String },
    Server(u16),
    Protocol(String),
}
pub type Result<T> = std::result::Result<T, SyncError>;
impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offline => write!(f, "Offline"),
            Self::Unauthorized => write!(f, "Sign in to sync"),
            Self::UpgradeRequired => write!(f, "Update Strider to sync"),
            Self::Rejected { index, reason } => write!(f, "Operation {index}: {reason}"),
            Self::Server(s) => write!(f, "Server returned {s}"),
            Self::Protocol(s) => write!(f, "{s}"),
        }
    }
}
impl std::error::Error for SyncError {}
impl From<strider_store::Error> for SyncError {
    fn from(e: strider_store::Error) -> Self {
        Self::Protocol(e.to_string())
    }
}
#[derive(Clone)]
pub struct Client {
    pub base_url: Url,
    pub token: Option<String>,
    pub app_version: String,
    agent: ureq::Agent,
}
impl Client {
    pub fn new(base_url: &str, token: Option<String>) -> Result<Self> {
        let base_url =
            Url::parse(base_url).map_err(|_| SyncError::Protocol("Invalid server URL".into()))?;
        if !matches!(base_url.scheme(), "http" | "https")
            || base_url.host_str().is_none()
            || !base_url.username().is_empty()
            || base_url.password().is_some()
        {
            return Err(SyncError::Protocol("Invalid server URL".into()));
        }
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self {
            base_url,
            token,
            app_version: env!("CARGO_PKG_VERSION").into(),
            agent,
        })
    }
    pub fn request(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value> {
        let url = self
            .base_url
            .join(path)
            .map_err(|_| SyncError::Protocol("Invalid API path".into()))?;
        let mut request = ureq::http::Request::builder()
            .method(method)
            .uri(url.as_str())
            .header("Accept", "application/json")
            .header("X-Strider-App-Version", &self.app_version);
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        let bytes = body
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|_| SyncError::Protocol("Invalid request JSON".into()))?
            .unwrap_or_default();
        if body.is_some() {
            request = request.header("Content-Type", "application/json");
        }
        let request = request
            .body(bytes)
            .map_err(|_| SyncError::Protocol("Invalid request".into()))?;
        let mut response = self.agent.run(request).map_err(|_| SyncError::Offline)?;
        let status = response.status().as_u16();
        match status {
            401 => return Err(SyncError::Unauthorized),
            426 => return Err(SyncError::UpgradeRequired),
            500..=599 => return Err(SyncError::Server(status)),
            _ => (),
        }
        let value: Value = response.body_mut().read_json().map_err(|e| match e {
            ureq::Error::Io(_) | ureq::Error::Timeout(_) => SyncError::Offline,
            _ => SyncError::Protocol("Invalid JSON response".into()),
        })?;
        if status == 409 {
            let r = &value["rejected"];
            return Err(SyncError::Rejected {
                index: r["index"]
                    .as_u64()
                    .ok_or_else(|| SyncError::Protocol("Missing rejection index".into()))?
                    as usize,
                reason: r["reason"]
                    .as_str()
                    .ok_or_else(|| SyncError::Protocol("Missing rejection reason".into()))?
                    .into(),
            });
        }
        if !(200..300).contains(&status) {
            return Err(SyncError::Server(status));
        }
        Ok(value)
    }
}
/// Injectable boundary for deterministic offline sync/permit tests.
pub trait Transport {
    fn origin(&self) -> String;
    fn request(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value>;
    fn upload_permit(&self, trip: &str, filename: &str, mime: &str, bytes: &[u8])
    -> Result<String>;
    fn download_permit(&self, trip: &str) -> Result<Vec<u8>>;
}
impl Transport for Client {
    fn origin(&self) -> String {
        self.base_url.origin().ascii_serialization()
    }
    fn request(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value> {
        Client::request(self, method, path, body)
    }
    fn upload_permit(
        &self,
        trip: &str,
        filename: &str,
        mime: &str,
        bytes: &[u8],
    ) -> Result<String> {
        let mut response = self.permit_request("PUT", trip, Some((filename, mime, bytes)))?;
        let value: Value = response
            .body_mut()
            .read_json()
            .map_err(|_| SyncError::Protocol("Invalid permit response".into()))?;
        value["sha256"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| SyncError::Protocol("Missing permit hash".into()))
    }
    fn download_permit(&self, trip: &str) -> Result<Vec<u8>> {
        let mut response = self.permit_request("GET", trip, None)?;
        response
            .body_mut()
            .with_config()
            .limit(strider_store::PERMIT_MAX_BYTES as u64)
            .read_to_vec()
            .map_err(|_| SyncError::Protocol("Invalid permit body".into()))
    }
}
impl Client {
    fn permit_request(
        &self,
        method: &str,
        trip: &str,
        file: Option<(&str, &str, &[u8])>,
    ) -> Result<ureq::http::Response<ureq::Body>> {
        let url = self
            .base_url
            .join(&format!("/api/v1/trips/{trip}/permit"))
            .map_err(|_| SyncError::Protocol("Invalid permit path".into()))?;
        let mut request = ureq::http::Request::builder()
            .method(method)
            .uri(url.as_str())
            .header("X-Strider-App-Version", &self.app_version);
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        let bytes = if let Some((name, mime, bytes)) = file {
            request = request
                .header("Content-Type", mime)
                .header("X-Filename", name);
            bytes.to_vec()
        } else {
            vec![]
        };
        let request = request
            .body(bytes)
            .map_err(|_| SyncError::Protocol("Invalid permit request".into()))?;
        let response = self.agent.run(request).map_err(|_| SyncError::Offline)?;
        match response.status().as_u16() {
            200..=299 => Ok(response),
            401 => Err(SyncError::Unauthorized),
            426 => Err(SyncError::UpgradeRequired),
            s => Err(SyncError::Server(s)),
        }
    }
}
