//! Rails' SQLite datetime encoding and the injectable clock.
//!
//! Rails writes `datetime(6)` columns as UTC text, `"%Y-%m-%d %H:%M:%S"` followed by
//! `".%06d"` microseconds only when the microseconds are non-zero
//! (`ActiveRecord::ConnectionAdapters::Quoting#quoted_date`). Values are truncated, not
//! rounded, to microseconds on assignment (`ActiveModel::Type::Helpers::TimeValue`).
//! `insert_all` instead lets SQLite stamp rows with `STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')`,
//! which has millisecond precision; see [`SQLITE_NOW`].

use std::fmt;
use std::sync::{Arc, Mutex};

use bnum::types::I512;
use jiff::{SignedDuration, Timestamp as JiffTimestamp};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

/// The timestamp expression Rails' `insert_all` uses on SQLite for `created_at`/`updated_at`.
pub const SQLITE_NOW: &str = "STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW')";

/// A UTC instant with microsecond precision, stored the way Active Record stores it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// A 128-byte Date._parse input needs at most 426 bits for its year and
// 471 bits for microseconds. Fixed-width arithmetic preserves Ruby's Integer
// values without allocating or growing work with their magnitude.
pub struct Timestamp(I512);

impl Timestamp {
    pub fn from_jiff(ts: JiffTimestamp) -> Self {
        Self(I512::from(ts.as_nanosecond().div_euclid(1000)))
    }

    pub fn from_microsecond(us: i64) -> Self {
        Self(I512::from(us))
    }

    pub fn from_second(s: i64) -> Self {
        Self(I512::from(s) * I512::from(1_000_000))
    }

    pub fn jiff(self) -> JiffTimestamp {
        self.try_jiff()
            .expect("timestamp within Jiff calendar range")
    }

    pub fn as_wide_microsecond(self) -> I512 {
        self.0
    }

    pub fn as_microsecond(self) -> i64 {
        self.0
            .try_into()
            .expect("timestamp microseconds in i64 range")
    }

    pub fn as_second(self) -> i64 {
        self.0
            .div_euclid(I512::from(1_000_000))
            .try_into()
            .expect("timestamp seconds in i64 range")
    }

    pub fn subsec_microsecond(self) -> i32 {
        self.0
            .rem_euclid(I512::from(1_000_000))
            .try_into()
            .expect("microsecond remainder in range")
    }

    /// `1.hour.ago`-style arithmetic.
    pub fn ago(self, duration: SignedDuration) -> Self {
        Self(
            (self.0 * I512::from(1000) - I512::from(duration.as_nanos()))
                .div_euclid(I512::from(1000)),
        )
    }

    pub fn since(self, duration: SignedDuration) -> Self {
        Self(
            (self.0 * I512::from(1000) + I512::from(duration.as_nanos()))
                .div_euclid(I512::from(1000)),
        )
    }

    /// Jiff's civil calendar ends at 9999; Rails credentials accept wider years.
    /// Readers that render such input must use calendar_proxy rather than jiff.
    pub fn try_jiff(self) -> Option<JiffTimestamp> {
        JiffTimestamp::from_microsecond(i64::try_from(self.0).ok()?).ok()
    }

    /// A Gregorian 400-year cycle preserves month/day and weekday. The shift is
    /// only an encoding bridge; no timezone rules are inferred from this proxy.
    pub fn from_shifted_jiff(ts: JiffTimestamp, year_shift: i64) -> Option<Self> {
        Self::from_wide_shifted_jiff(ts, I512::from(year_shift))
    }

    /// The request grammar permits years outside i64 as well as Jiff's range.
    pub fn from_wide_shifted_jiff(ts: JiffTimestamp, year_shift: I512) -> Option<Self> {
        if year_shift % I512::from(400) != I512::ZERO {
            return None;
        }
        let cycles = year_shift / I512::from(400);
        let shift = cycles.checked_mul(I512::from(146_097_i128 * 86_400_000_000))?;
        Some(Self(shift.checked_add(I512::from(
            ts.as_nanosecond().div_euclid(1000),
        ))?))
    }

