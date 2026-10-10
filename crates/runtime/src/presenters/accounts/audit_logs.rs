//! Audit browsing filters and CSV formatting shared by legacy and SPA endpoints.
use campfire_db::{
    Timestamp,
    models::audit_log::{self, AuditLog, browsing},
};
use campfire_kit::Error;
use campfire_presentation::time::Zone;

pub fn filters(
    get: impl Fn(&str) -> Option<String>,
) -> campfire_presentation::accounts::audit_logs::Filters {
    use campfire_richtext::ruby::{is_blank, strip};
    campfire_presentation::accounts::audit_logs::Filters {
        actor: get("actor")
            .map(|s| strip(&s).to_owned())
            .filter(|s| !is_blank(s)),
        action: get("audit_action").filter(|s| audit_log::ACTIONS.contains(&s.as_str())),
        target_type: get("target_type").filter(|s| browsing::TARGET_TYPES.contains(&s.as_str())),
        from: get("from")
            .and_then(|s| parse_date(&s))
            .map(|d| d.to_string()),
        to: get("to")
            .and_then(|s| parse_date(&s))
            .map(|d| d.to_string()),
    }
}
/// Date.parse's explicit full-date forms, including Ruby's day/month/year slash order.
/// Relative and partial dates are tracked separately in the parity report.
pub fn parse_date(s: &str) -> Option<jiff::civil::Date> {
    let s = campfire_richtext::ruby::strip(s);
    let s = s.split_once('T').map_or(s, |(date, _)| date);
    if s.split_once('-').is_some_and(|(year, _)| year.len() == 2) {
        return jiff::civil::Date::strptime("%y-%m-%d", s).ok();
    }
    for format in [
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%d/%m/%Y",
        "%d %b %Y",
        "%B %d, %Y",
        "%Y%m%d",
    ] {
        if let Ok(date) = jiff::civil::Date::strptime(format, s) {
            return Some(date);
        }
    }
    jiff::civil::Date::strptime("%y-%m-%d", s).ok()
}
pub fn selection(
    filters: &campfire_presentation::accounts::audit_logs::Filters,
    zone: &Zone,
) -> campfire_kit::Result<browsing::Filters> {
    let bound = |value: &Option<String>, end: bool| -> campfire_kit::Result<Option<Timestamp>> {
        value
            .as_deref()
            .map(|s| {
                let date = s.parse::<jiff::civil::Date>().map_err(Error::internal)?;
                let time = if end {
                    jiff::civil::Time::new(23, 59, 59, 999_999_000).unwrap()
                } else {
                    jiff::civil::Time::midnight()
                };
                let t = date
                    .to_datetime(time)
                    .to_zoned(zone.tz().clone())
                    .map_err(Error::internal)?;
                Ok(Timestamp::from_jiff(t.timestamp()))
            })
            .transpose()
    };
    Ok(browsing::Filters {
        actor: filters.actor.clone(),
        action: filters.action.clone(),
        target_type: filters.target_type.clone(),
        from: bound(&filters.from, false)?,
        to: bound(&filters.to, true)?,
    })
}

pub fn csv_body(entries: &[AuditLog], zone: &Zone) -> String {
    fn safe(s: Option<&str>) -> Option<String> {
        s.map(|s| {
            if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
                format!("'{s}")
            } else {
                s.into()
            }
        })
    }
    fn cell(value: Option<String>) -> String {
        match value {
            None => String::new(),
            Some(s) if s.is_empty() || s.contains([',', '"', '\n', '\r']) => {
                format!("\"{}\"", s.replace('"', "\"\""))
            }
            Some(s) => s,
        }
    }
    let mut csv = "time,action,actor,target_type,target,changes,ip_address,user_agent\n".to_owned();
    for row in entries {
        let cells = [
            Some(zone.iso8601(row.created_at.jiff())),
            Some(row.action.clone()),
            safe(row.actor_label.as_deref()),
            row.target_type.clone(),
            safe(row.target_label.as_deref()),
            safe(Some(&campfire_presentation::helpers::to_rails_json(
                &row.details,
            ))),
            safe(row.ip_address.as_deref()),
            safe(row.user_agent.as_deref()),
        ];
        csv.push_str(&cells.into_iter().map(cell).collect::<Vec<_>>().join(","));
        csv.push('\n');
    }
    csv
}
