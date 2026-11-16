//! Small display helpers for ages and sizes.

/// "just now", "5 min ago", "3 h ago", "2 d ago".
pub fn age(now: i64, then: i64) -> String {
    let s = now.saturating_sub(then).max(0);
    match s {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", s / 60),
        3600..=86_399 => format!("{} h ago", s / 3600),
        _ => format!("{} d ago", s / 86_400),
    }
}
/// Whole megabytes, "<1 MB" below one.
pub fn megabytes(bytes: u64) -> String {
    let mb = (bytes as f64 / 1_048_576.).round() as u64;
    if bytes > 0 && mb == 0 {
        "<1 MB".into()
    } else {
        format!("{mb} MB")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ages_and_sizes() {
        assert_eq!(age(100, 100), "just now");
        assert_eq!(age(1000, 700), "5 min ago");
        assert_eq!(age(20_000, 9_200), "3 h ago");
        assert_eq!(age(300_000, 0), "3 d ago");
        assert_eq!(age(5, 10), "just now");
        assert_eq!(megabytes(0), "0 MB");
        assert_eq!(megabytes(1000), "<1 MB");
        assert_eq!(megabytes(42 * 1_048_576), "42 MB");
    }
}