    /// Clamp solely for selecting a timezone's finite transition-table boundary.
    /// The timestamp itself, comparisons and SQLite encoding retain the exact value.
    pub fn transition_second(self) -> i64 {
        let seconds = self.0.div_euclid(I512::from(1_000_000));
        seconds.try_into().unwrap_or(if seconds.is_negative() {
            i64::MIN
        } else {
            i64::MAX
        })
    }

    pub fn calendar_proxy(self) -> (JiffTimestamp, I512) {
        if let Some(ts) = self.try_jiff() {
            return (ts, I512::ZERO);
        }
        let base_year = if self.0.is_negative() { -9600 } else { 9200 };
        let base = jiff::civil::Date::new(base_year, 1, 1)
            .unwrap()
            .at(0, 0, 0, 0)
            .to_zoned(jiff::tz::TimeZone::UTC)
            .unwrap()
            .timestamp()
            .as_microsecond();
        let cycle_us = I512::from(146_097_i128 * 86_400_000_000);
        let cycles = (self.0 - I512::from(base)).div_euclid(cycle_us);
        let proxy = self.0 - cycles * cycle_us;
        (
            JiffTimestamp::from_microsecond(proxy.try_into().expect("calendar proxy in range"))
                .expect("calendar proxy in Jiff range"),
            cycles * I512::from(400),
        )
    }

    /// The exact text Active Record writes to SQLite, including signed years.
    pub fn to_db(self) -> String {
        let (proxy, shift) = self.calendar_proxy();
        let year = I512::from(proxy.to_zoned(jiff::tz::TimeZone::UTC).year()) + shift;
        let year = if year.is_negative() {
            format!("-{:04}", year.unsigned_abs())
        } else {
            format!("{year:04}")
        };
        let base = format!("{year}{}", proxy.strftime("-%m-%d %H:%M:%S"));
        match self.subsec_microsecond() {
            0 => base,
            us => format!("{base}.{us:06}"),
        }
    }

    /// Parses Rails' signed/extended-year UTC datetime(6) encoding.
    pub fn parse_db(text: &str) -> Option<Self> {
        let text = text
            .trim()
            .strip_suffix(" UTC")
            .or_else(|| text.trim().strip_suffix('Z'))
            .unwrap_or(text.trim());
        let year_end = text.get(usize::from(text.starts_with('-'))..)?.find('-')?
            + usize::from(text.starts_with('-'));
        let year = text.get(..year_end)?.parse::<I512>().ok()?;
        let remainder = text.get(year_end + 1..)?;
        if remainder.len() < 14 {
            return None;
        }
        let whole = remainder.get(..14)?;
        let b = whole.as_bytes();
        if b[2] != b'-' || !matches!(b[5], b' ' | b'T') || b[8] != b':' || b[11] != b':' {
            return None;
        }
        let num = |range: std::ops::Range<usize>| whole.get(range)?.parse::<i8>().ok();
        let proxy_year = if let Ok(year) = i16::try_from(year)
            // Jiff reserves room for timezone offsets at both timestamp ends.
            // Its civil +/-9999 years therefore cannot all become UTC instants.
            && (-9998..=9998).contains(&year)
        {
            year
        } else {
            2000 + i16::try_from(year.rem_euclid(I512::from(400))).ok()?
        };
        let date = jiff::civil::Date::new(proxy_year, num(0..2)?, num(3..5)?).ok()?;
        let fraction = remainder.get(14..)?;
        let micros = match fraction.strip_prefix('.') {
            Some(d) if !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()) => {
                let mut padded: String = d.chars().take(6).collect();
                while padded.len() < 6 {
                    padded.push('0');
                }
                padded.parse::<i32>().ok()?
            }
            None if fraction.is_empty() => 0,
            _ => return None,
        };
        let time =
            jiff::civil::Time::new(num(6..8)?, num(9..11)?, num(12..14)?, micros * 1000).ok()?;
        let ts = jiff::civil::DateTime::from_parts(date, time)
            .to_zoned(jiff::tz::TimeZone::UTC)
            .ok()?
            .timestamp();
        Self::from_wide_shifted_jiff(ts, year.checked_sub(I512::from(proxy_year))?)
    }
}

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_db())
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_db())
    }
}

