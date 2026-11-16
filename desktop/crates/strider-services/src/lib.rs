//! Online services with offline caches: weather, snow, calendar feeds, Overpass, sharing.
//!
//! Every network call goes through [`HttpClient`] (public data sources) or
//! `strider_sync::Transport` (the signed-in `/api/v1` server), so tests use fakes and never
//! touch the network. Cached reads return [`Cached`] so the UI can say "Updated 3 h ago".
pub mod calendar;
pub mod forecast_window;
pub mod http;
pub mod overpass;
pub mod share;
pub mod snow;
pub mod weather;

use serde::{Serialize, de::DeserializeOwned};
use std::fmt;
use strider_store::{CacheKind, Store};

pub use http::{HttpClient, HttpResponse, UreqClient};

#[derive(Debug, PartialEq, Eq)]
pub enum ServiceError {
    Offline,
    Unauthorized,
    /// The route exists but the server has not implemented it (HTTP 501).
    NotImplemented,
    Server(u16),
    Protocol(String),
    Store(String),
}
pub type Result<T> = std::result::Result<T, ServiceError>;
impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offline => write!(f, "Offline"),
            Self::Unauthorized => write!(f, "Sign in to continue"),
            Self::NotImplemented => write!(f, "Not available yet"),
            Self::Server(s) => write!(f, "Server returned {s}"),
            Self::Protocol(s) | Self::Store(s) => write!(f, "{s}"),
        }
    }
}
impl std::error::Error for ServiceError {}
impl From<strider_store::Error> for ServiceError {
    fn from(e: strider_store::Error) -> Self {
        Self::Store(e.to_string())
    }
}
impl From<strider_sync::SyncError> for ServiceError {
    fn from(e: strider_sync::SyncError) -> Self {
        use strider_sync::SyncError as S;
        match e {
            S::Offline => Self::Offline,
            S::Unauthorized => Self::Unauthorized,
            S::Server(501) => Self::NotImplemented,
            S::Server(s) => Self::Server(s),
            other => Self::Protocol(other.to_string()),
        }
    }
}

/// A value with the moment it was fetched. `stale` means a refresh was needed but impossible
/// (offline or the source failed), so this is the last good copy.
#[derive(Clone, Debug, PartialEq)]
pub struct Cached<T> {
    pub value: T,
    /// Unix seconds.
    pub fetched_at: i64,
    pub stale: bool,
}
impl<T> Cached<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Cached<U> {
        Cached {
            value: f(self.value),
            fetched_at: self.fetched_at,
            stale: self.stale,
        }
    }
}

/// Unix-second clock, injectable for freshness tests.
pub trait Clock: Send + Sync {
    fn now(&self) -> i64;
}
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

/// Freshness lifetimes, mirroring the web's revalidate windows (`lib/snow.ts`, `lib/overpass.ts`,
/// `lib/calendar.ts`).
pub mod ttl {
    pub const FORECAST: i64 = 3_600;
    pub const HISTORICAL: i64 = 7 * 86_400;
    pub const CLIMATOLOGY: i64 = 30 * 86_400;
    pub const SNOW_CURRENT: i64 = 1_800;
    pub const CALENDAR: i64 = 300;
    pub const OVERPASS: i64 = 7 * 86_400;
}

/// Read-through cache. A fresh hit is returned without fetching. Otherwise `fetch` runs: `Ok(Some)`
/// is stored; `Ok(None)` means the source had no data (nothing is cached and `None` is returned);
/// an error returns the cached copy marked stale, or the error when there is none.
pub(crate) fn read_through<T: Serialize + DeserializeOwned>(
    store: &Store,
    kind: CacheKind,
    key: &str,
    ttl: i64,
    now: i64,
    fetch: impl FnOnce() -> Result<Option<T>>,
) -> Result<Option<Cached<T>>> {
    let hit = store.cache_get(kind, key)?.and_then(|e| {
        serde_json::from_str::<T>(&e.body)
            .ok()
            .map(|v| (v, e.fetched_at))
    });
    let hit = match hit {
        Some((value, fetched_at)) if (0..ttl).contains(&now.saturating_sub(fetched_at)) => {
            return Ok(Some(Cached {
                value,
                fetched_at,
                stale: false,
            }));
        }
        other => other,
    };
    match fetch() {
        Ok(Some(value)) => {
            store.cache_put(
                kind,
                key,
                now,
                &serde_json::to_string(&value).map_err(proto)?,
            )?;
            Ok(Some(Cached {
                value,
                fetched_at: now,
                stale: false,
            }))
        }
        Ok(None) => Ok(None),
        Err(e) => match hit {
            Some((value, fetched_at)) => Ok(Some(Cached {
                value,
                fetched_at,
                stale: true,
            })),
            None => Err(e),
        },
    }
}
fn proto(e: serde_json::Error) -> ServiceError {
    ServiceError::Protocol(format!("Invalid cached data: {e}"))
}

/// Coordinates rounded for cache keys (about 1 km).
pub(crate) fn coord_key(lat: f64, lon: f64) -> String {
    format!("{lat:.2},{lon:.2}")
}
