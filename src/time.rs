// Minimal UTC calendar math so we don't need a date/time dependency.
// Also included by build.rs via #[path], so keep it std-only.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcDateTime {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl UtcDateTime {
    pub fn now() -> Self {
        let secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs() as i64,
            Err(e) => -(e.duration().as_secs() as i64),
        };
        Self::from_unix(secs)
    }

    pub fn from_unix(secs: i64) -> Self {
        let days = secs.div_euclid(86400);
        let rem = secs.rem_euclid(86400) as u32;
        let (year, month, day) = civil_from_days(days);
        Self { year, month, day, hour: rem / 3600, minute: rem % 3600 / 60, second: rem % 60 }
    }
}

// Howard Hinnant's days-to-civil algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe as i64 + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_timestamps() {
        let t = |y, mo, d, h, mi, s| UtcDateTime { year: y, month: mo, day: d, hour: h, minute: mi, second: s };
        assert_eq!(UtcDateTime::from_unix(0), t(1970, 1, 1, 0, 0, 0));
        assert_eq!(UtcDateTime::from_unix(951782400), t(2000, 2, 29, 0, 0, 0));
        assert_eq!(UtcDateTime::from_unix(1788219296), t(2026, 8, 31, 23, 34, 56));
        assert_eq!(UtcDateTime::from_unix(-1), t(1969, 12, 31, 23, 59, 59));
    }
}
