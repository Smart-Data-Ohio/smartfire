//! `Accounts::AuditLogsController`: authorization, pagination and export presentation.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    controllers::presenters::{page::framed_page, pagination::Page},
};
use campfire_db::{
    Timestamp,
    models::audit_log::{self, AuditLog, browsing},
};
use campfire_kit::{Ctx, Error, Result, SendOptions, StatusCode, format};
use campfire_views::{accounts::audit_logs as views, time::Zone};

pub async fn show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    c.no_store();
    c.set_header("pragma", "no-cache");
    let csv = c.formats()?.first().is_some_and(|f| f.is("csv"));
    if csv {
        concerns::sudo::require_sudo_mode(c)?;
    }
    c.respond_to(&[&format::HTML, &format::CSV])?;
    let id = concerns::require_current_user(c)?.id;
    let time_zone = c
        .app()
        .db
        .read(move |conn| {
            campfire_db::models::user::profile_settings::appearance(conn, id).map(|s| s.time_zone)
        })
        .await
        .map_err(Error::internal)?;
    let zone = Zone::for_user(time_zone.as_deref());
    let filters = filters(|key| c.param_str(key).map(str::to_owned));
    let selection = selection(&filters, &zone)?;
    let page_param = c.param_str("page").map(str::to_owned);
    let (count, page, entries) = c
        .app()
        .db
        .read(move |conn| {
            let count = selection.count(conn)?;
            let page = Page::new(page_param.as_deref(), count, &[browsing::PAGE_SIZE]);
            let entries = selection.entries(
                conn,
                if csv {
                    browsing::CSV_EXPORT_LIMIT
                } else {
                    page.limit()
                },
                if csv { 0 } else { page.offset() },
            )?;
            Ok((count, page, entries))
        })
        .await
        .map_err(Error::internal)?;
    let truncated = count > browsing::CSV_EXPORT_LIMIT;
    if csv {
        let suffix = if truncated {
            format!("-truncated-to-{}", browsing::CSV_EXPORT_LIMIT)
        } else {
            String::new()
        };
        let filename = format!(
            "audit-log-{}{suffix}.csv",
            zone.format(c.now(), "%Y%m%d-%H%M%S")
        );
        return Ok(c.send_data(
            csv_body(&entries, &zone),
            SendOptions {
                filename: Some(filename),
                content_type: Some("text/csv".into()),
                ..Default::default()
            },
        ));
    }
    let entries = entries.into_iter().map(entry).collect::<Vec<_>>();
    let next_page = (!page.is_last()).then(|| page.next_param().to_string());
    framed_page!(c, StatusCode::OK, |ctx| views::Show {
        ctx,
        filters: filters.clone(),
        entries: entries.clone(),
        actions: audit_log::actions(),
        target_types: browsing::TARGET_TYPES.iter().map(|s| (*s).into()).collect(),
        export_truncated: truncated,
        export_limit: "5,000".into(),
        first_page: page.number == 1,
        next_page: next_page.clone()
    })
    .await
}

pub(super) fn filters(get: impl Fn(&str) -> Option<String>) -> views::Filters {
    use campfire_richtext::ruby::{is_blank, strip};
    views::Filters {
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
pub(super) fn parse_date(s: &str) -> Option<jiff::civil::Date> {
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
pub(super) fn selection(
    filters: &views::Filters,
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
pub(super) fn entry(row: AuditLog) -> views::Entry {
    fn label(s: Option<String>) -> String {
        s.filter(|s| !campfire_richtext::ruby::is_blank(s))
            .unwrap_or_else(|| "—".into())
    }
    views::Entry {
        created_at: row.created_at.jiff(),
        action: row.action,
        actor: label(row.actor_label),
        target: label(row.target_label),
        changes: views::changes_summary(&row.details),
        ip_address: label(row.ip_address),
    }
}
pub(super) fn csv_body(entries: &[AuditLog], zone: &Zone) -> String {
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
            safe(Some(&campfire_views::helpers::to_rails_json(&row.details))),
            safe(row.ip_address.as_deref()),
            safe(row.user_agent.as_deref()),
        ];
        csv.push_str(&cells.into_iter().map(cell).collect::<Vec<_>>().join(","));
        csv.push('\n');
    }
    csv
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_tests;
