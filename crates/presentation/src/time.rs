pub mod calendar;
pub mod rails_zones;

pub use calendar::CalendarTime;

/// An `ActiveSupport::TimeZone`.
#[derive(Clone, Debug)]
pub struct Zone {
    name: String,
    tz: jiff::tz::TimeZone,
}

/// ActionView::Helpers::DateHelper, English defaults (our config/locales/en.yml is empty).
pub fn distance_of_time_in_words(
    zone: &Zone,
    from: jiff::Timestamp,
    to: jiff::Timestamp,
    include_seconds: bool,
) -> String {
    let (from, to) = if from > to { (to, from) } else { (from, to) };
    let seconds = (to.as_nanosecond() - from.as_nanosecond()) as f64 / 1e9;
    let minutes = (seconds / 60.0).round() as i64;
    let words = |prefix: &str, count: i64, unit: &str| {
        format!(
            "{prefix}{count} {unit}{}",
            if count == 1 { "" } else { "s" }
        )
    };
    match minutes {
        0..=1 if !include_seconds => {
            if minutes == 0 {
                "less than a minute".into()
            } else {
                "1 minute".into()
            }
        }
        0..=1 => match seconds.round() as i64 {
            0..=4 => "less than 5 seconds".into(),
            5..=9 => "less than 10 seconds".into(),
            10..=19 => "less than 20 seconds".into(),
            20..=39 => "half a minute".into(),
            40..=59 => "less than a minute".into(),
            _ => "1 minute".into(),
        },
        2..=44 => words("", minutes, "minute"),
        45..=89 => "about 1 hour".into(),
        90..=1439 => words("about ", (minutes as f64 / 60.0).round() as i64, "hour"),
        1440..=2519 => "1 day".into(),
        2520..=43199 => words("", (minutes as f64 / 1440.0).round() as i64, "day"),
        43200..=86399 => words("about ", (minutes as f64 / 43200.0).round() as i64, "month"),
        86400..=525599 => words("", (minutes as f64 / 43200.0).round() as i64, "month"),
        _ => {
            let from = from.to_zoned(zone.tz.clone());
            let to = to.to_zoned(zone.tz.clone());
            let first_year = i64::from(from.year()) + i64::from(from.month() >= 3);
            let last_year = i64::from(to.year()) - i64::from(to.month() < 3);
            let leap_count = |year: i64| year / 4 - year / 100 + year / 400;
            let leap_days = if first_year > last_year {
                0
            } else {
                leap_count(last_year) - leap_count(first_year - 1)
            };
            let adjusted = minutes - leap_days * 1440;
            let years = adjusted / 525600;
            match adjusted % 525600 {
                0..=131399 => words("about ", years, "year"),
                131400..=394199 => words("over ", years, "year"),
                _ => words("almost ", years + 1, "year"),
            }
        }
    }
}

/// The caller passes the request's clock so frozen parity runs and live pages use the same API.
pub fn time_ago_in_words(zone: &Zone, instant: jiff::Timestamp, now: jiff::Timestamp) -> String {
    distance_of_time_in_words(zone, instant, now, false)
}
impl Zone {
    /// `ActiveSupport::TimeZone["UTC"]`, the app's default `Time.zone`.
    pub fn utc() -> Self {
        Zone {
            name: "UTC".into(),
            tz: jiff::tz::TimeZone::UTC,
        }
    }

    /// `SetTimeZone#apply_user_time_zone`: the user's zone if it names one, else the default.
    pub fn for_user(_time_zone: Option<&str>) -> Self {
        _time_zone.and_then(Self::lookup).unwrap_or_else(Self::utc)
    }

    /// Case-sensitive, like ActiveSupport/TZInfo (Jiff's database lookup is case-insensitive).
    pub fn lookup(name: &str) -> Option<Self> {
        let identifier = rails_zones::MAPPING
            .iter()
            .find(|(label, _)| *label == name)
            .map(|(_, identifier)| *identifier)
            .unwrap_or(name);
        if rails_zones::IDENTIFIERS.binary_search(&identifier).is_err() {
            return None;
        }
        let tz = jiff::tz::TimeZone::get(identifier).ok()?;
        Some(Self {
            name: name.into(),
            tz,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tz(&self) -> &jiff::tz::TimeZone {
        &self.tz
    }

    pub fn format(&self, instant: jiff::Timestamp, format: &str) -> String {
        let local = instant.to_zoned(self.tz.clone());
        let year = local.year();
        let format = if year < 0 {
            format.replace("%Y", &format!("-{:04}", year.unsigned_abs()))
        } else {
            format.to_owned()
        };
        local.strftime(&format).to_string()
    }

    pub fn iso8601(&self, instant: jiff::Timestamp) -> String {
        if self.tz.to_offset_info(instant).abbreviation() == "UTC" {
            self.format(instant, "%Y-%m-%dT%H:%M:%SZ")
        } else {
            self.format(instant, "%Y-%m-%dT%H:%M:%S%:z")
        }
    }

    pub fn to_fs(&self, instant: jiff::Timestamp, style: &str) -> String {
        match style {
            "short" => self.format(instant, "%d %b %H:%M"),
            "long" => self.format(instant, "%B %d, %Y %H:%M"),
            // TimeWithZone's :db is UTC, irrespective of the viewer's zone.
            "db" => instant.strftime("%Y-%m-%d %H:%M:%S").to_string(),
            "number" => self.format(instant, "%Y%m%d%H%M%S"),
            "epoch" => instant.as_millisecond().to_string(),
            _ if self.tz.to_offset_info(instant).abbreviation() == "UTC" => {
                self.format(instant, "%Y-%m-%d %H:%M:%S UTC")
            }
            _ => self.format(instant, "%Y-%m-%d %H:%M:%S %z"),
        }
    }
}
