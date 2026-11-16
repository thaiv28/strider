use crate::format::{OZ_TO_G, js_round};
pub const VOLUME_ML: [(&str, f64); 6] = [
    ("ml", 1.),
    ("l", 1000.),
    ("tsp", 4.92892),
    ("tbsp", 14.7868),
    ("floz", 29.5735),
    ("cup", 236.588),
];
pub fn volume_ml(unit: &str) -> Option<f64> {
    VOLUME_ML
        .iter()
        .find(|(u, _)| *u == unit)
        .map(|(_, ml)| *ml)
}
pub fn kcal_per_oz(kcal: Option<f64>) -> Option<f64> {
    kcal.map(|k| k / 100. * OZ_TO_G)
}
pub fn kcal_for_grams(kcal: Option<f64>, grams: f64) -> i64 {
    js_round(kcal.unwrap_or(0.) / 100. * grams) as i64
}
pub fn grams_from_volume(amount: f64, unit: &str, density: Option<f64>) -> Option<i64> {
    Some(js_round(amount * volume_ml(unit)? * density?) as i64)
}
pub fn volume_from_grams(grams: f64, unit: &str, density: Option<f64>) -> Option<f64> {
    let d = density?;
    if d == 0. {
        None
    } else {
        Some(grams / d / volume_ml(unit)?)
    }
}
/// JavaScript Number coercion for nullable form fields (invalid text is NaN).
pub fn num(s: Option<&str>) -> Option<f64> {
    let s = s.filter(|s| !s.is_empty())?.trim();
    if s.is_empty() {
        return Some(0.);
    }
    for (prefixes, radix) in [(["0x", "0X"], 16), (["0b", "0B"], 2), (["0o", "0O"], 8)] {
        if let Some(digits) = prefixes.iter().find_map(|p| s.strip_prefix(p)) {
            if digits.is_empty() {
                return Some(f64::NAN);
            }
            let mut out = 0.;
            for c in digits.chars() {
                let Some(digit) = c.to_digit(radix) else {
                    return Some(f64::NAN);
                };
                out = out * f64::from(radix) + f64::from(digit);
            }
            return Some(out);
        }
    }
    Some(match s {
        "Infinity" | "+Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ if s
            .bytes()
            .any(|b| b.is_ascii_alphabetic() && b != b'e' && b != b'E') =>
        {
            f64::NAN
        }
        _ => s.parse().unwrap_or(f64::NAN),
    })
}
pub fn is_amount_unit(unit: &str) -> bool {
    matches!(unit, "g" | "oz") || volume_ml(unit).is_some()
}
pub fn parse_volume_unit(label: &str) -> Option<&'static str> {
    let text = label.to_lowercase().replace('.', "");
    let words: Vec<_> = text
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|s| !s.is_empty())
        .collect();
    // The fluid-ounce pattern permits whitespace only between the two words.
    for start in 0..text.len() {
        if !text.is_char_boundary(start) {
            continue;
        }
        let rest = &text[start..];
        if start > 0
            && (text.as_bytes()[start - 1].is_ascii_alphanumeric()
                || text.as_bytes()[start - 1] == b'_')
        {
            continue;
        }
        for (prefix, suffixes) in [("fl", &["oz"][..]), ("fluid", &["ounce", "ounces"][..])] {
            if let Some(tail) = rest.strip_prefix(prefix) {
                let trimmed = tail.trim_start();
                if trimmed.len() == tail.len() {
                    continue;
                }
                for suffix in suffixes {
                    if let Some(after) = trimmed.strip_prefix(suffix)
                        && (after.is_empty()
                            || !(after.as_bytes()[0].is_ascii_alphanumeric()
                                || after.as_bytes()[0] == b'_'))
                    {
                        return Some("floz");
                    }
                }
            }
        }
    }
    words.iter().find_map(|word| match *word {
        "cup" | "cups" => Some("cup"),
        "tablespoon" | "tablespoons" | "tbsp" | "tbs" => Some("tbsp"),
        "teaspoon" | "teaspoons" | "tsp" => Some("tsp"),
        "milliliter" | "milliliters" | "millilitre" | "millilitres" | "ml" => Some("ml"),
        "liter" | "liters" | "litre" | "litres" | "l" => Some("l"),
        _ => None,
    })
}
fn number_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    i
}
fn spaces(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}
fn integer_end(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i
}
fn fraction_end(bytes: &[u8], start: usize) -> Option<usize> {
    let end = integer_end(bytes, start);
    if end == start || bytes.get(end) != Some(&b'/') {
        return None;
    }
    let denom = integer_end(bytes, end + 1);
    (denom > end + 1).then_some(denom)
}
pub fn density_from_serving(text: Option<&str>, grams_hint: Option<f64>) -> Option<f64> {
    let text = text.filter(|s| !s.is_empty())?;
    let lower = text.to_lowercase();
    let bytes = lower.as_bytes();
    let grams = grams_hint.or_else(|| {
        for i in 0..bytes.len() {
            if !bytes[i].is_ascii_digit() {
                continue;
            }
            let end = number_end(bytes, i);
            let unit_start = spaces(bytes, end);
            let rest = &lower[unit_start..];
            for unit in ["grams", "gram", "gr", "g"] {
                if let Some(after) = rest.strip_prefix(unit)
                    && (after.is_empty()
                        || !(after.as_bytes()[0].is_ascii_alphanumeric()
                            || after.as_bytes()[0] == b'_'))
                {
                    return lower[i..end].parse().ok();
                }
            }
        }
        None
    })?;
    if !grams.is_finite() || grams <= 0. {
        return None;
    }
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        let integer = integer_end(bytes, start);
        let second = spaces(bytes, integer);
        let end = if second > integer {
            fraction_end(bytes, second).unwrap_or_else(|| number_end(bytes, start))
        } else {
            fraction_end(bytes, start).unwrap_or_else(|| number_end(bytes, start))
        };
        let unit_start = spaces(bytes, end);
        let mut unit_end = unit_start;
        while unit_end < bytes.len() && bytes[unit_end].is_ascii_alphabetic() {
            unit_end += 1;
        }
        if unit_end == unit_start {
            i += 1;
            continue;
        }
        let dot_end = unit_end + usize::from(bytes.get(unit_end) == Some(&b'.'));
        let next = spaces(bytes, dot_end);
        if next > dot_end {
            for tail in ["ounces", "ounce", "oz"] {
                if lower[next..].starts_with(tail) {
                    unit_end = next + tail.len();
                    break;
                }
            }
        }
        if bytes.get(unit_end) == Some(&b'.') {
            unit_end += 1;
        }
        if let Some(unit) = parse_volume_unit(&lower[unit_start..unit_end]) {
            let quantity: f64 = lower[start..end]
                .split_whitespace()
                .map(|part| {
                    if let Some((a, b)) = part.split_once('/') {
                        a.parse::<f64>().unwrap_or(0.) / b.parse::<f64>().unwrap_or(0.)
                    } else {
                        part.parse().unwrap_or(0.)
                    }
                })
                .sum();
            let raw = grams / (quantity * volume_ml(unit)?);
            let density = crate::format::fixed(raw, 3)
                .parse::<f64>()
                .unwrap_or(f64::NAN);
            return (density.is_finite() && (0.02..=2.5).contains(&density)).then_some(density);
        }
        i = unit_end;
    }
    None
}
