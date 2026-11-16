//! Per-device preferences: local to this Mac, never synced.
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

const FILE: &str = "devices.json";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevicePrefs {
    /// The folder watched for new `.gpx` and `.fit` files.
    #[serde(default)]
    pub watched_folder: Option<PathBuf>,
}
impl DevicePrefs {
    /// Read `<dir>/devices.json`. A missing or unreadable file means defaults.
    pub fn load(dir: &Path) -> Self {
        fs::read(dir.join(FILE))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    /// Write `<dir>/devices.json` atomically.
    pub fn save(&self, dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        let tmp = dir.join(format!("{FILE}.tmp"));
        fs::write(
            &tmp,
            serde_json::to_vec_pretty(self).map_err(io::Error::other)?,
        )?;
        fs::rename(&tmp, dir.join(FILE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::temp_dir;

    #[test]
    fn preferences_round_trip_and_tolerate_damage() {
        let dir = temp_dir("prefs");
        assert_eq!(DevicePrefs::load(&dir), DevicePrefs::default());
        let prefs = DevicePrefs {
            watched_folder: Some(PathBuf::from("/some/folder")),
        };
        prefs.save(&dir).expect("save");
        assert_eq!(DevicePrefs::load(&dir), prefs);
        fs::write(dir.join(FILE), "not json").expect("damage");
        assert_eq!(DevicePrefs::load(&dir), DevicePrefs::default());
        fs::remove_dir_all(dir).ok();
    }
}
