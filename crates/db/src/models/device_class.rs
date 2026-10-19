use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    Desktop,
    Mobile,
    Unknown,
}

impl DeviceClass {
    pub fn name(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Mobile => "mobile",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "desktop" => Some(Self::Desktop),
            "mobile" => Some(Self::Mobile),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    pub fn from_user_agent(user_agent: Option<&str>) -> Self {
        let agent = user_agent.unwrap_or_default().to_ascii_lowercase();
        if ["iphone", "ipad", "ipod", "android", "mobile"].iter().any(|part| agent.contains(part)) {
            Self::Mobile
        } else if ["windows", "macintosh", "x11", "cros", "linux"].iter().any(|part| agent.contains(part)) {
            Self::Desktop
        } else {
            Self::Unknown
        }
    }
}

impl ToSql for DeviceClass {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.name()))
    }
}

impl FromSql for DeviceClass {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(Self::from_name(value.as_str()?).unwrap_or(Self::Unknown))
    }
}

#[cfg(test)]
mod tests {
    use super::DeviceClass::{self, Desktop, Mobile, Unknown};

    #[test]
    fn future_stored_metadata_is_unknown_instead_of_failing_a_recipient_batch() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        assert_eq!(conn.query_row("SELECT 'future-device'", [], |row| row.get::<_, DeviceClass>(0)).unwrap(), Unknown);
    }

    #[test]
    fn legacy_device_classes_require_known_user_agent_markers() {
        for (agent, expected) in [
            (None, Unknown), (Some(""), Unknown), (Some("test"), Unknown),
            (Some("Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)"), Mobile),
            (Some("Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X)"), Mobile),
            (Some("Mozilla/5.0 (Linux; Android 15; Pixel 9)"), Mobile),
            (Some("Mozilla/5.0 (X11; Linux x86_64)"), Desktop),
            (Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64)"), Desktop),
            (Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)"), Desktop),
        ] {
            assert_eq!(DeviceClass::from_user_agent(agent), expected, "{agent:?}");
        }
    }
}
