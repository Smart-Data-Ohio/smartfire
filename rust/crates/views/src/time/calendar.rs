//! Event presentation preserves Rails extended years and its finite TZInfo periods.
use super::Zone;
use bnum::types::I512;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalendarTime {
    proxy: jiff::Timestamp,
    year_shift: I512,
}
impl CalendarTime {
    pub fn new(proxy: jiff::Timestamp, year_shift: I512) -> Self {
        Self { proxy, year_shift }
    }
    fn period(&self, zone: &Zone) -> (i64, String) {
        let seconds = (self.year_shift / I512::from(400))
            .checked_mul(I512::from(146_097_i64 * 86_400))
            .and_then(|shift| shift.checked_add(I512::from(self.proxy.as_second())));
        let seconds = seconds.and_then(|n| i64::try_from(n).ok()).unwrap_or(
            if self.year_shift.is_negative() {
                i64::MIN
            } else {
                i64::MAX
            },
        );
        rails_compat::datetime::boundary_period(zone.tz(), seconds).unwrap_or_else(|| {
            let period = zone.tz().to_offset_info(self.proxy);
            (
                i64::from(period.offset().seconds()),
                period.abbreviation().to_owned(),
            )
        })
    }
    pub fn format(self, zone: &Zone, pattern: &str) -> String {
        if self.year_shift == I512::ZERO
            && rails_compat::datetime::boundary_period(zone.tz(), self.proxy.as_second()).is_none()
        {
            return zone.format(self.proxy, pattern);
        }
        let (offset, abbreviation) = self.period(zone);
        let local = self
            .proxy
            .to_zoned(jiff::tz::TimeZone::UTC)
            .datetime()
            .checked_add(jiff::SignedDuration::from_secs(offset))
            .expect("civil calendar leaves a timezone offset margin");
        let year = year_text(I512::from(local.year()) + self.year_shift);
        let numeric = format!(
            "{}{:02}{:02}",
            if offset < 0 { '-' } else { '+' },
            offset.abs() / 3600,
            offset.abs() % 3600 / 60
        );
        let colon = format!(
            "{}{:02}:{:02}",
            if offset < 0 { '-' } else { '+' },
            offset.abs() / 3600,
            offset.abs() % 3600 / 60
        );
        local
            .strftime(
                &pattern
                    .replace("%Y", &year)
                    .replace("%Z", &abbreviation)
                    .replace("%:z", &colon)
                    .replace("%z", &numeric),
            )
            .to_string()
    }
    pub fn iso8601(self, zone: &Zone) -> String {
        if self.year_shift == I512::ZERO
            && rails_compat::datetime::boundary_period(zone.tz(), self.proxy.as_second()).is_none()
        {
            return zone.iso8601(self.proxy);
        }
        let (_, abbreviation) = self.period(zone);
        self.format(
            zone,
            if abbreviation == "UTC" {
                "%Y-%m-%dT%H:%M:%SZ"
            } else {
                "%Y-%m-%dT%H:%M:%S%:z"
            },
        )
    }
}
fn year_text(year: I512) -> String {
    if year.is_negative() {
        format!("-{:04}", year.unsigned_abs())
    } else {
        format!("{year:04}")
    }
}
impl From<jiff::Timestamp> for CalendarTime {
    fn from(proxy: jiff::Timestamp) -> Self {
        Self::new(proxy, I512::ZERO)
    }
}
impl Serialize for CalendarTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.year_shift == I512::ZERO {
            return self.proxy.serialize(serializer);
        }
        let proxy = self.proxy.to_string();
        let width = proxy
            .trim_start_matches('-')
            .find('-')
            .expect("ISO date has a month")
            + usize::from(proxy.starts_with('-'));
        let year =
            I512::from(self.proxy.to_zoned(jiff::tz::TimeZone::UTC).year()) + self.year_shift;
        serializer.serialize_str(&format!("{}{}", year_text(year), &proxy[width..]))
    }
}
impl<'de> Deserialize<'de> for CalendarTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        if let Ok(proxy) = text.parse::<jiff::Timestamp>() {
            return Ok(proxy.into());
        }
        let sign = usize::from(text.starts_with(['-', '+']));
        let width = sign + text[sign..].bytes().take_while(u8::is_ascii_digit).count();
        let year = text[..width]
            .parse::<I512>()
            .map_err(serde::de::Error::custom)?;
        let proxy_year = 2000
            + i16::try_from(year.rem_euclid(I512::from(400))).map_err(serde::de::Error::custom)?;
        let proxy = format!("{proxy_year}{}", &text[width..])
            .parse::<jiff::Timestamp>()
            .map_err(serde::de::Error::custom)?;
        Ok(Self::new(proxy, year - I512::from(proxy_year)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wide_calendar_time_keeps_exact_year_and_real_rails_zone_period() {
        for (iso, label) in [
            ("60310-02-02T20:30:00Z", "February 2, 60310 at 3:30 PM"),
            ("12026-07-04T20:30:00Z", "July 4, 12026 at 3:30 PM"),
            ("2127-07-04T20:30:00Z", "July 4, 2127 at 3:30 PM"),
        ] {
            let time: CalendarTime = serde_json::from_value(serde_json::json!(iso)).unwrap();
            assert_eq!(time.iso8601(&Zone::utc()), iso);
            assert_eq!(
                time.format(
                    &Zone::lookup("America/New_York").unwrap(),
                    "%B %-d, %Y at %-I:%M %p"
                ),
                label
            );
            assert_eq!(
                time.format(&Zone::lookup("America/New_York").unwrap(), "%Z"),
                "EST"
            );
            assert_eq!(
                time.iso8601(&Zone::lookup("Hawaii").unwrap()),
                iso.replace("20:30:00Z", "10:30:00-10:00")
            );
            assert_eq!(serde_json::to_value(time).unwrap(), serde_json::json!(iso));
        }
        let time: CalendarTime =
            serde_json::from_value(serde_json::json!("60310-02-02T20:30:00.123456Z")).unwrap();
        assert_eq!(
            serde_json::to_value(time).unwrap(),
            serde_json::json!("60310-02-02T20:30:00.123456Z")
        );
        for (instant, zone) in [
            (jiff::Timestamp::MIN, "America/New_York"),
            (jiff::Timestamp::MAX, "Asia/Tokyo"),
        ] {
            let zone = Zone::lookup(zone).unwrap();
            assert_eq!(
                CalendarTime::from(instant).format(&zone, "%Y-%m-%d %H:%M:%S %Z"),
                zone.format(instant, "%Y-%m-%d %H:%M:%S %Z")
            );
        }
    }
}
