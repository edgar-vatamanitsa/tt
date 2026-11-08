// HUID - Human-Unique IDentifier: `[0-9]{8}-[0-9]{6}(-[a-zA-Z0-9\-]*)?`

use crate::time::UtcDateTime;

pub const HUID_REGEXP_FOR_USER_REPORT_PURPOSES: &str = "/[0-9]{8}-[0-9]{6}(-[a-zA-Z0-9\\-]*)?/";

fn is_alnum_or_dash(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-'
}

/// Length of the HUID at the very start of `s`, if `s` starts with one.
pub fn huid_prefix_len(s: &[u8]) -> Option<usize> {
    if s.len() < 15
        || !s[..8].iter().all(u8::is_ascii_digit)
        || s[8] != b'-'
        || !s[9..15].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let mut len = 15;
    if s.get(len) == Some(&b'-') {
        len += 1;
        while len < s.len() && is_alnum_or_dash(s[len]) {
            len += 1;
        }
    }
    Some(len)
}

pub fn is_valid_huid(id: &str) -> bool {
    huid_prefix_len(id.as_bytes()) == Some(id.len())
}

pub fn new_huid(suffix: Option<&str>) -> String {
    let t = UtcDateTime::now();
    let id = format!("{:04}{:02}{:02}-{:02}{:02}{:02}", t.year, t.month, t.day, t.hour, t.minute, t.second);
    match suffix {
        Some(suffix) => format!("{id}-{suffix}"),
        None => id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_huids() {
        assert!(is_valid_huid("20260829-235855"));
        assert!(is_valid_huid("20260829-235855-rexim"));
        assert!(is_valid_huid("20260829-235902-01"));
        assert!(is_valid_huid("20260829-235902-"));
        assert!(!is_valid_huid("20260829-23585"));
        assert!(!is_valid_huid("20260829_235855"));
        assert!(!is_valid_huid("20260829-235855x"));
        assert!(!is_valid_huid("20260829-235855-a_b"));
        assert!(!is_valid_huid("README.md"));
    }

    #[test]
    fn finds_huid_prefix_in_text() {
        assert_eq!(huid_prefix_len(b"20260829-235855): rest"), Some(15));
        assert_eq!(huid_prefix_len(b"20260829-235855-rexim rest"), Some(21));
        assert_eq!(huid_prefix_len(b"2026082-235855"), None);
    }

    #[test]
    fn generates_valid_huids() {
        assert!(is_valid_huid(&new_huid(None)));
        assert!(new_huid(Some("rexim")).ends_with("-rexim"));
    }
}