impl ToSql for Timestamp {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.to_db()))
    }
}

impl FromSql for Timestamp {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let text = value.as_str()?;
        Timestamp::parse_db(text)
            .ok_or_else(|| FromSqlError::Other(format!("invalid datetime {text:?}").into()))
    }
}

/// Where models get `Time.current` from.
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_jiff(JiffTimestamp::now())
    }
}

/// A clock for tests: real time shifted by `travel`, or frozen with `travel_to`
/// (like `ActiveSupport::Testing::TimeHelpers`).
#[derive(Debug, Clone, Default)]
pub struct TestClock {
    state: Arc<Mutex<TestClockState>>,
}

#[derive(Debug, Default)]
struct TestClockState {
    offset: SignedDuration,
    frozen: Option<Timestamp>,
}

impl TestClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn frozen_at(at: Timestamp) -> Self {
        let clock = Self::new();
        clock.travel_to(at);
        clock
    }

    /// `travel_to`: freezes time at `at`.
    pub fn travel_to(&self, at: Timestamp) {
        self.state.lock().unwrap().frozen = Some(at);
    }

    /// `travel`: moves the clock forward by `by` (keeping it frozen if it was).
    pub fn travel(&self, by: SignedDuration) {
        let mut state = self.state.lock().unwrap();
        match state.frozen {
            Some(at) => state.frozen = Some(at.since(by)),
            None => state.offset += by,
        }
    }

    pub fn travel_back(&self) {
        *self.state.lock().unwrap() = TestClockState::default();
    }
}

impl Clock for TestClock {
    fn now(&self) -> Timestamp {
        let state = self.state.lock().unwrap();
        state
            .frozen
            .unwrap_or_else(|| SystemClock.now().since(state.offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impossible_dates_dont_parse() {
        assert!(Timestamp::parse_db("2026-13-01 00:00:00").is_none());
        assert!(Timestamp::parse_db("2026-02-30 00:00:00").is_none());
    }

    #[test]
    fn encodes_like_active_record() {
        let ts = Timestamp::parse_db("2026-09-26 12:34:56.123456").unwrap();
        assert_eq!(ts.to_db(), "2026-09-26 12:34:56.123456");
        assert_eq!(
            Timestamp::parse_db("2026-09-26 12:34:56").unwrap().to_db(),
            "2026-09-26 12:34:56"
        );
        assert_eq!(
            Timestamp::parse_db("2026-09-26 12:34:56.120000")
                .unwrap()
                .to_db(),
            "2026-09-26 12:34:56.120000"
        );
        assert_eq!(
            Timestamp::parse_db("2026-01-01 00:00:00.000001")
                .unwrap()
                .to_db(),
            "2026-01-01 00:00:00.000001"
        );
    }

    #[test]
    fn reads_sqlite_strftime_milliseconds() {
        assert_eq!(
            Timestamp::parse_db("2026-09-26 12:25:26.826")
                .unwrap()
                .to_db(),
            "2026-09-26 12:25:26.826000"
        );
    }

    #[test]
    fn truncates_to_microseconds() {
        let jiff = JiffTimestamp::new(1_700_000_000, 123_456_999).unwrap();
        assert_eq!(Timestamp::from_jiff(jiff).subsec_microsecond(), 123_456);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Timestamp::parse_db("yesterday").is_none());
        assert!(Timestamp::parse_db("2026-09-26 12:34:56.x").is_none());
    }
}
