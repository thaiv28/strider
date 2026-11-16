//! Canonical storage units and the web application's weight formatting.
use std::iter::Sum;
use std::ops::{Add, Mul};

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
pub const OZ_TO_G: f64 = 28.3495;
pub fn g_to_oz(g: Grams) -> f64 {
    f64::from(g.0) / OZ_TO_G
}
pub fn oz_to_g(oz: f64) -> Grams {
    Grams((oz * OZ_TO_G).round() as i32)
}
pub fn display_oz(g: Grams) -> f64 {
    let oz = g_to_oz(g);
    let quarter = (oz * 4.).round() / 4.;
    if oz_to_g(quarter) == g { quarter } else { oz }
}
pub fn fmt_weight(g: Option<Grams>) -> String {
    let Some(g) = g else { return "—".into() };
    let oz = display_oz(g);
    if oz < 1. {
        format!("{} oz", fixed(oz, 2))
    } else if oz < 16. {
        format!("{} oz", fixed(oz, 1))
    } else {
        let lb = (oz / 16.).floor();
        format!("{lb:.0} lb {} oz", fixed(oz - lb * 16., 1))
    }
}
pub fn fmt_oz(g: Option<Grams>) -> String {
    g.map_or_else(|| "—".into(), |g| format!("{} oz", fixed(display_oz(g), 2)))
}
pub fn fmt_lbs(g: Option<Grams>) -> String {
    g.map_or_else(
        || "—".into(),
        |g| format!("{} lbs", fixed(g_to_oz(g) / 16., 1)),
    )
}
// JS toFixed rounds an exact decimal tie away from zero; Rust formatting uses
// ties-to-even. Work with the stored binary rational so near-ties also match JS.
fn fixed(value: f64, digits: u32) -> String {
    if value == 0. {
        return format!("{:.*}", digits as usize, 0.);
    }
    let bits = value.abs().to_bits();
    let mantissa = (bits & ((1u64 << 52) - 1)) | (1u64 << 52);
    let exponent = ((bits >> 52) as i32) - 1023 - 52;
    let scale = 10u128.pow(digits);
    let numerator = u128::from(mantissa) * scale;
    let rounded = if exponent >= 0 {
        numerator << exponent
    } else {
        let denominator = 1u128 << -exponent;
        numerator / denominator + u128::from((numerator % denominator) * 2 >= denominator)
    };
    let sign = if value.is_sign_negative() { "-" } else { "" };
    format!(
        "{sign}{}.{:0width$}",
        rounded / scale,
        rounded % scale,
        width = digits as usize
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn web_formatting() {
        assert_eq!(
            (fmt_weight(None), fmt_oz(None), fmt_lbs(None)),
            ("—".into(), "—".into(), "—".into())
        );
        for (g, weight, oz, lbs) in [
            (0, "0.00 oz", "0.00 oz", "0.0 lbs"),
            (5, "0.18 oz", "0.18 oz", "0.0 lbs"),
            (28, "1.0 oz", "1.00 oz", "0.1 lbs"),
            (454, "1 lb 0.0 oz", "16.00 oz", "1.0 lbs"),
            (1000, "2 lb 3.3 oz", "35.27 oz", "2.2 lbs"),
            (35, "1.3 oz", "1.25 oz", "0.1 lbs"),
            (50, "1.8 oz", "1.75 oz", "0.1 lbs"),
            (51, "1.8 oz", "1.80 oz", "0.1 lbs"),
        ] {
            assert_eq!(fmt_weight(Some(Grams(g))), weight);
            assert_eq!(fmt_oz(Some(Grams(g))), oz);
            assert_eq!(fmt_lbs(Some(Grams(g))), lbs);
        }
        assert_eq!(display_oz(Grams(50)), 1.75);
        assert_ne!(display_oz(Grams(51)), 1.75);
        assert_eq!(oz_to_g(1.75), Grams(50));
    }
}
