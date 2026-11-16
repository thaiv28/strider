use crate::units::{Cents, Grams};
/// ECMAScript Math.round: nearest integer, with ties toward positive infinity.
/// Adding 0.5 first is incorrect for the float immediately below a half.
pub fn js_round(value: f64) -> f64 {
    if !value.is_finite() || value == 0. {
        return value;
    }
    let floor = value.floor();
    let rounded = if value - floor >= 0.5 {
        floor + 1.
    } else {
        floor
    };
    if rounded == 0. && value.is_sign_negative() {
        -0.
    } else {
        rounded
    }
}
pub const OZ_TO_G: f64 = 28.3495;
pub fn g_to_oz(g: Grams) -> f64 {
    f64::from(g.0) / OZ_TO_G
}
pub fn oz_to_g(oz: f64) -> Grams {
    Grams(js_round(oz * OZ_TO_G) as i32)
}
pub fn display_oz(g: Grams) -> f64 {
    let oz = g_to_oz(g);
    let quarter = js_round(oz * 4.) / 4.;
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
/// JavaScript Number.toFixed for bounded domain values.
pub fn fixed(value: f64, digits: u32) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    if value.abs() >= 1e21 {
        return value.to_string();
    }
    if value == 0. {
        return format!("{:.*}", digits as usize, 0.);
    }
    let bits = value.abs().to_bits();
    let mantissa = (bits & ((1u64 << 52) - 1)) | (1u64 << 52);
    let exponent = ((bits >> 52) as i32) - 1023 - 52;
    let scale = 10u128.pow(digits);
    let numerator = u128::from(mantissa) * scale;
    let rounded = if exponent <= -128 {
        0
    } else if exponent >= 0 {
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

pub fn fmt_usd(cents: Option<Cents>) -> String {
    let Some(Cents(c)) = cents else {
        return "—".into();
    };
    let abs = i64::from(c).abs();
    let digits = (abs / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let fraction = if abs % 100 == 0 {
        String::new()
    } else {
        format!(".{:02}", abs % 100)
    };
    format!("${}{}{}", if c < 0 { "-" } else { "" }, grouped, fraction)
}

// Gregorian civil-date algorithms, with days counted from 1970-01-01.
fn days_from_civil(mut y: i64, m: i64, d: i64) -> i64 {
    y -= i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * mp + 2) / 5 + d - 1 - 719468
}
fn civil_from_days(z: i64) -> (i64, usize, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += i64::from(m <= 2);
    (y, m as usize, d)
}
pub fn fmt_date_range(start: Option<&str>, nights: Option<i32>) -> Option<String> {
    let start = start.filter(|s| !s.is_empty())?;
    let parts: Vec<i64> = start
        .split('-')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if parts.len() != 3
        || !(0..=9999).contains(&parts[0])
        || !(1..=12).contains(&parts[1])
        || !(1..=31).contains(&parts[2])
    {
        return None;
    }
    let mut year = parts[0];
    if (0..100).contains(&year) {
        year += 1900;
    }
    let day = days_from_civil(year, parts[1], parts[2]);
    let (sy, sm, sd) = civil_from_days(day);
    let (ey, em, ed) = civil_from_days(day + i64::from(nights.unwrap_or(0)));
    let months = [
        "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let short = |m: usize, d: i64| format!("{} {d}", months[m]);
    let full = |y, m, d| format!("{}, {y}", short(m, d));
    Some(if nights.unwrap_or(0) == 0 {
        full(sy, sm, sd)
    } else if sy != ey {
        format!("{}–{}", full(sy, sm, sd), full(ey, em, ed))
    } else {
        format!(
            "{}–{}, {ey}",
            short(sm, sd),
            if sm == em {
                ed.to_string()
            } else {
                short(em, ed)
            }
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn javascript_rounding_boundaries() {
        for (value, expected) in [
            (0.49999999999999994, 0.),
            (0.5, 1.),
            (-0.5, 0.),
            (-0.5000000000000001, -1.),
            (1.5, 2.),
            (-1.5, -1.),
            (1e-300, 0.),
            (-1e-300, 0.),
        ] {
            assert_eq!(js_round(value), expected, "{value}");
        }
        assert!(js_round(-0.5).is_sign_negative());
        assert!(js_round(-1e-300).is_sign_negative());
    }
    #[test]
    fn decimal_binary_ties_and_tiny_values() {
        assert_eq!(fixed(1.25, 1), "1.3");
        assert_eq!(fixed(1.005, 2), "1.00");
        assert_eq!(fixed(1e-300, 2), "0.00");
        assert_eq!(fixed(-1e-300, 3), "-0.000");
        assert_eq!(fmt_date_range(Some("999999999999999999-01-01"), None), None);
    }
}
