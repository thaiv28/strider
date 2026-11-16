//! Pure, dependency-free Strider domain rules.
pub mod energy;
pub mod food;
pub mod format;
pub mod fuel;
pub mod gear;
pub mod propagation;
pub mod route;
pub mod trip;
pub mod units;
pub use format::*;
pub use units::{Cents, Dec, Grams};
