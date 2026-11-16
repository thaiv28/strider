//! GPS volumes, the watched folder and FIT import.
//!
//! Only devices that mount as volumes are supported (Garmin units in USB mass-storage mode).
//! MTP devices do not mount, so they never appear here.
pub mod fit;
pub mod prefs;
pub mod volumes;
pub mod watch;

#[cfg(any(test, feature = "test-support"))]
pub mod testing;
