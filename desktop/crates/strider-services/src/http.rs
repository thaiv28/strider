//! Blocking HTTP boundary for public data sources (Open-Meteo, Overpass, calendar feeds).
//! Implementations never log URLs: calendar feed addresses are private.
use crate::{Result, ServiceError};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}
impl HttpResponse {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
    /// Map a failed status to the error a caller should fall back on.
    pub fn error(&self) -> ServiceError {
        match self.status {
            401 | 403 => ServiceError::Unauthorized,
            s => ServiceError::Server(s),
        }
    }
}
pub trait HttpClient: Send + Sync {
    /// Transport failures (DNS, connect, timeout) are `Err(Offline)`; any HTTP status is `Ok`.
    fn get(&self, url: &str) -> Result<HttpResponse>;
    /// POST an `application/x-www-form-urlencoded` body.
    fn post_form(&self, url: &str, body: &str) -> Result<HttpResponse>;
}

const MAX_BODY: u64 = 16 * 1024 * 1024;

pub struct UreqClient {
    agent: ureq::Agent,
}
impl UreqClient {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
    fn read(mut response: ureq::http::Response<ureq::Body>) -> Result<HttpResponse> {
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_BODY)
            .read_to_string()
            .map_err(|_| ServiceError::Offline)?;
        Ok(HttpResponse { status, body })
    }
}
impl Default for UreqClient {
    fn default() -> Self {
        Self::new()
    }
}
const AGENT: &str = concat!("StriderDesktop/", env!("CARGO_PKG_VERSION"));
impl HttpClient for UreqClient {
    fn get(&self, url: &str) -> Result<HttpResponse> {
        let response = self
            .agent
            .get(url)
            .header("User-Agent", AGENT)
            .call()
            .map_err(|_| ServiceError::Offline)?;
        Self::read(response)
    }
    fn post_form(&self, url: &str, body: &str) -> Result<HttpResponse> {
        let response = self
            .agent
            .post(url)
            .header("User-Agent", AGENT)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send(body)
            .map_err(|_| ServiceError::Offline)?;
        Self::read(response)
    }
}

/// Deterministic fake for tests: canned bodies by URL substring, with a call log.
#[cfg(test)]
pub(crate) mod fake {
    use super::*;
    use std::sync::Mutex;
    #[derive(Default)]
    pub struct FakeHttp {
        pub routes: Mutex<Vec<(String, std::result::Result<HttpResponse, ()>)>>,
        pub log: Mutex<Vec<String>>,
    }
    impl FakeHttp {
        pub fn route(&self, needle: &str, status: u16, body: &str) -> &Self {
            self.routes.lock().unwrap().push((
                needle.into(),
                Ok(HttpResponse {
                    status,
                    body: body.into(),
                }),
            ));
            self
        }
        pub fn clear(&self) {
            self.routes.lock().unwrap().clear();
        }
        pub fn calls(&self) -> usize {
            self.log.lock().unwrap().len()
        }
        fn answer(&self, url: &str, body: Option<&str>) -> Result<HttpResponse> {
            self.log
                .lock()
                .unwrap()
                .push(format!("{url} {}", body.unwrap_or("")));
            let routes = self.routes.lock().unwrap();
            match routes.iter().find(|(n, _)| url.contains(n.as_str())) {
                Some((_, Ok(r))) => Ok(r.clone()),
                _ => Err(ServiceError::Offline),
            }
        }
    }
    impl HttpClient for FakeHttp {
        fn get(&self, url: &str) -> Result<HttpResponse> {
            self.answer(url, None)
        }
        fn post_form(&self, url: &str, body: &str) -> Result<HttpResponse> {
            self.answer(url, Some(body))
        }
    }
    pub struct FixedClock(pub std::sync::atomic::AtomicI64);
    impl crate::Clock for FixedClock {
        fn now(&self) -> i64 {
            self.0.load(std::sync::atomic::Ordering::SeqCst)
        }
    }
    impl FixedClock {
        pub fn at(secs: i64) -> Self {
            Self(std::sync::atomic::AtomicI64::new(secs))
        }
        pub fn advance(&self, secs: i64) {
            self.0.fetch_add(secs, std::sync::atomic::Ordering::SeqCst);
        }
    }
}
