//! Canonical integer units and fixed-scale PostgreSQL-compatible decimals.
use std::{
    fmt,
    iter::Sum,
    ops::{Add, Div, Mul, Sub},
    str::FromStr,
};
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Grams(pub i32);
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Cents(pub i32);
impl Add for Grams {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}
impl Mul<i32> for Grams {
    type Output = Self;
    fn mul(self, rhs: i32) -> Self {
        Self(self.0 * rhs)
    }
}
impl Sum for Grams {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(|g| g.0).sum())
    }
}
/// Exact decimal with nine fractional places. Operations round half away from zero
/// when a result needs more than nine places; integral database rounding uses the
/// same rule. Parsing rejects excess precision rather than silently losing it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Dec(i128);
const SCALE: i128 = 1_000_000_000;
pub(crate) fn rounded_div(n: i128, d: i128) -> i128 {
    assert_ne!(d, 0, "decimal division by zero");
    let q = n / d;
    let r = n % d;
    if r.unsigned_abs() >= d.unsigned_abs().div_ceil(2) {
        q + if (n < 0) == (d < 0) { 1 } else { -1 }
    } else {
        q
    }
}
impl Dec {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(SCALE);
    pub const fn from_scaled(value: i128) -> Self {
        Self(value)
    }
    pub const fn scaled(self) -> i128 {
        self.0
    }
    pub fn as_f64(self) -> f64 {
        self.0 as f64 / SCALE as f64
    }
    pub fn round_half_away(self) -> i64 {
        rounded_div(self.0, SCALE) as i64
    }
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.0 == 0 {
            None
        } else {
            self.0
                .checked_mul(SCALE)
                .map(|n| Self(rounded_div(n, rhs.0)))
        }
    }
}
impl From<i32> for Dec {
    fn from(value: i32) -> Self {
        Self(i128::from(value) * SCALE)
    }
}
impl From<i64> for Dec {
    fn from(value: i64) -> Self {
        Self(i128::from(value) * SCALE)
    }
}
impl Add for Dec {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}
impl Sub for Dec {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}
impl Mul for Dec {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self(rounded_div(self.0 * rhs.0, SCALE))
    }
}
// Scaling the numerator is required for fixed-point division.
#[allow(clippy::suspicious_arithmetic_impl)]
impl Div for Dec {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        Self(rounded_div(self.0 * SCALE, rhs.0))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDecError;
impl fmt::Display for ParseDecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid decimal or precision exceeds nine places")
    }
}
impl std::error::Error for ParseDecError {}
impl FromStr for Dec {
    type Err = ParseDecError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (negative, s) = if let Some(s) = s.strip_prefix('-') {
            (true, s)
        } else {
            (false, s.strip_prefix('+').unwrap_or(s))
        };
        let mut parts = s.split('.');
        let whole = parts.next().ok_or(ParseDecError)?;
        let fraction = parts.next().unwrap_or("");
        if parts.next().is_some()
            || (whole.is_empty() && fraction.is_empty())
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(ParseDecError);
        }
        let fraction = fraction.trim_end_matches('0');
        if fraction.len() > 9 {
            return Err(ParseDecError);
        }
        let whole = if whole.is_empty() {
            0
        } else {
            whole.parse::<i128>().map_err(|_| ParseDecError)?
        };
        let frac = if fraction.is_empty() {
            0
        } else {
            fraction.parse::<i128>().map_err(|_| ParseDecError)?
                * 10_i128.pow(9 - fraction.len() as u32)
        };
        let magnitude = whole
            .checked_mul(SCALE)
            .and_then(|v| v.checked_add(frac))
            .ok_or(ParseDecError)?;
        Ok(Self(if negative { -magnitude } else { magnitude }))
    }
}
impl fmt::Display for Dec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let magnitude = self.0.unsigned_abs();
        if self.0 < 0 {
            f.write_str("-")?;
        }
        write!(f, "{}", magnitude / SCALE as u128)?;
        let rem = magnitude % SCALE as u128;
        if rem != 0 {
            write!(f, ".{}", format!("{rem:09}").trim_end_matches('0'))?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn d(s: &str) -> Dec {
        s.parse().unwrap()
    }
    #[test]
    fn decimal_parse_display() {
        for (s, out) in [
            ("1.25000", "1.25"),
            ("-0.000000001", "-0.000000001"),
            (".5", "0.5"),
            ("-0.000", "0"),
            ("12", "12"),
        ] {
            assert_eq!(d(s).to_string(), out)
        }
        assert!("1.0000000001".parse::<Dec>().is_err());
        assert!("1e2".parse::<Dec>().is_err());
    }
    #[test]
    fn decimal_arithmetic_and_ties() {
        assert_eq!((d("1.25") + d("2.5")).to_string(), "3.75");
        assert_eq!((d("1.25") - d("2.5")).to_string(), "-1.25");
        assert_eq!((d("1.25") * d("2.5")).to_string(), "3.125");
        assert_eq!((d("1") / d("6")).to_string(), "0.166666667");
        assert_eq!((d("-1") / d("6")).to_string(), "-0.166666667");
        assert_eq!((d("0.000000001") / d("2")).to_string(), "0.000000001");
        for (s, n) in [
            ("0.5", 1),
            ("-0.5", -1),
            ("1.5", 2),
            ("-1.5", -2),
            ("1.499999999", 1),
        ] {
            assert_eq!(d(s).round_half_away(), n)
        }
    }
}
