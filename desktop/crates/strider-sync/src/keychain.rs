use crate::{Result, SyncError};
use std::{collections::BTreeMap, sync::Mutex};
use url::Url;
pub trait TokenStore {
    fn store(&self, base_url: &str, token: &str) -> Result<()>;
    fn load(&self, base_url: &str) -> Result<Option<String>>;
    fn delete(&self, base_url: &str) -> Result<()>;
}
fn account(base: &str) -> Result<String> {
    Url::parse(base)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
        .ok_or_else(|| SyncError::Protocol("Invalid server host".into()))
}
pub struct Keychain;
const SERVICE: &str = "dev.thaiv.strider";
fn error(_: security_framework::base::Error) -> SyncError {
    SyncError::Protocol("Could not access Strider's Keychain credential".into())
}
impl TokenStore for Keychain {
    fn store(&self, base: &str, token: &str) -> Result<()> {
        security_framework::passwords::set_generic_password(
            SERVICE,
            &account(base)?,
            token.as_bytes(),
        )
        .map_err(error)
    }
    fn load(&self, base: &str) -> Result<Option<String>> {
        match security_framework::passwords::get_generic_password(SERVICE, &account(base)?) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| SyncError::Protocol("Invalid Keychain credential".into())),
            Err(e) if e.code() == -25300 => Ok(None),
            Err(e) => Err(error(e)),
        }
    }
    fn delete(&self, base: &str) -> Result<()> {
        match security_framework::passwords::delete_generic_password(SERVICE, &account(base)?) {
            Ok(()) => Ok(()),
            Err(e) if e.code() == -25300 => Ok(()),
            Err(e) => Err(error(e)),
        }
    }
}
#[derive(Default)]
pub struct MemoryTokenStore(Mutex<BTreeMap<String, String>>);
impl TokenStore for MemoryTokenStore {
    fn store(&self, base: &str, token: &str) -> Result<()> {
        self.0
            .lock()
            .map_err(|_| SyncError::Protocol("Credential store unavailable".into()))?
            .insert(account(base)?, token.into());
        Ok(())
    }
    fn load(&self, base: &str) -> Result<Option<String>> {
        Ok(self
            .0
            .lock()
            .map_err(|_| SyncError::Protocol("Credential store unavailable".into()))?
            .get(&account(base)?)
            .cloned())
    }
    fn delete(&self, base: &str) -> Result<()> {
        self.0
            .lock()
            .map_err(|_| SyncError::Protocol("Credential store unavailable".into()))?
            .remove(&account(base)?);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn memory_store_roundtrip() {
        let s = MemoryTokenStore::default();
        assert!(s.load("https://example.com").unwrap().is_none());
        s.store("https://example.com", "secret").unwrap();
        assert_eq!(
            s.load("https://example.com").unwrap().as_deref(),
            Some("secret")
        );
        assert!(s.load("https://other.example.com").unwrap().is_none());
        s.delete("https://example.com").unwrap();
        assert!(s.load("https://example.com").unwrap().is_none());
    }
}
