//! `Accounts::AuditLogsController`: authorization, pagination and export presentation.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
};
use campfire_db::models::audit_log::browsing;
use campfire_kit::{Ctx, Error, Result, SendOptions, format};
use campfire_presentation::time::Zone;
pub use campfire_runtime::presenters::accounts::audit_logs::*;

#[cfg(any(test, feature = "test-support"))]
#[derive(Clone)]
pub struct TestExportLimit(pub i64);

pub async fn show(c: &mut Ctx) -> Result {
    #[cfg(any(test, feature = "test-support"))]
    if let Some(limit) = c.current::<TestExportLimit>().cloned() {
        return show_with_limit(c, limit.0, &limit.0.to_string()).await;
    }
    show_with_limit(c, browsing::CSV_EXPORT_LIMIT, "5,000").await
}
// The original tests temporarily replace the export-limit constant. Keep the
// same action body and permission/selection path when replaying those overrides.
async fn show_with_limit(c: &mut Ctx, csv_export_limit: i64, _export_limit_label: &str) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    c.no_store();
    c.set_header("pragma", "no-cache");
    let csv = c.formats()?.first().is_some_and(|f| f.is("csv"));
    if csv {
        concerns::sudo::require_sudo_mode(c).await?;
    }
    if *c.respond_to(&[&format::HTML, &format::CSV])? == format::HTML {
        return campfire_runtime::navigation::redirect(c).await;
    }
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
    let (count, entries) = c.app().db.read(move |conn| {
        Ok((selection.count(conn)?, selection.entries(conn, csv_export_limit, 0)?))
    }).await.map_err(Error::internal)?;
    let truncated = count > csv_export_limit;
    if csv {
        let suffix = if truncated {
            format!("-truncated-to-{csv_export_limit}")
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
    campfire_runtime::navigation::redirect(c).await
}
