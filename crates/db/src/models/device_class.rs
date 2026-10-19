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
    use super::DeviceClass::{self, Unknown};

    #[test]
    fn future_stored_metadata_is_unknown_instead_of_failing_a_recipient_batch() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        assert_eq!(conn.query_row("SELECT 'future-device'", [], |row| row.get::<_, DeviceClass>(0)).unwrap(), Unknown);
    }
}
