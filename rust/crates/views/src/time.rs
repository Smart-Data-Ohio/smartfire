//! Time zones and time formatting as Rails renders them (placeholder; see the time module work).

/// An `ActiveSupport::TimeZone`.
#[derive(Clone, Debug)]
pub struct Zone {
    name: String,
    tz: jiff::tz::TimeZone,
}

impl Zone {
    /// `ActiveSupport::TimeZone["UTC"]`, the app's default `Time.zone`.
    pub fn utc() -> Self {
        Zone { name: "UTC".into(), tz: jiff::tz::TimeZone::UTC }
    }

    /// `SetTimeZone#apply_user_time_zone`: the user's zone if it names one, else the default.
    pub fn for_user(_time_zone: Option<&str>) -> Self {
        Self::utc()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tz(&self) -> &jiff::tz::TimeZone {
        &self.tz
    }
}
