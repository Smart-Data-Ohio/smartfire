//! app/controllers/slack read facts, independent of the import route adapters.
use crate::app::AppCtx;
use crate::integrations::slack::oauth;
use crate::presenters::pagination::Page;
use campfire_db::{
    User,
    models::slack::{SlackConnection, SlackWorkspace},
    models::slack_import::SlackImport,
};
use campfire_kit::{Ctx, Error, Result};
use campfire_presentation::slack::{Issue, RoomTarget, RunData, RunSummary, SetupData};
use rusqlite::{OptionalExtension, params};
use serde_json::Value;

/// The setup page's state for administrator `uid`: the app credentials, their Slack connection,
/// the active run and the manifest.
pub async fn setup_data(c: &Ctx, uid: i64) -> Result<SetupData> {
    let crypto = std::sync::Arc::new(rails_compat::ar_encryption::ArEncryption::new(
        &c.app().secrets,
    ));
    let mut data=c.app().db.read(move|conn| {
  let w=SlackWorkspace::current(conn)?;let connection=SlackConnection::for_user(conn,uid)?;
  let active=conn.query_row("SELECT id,kind,mode,status FROM slack_imports WHERE status IN ('queued','running','undoing') ORDER BY created_at DESC, id DESC LIMIT 1",[],|r|Ok(RunSummary{id:r.get(0)?,kind:r.get(1)?,mode:r.get(2)?,status:r.get(3)?})).optional()?;
  Ok(SetupData{client_id:w.as_ref().and_then(|w|w.client_id.clone()),configured:w.as_ref().map(|w|w.app_configured(&crypto)).transpose()?.unwrap_or(false),configured_by:w.as_ref().and_then(|w|w.configured_by_id).map(|id|User::find_by_id(conn,id)).transpose()?.flatten().map(|u|u.display_name().to_owned()),team_name:w.as_ref().and_then(|w|w.team_name.clone()).filter(|s|!campfire_richtext::ruby::is_blank(s)),team_known:w.as_ref().and_then(|w|w.team_id.as_deref()).is_some_and(|s|!campfire_richtext::ruby::is_blank(s)),connection_exists:connection.is_some(),connected:connection.as_ref().map(|c|c.connected(&crypto)).transpose()?.unwrap_or(false),disconnected_reason:connection.and_then(|c|c.disconnected_reason).filter(|s|!campfire_richtext::ruby::is_blank(s)),active_run:active,..Default::default()})
 }).await.map_err(Error::internal)?;
    data.manifest = oauth::manifest(&c.url_for(""));
    Ok(data)
}
/// Where the classic `Connect Slack` link starts the OAuth round trip, coming back to the
/// administrator's setup page or the personal imports page.
pub fn connect_path(admin: bool) -> &'static str {
    if admin {
        "/slack/oauth/start?return_to=%2Faccount%2Fslack_import"
    } else {
        "/slack/oauth/start?return_to=%2Fslack%2Fimports"
    }
}
/// The person's saved time zone, as the run pages show times and read dates in it.
pub async fn zone(c: &Ctx, id: i64) -> Result<campfire_presentation::time::Zone> {
    let saved = c
        .app()
        .db
        .read(move |conn| User::saved_time_zone(conn, id))
        .await
        .map_err(Error::internal)?;
    Ok(campfire_presentation::time::Zone::for_user(
        saved.as_deref(),
    ))
}
pub fn data(
    conn: &rusqlite::Connection,
    run: SlackImport,
    now: campfire_db::Timestamp,
) -> campfire_db::Result<RunData> {
    let name = User::find_by_id(conn, run.user_id)?
        .ok_or(campfire_db::Error::RecordNotFound("User"))?
        .name;
    let issues_count = conn.query_row(
        "SELECT COUNT(*) FROM slack_import_issues WHERE slack_import_id=?",
        [run.id],
        |r| r.get(0),
    )?;
    let queued_behind=run.status=="queued" && conn.query_row("SELECT EXISTS(SELECT 1 FROM slack_imports WHERE id!=? AND status IN ('queued','running','undoing'))",[run.id],|r|r.get::<_,bool>(0))?;
    let undo_reason = run.undo_blocked_reason(conn, now)?;
    Ok(RunData {
        id: run.id,
        kind: run.kind,
        mode: run.mode,
        status: run.status,
        options: run.options,
        stats: run.stats,
        error: run.error,
        created_at: run.created_at.to_string(),
        started_at: run.started_at.map(|t| t.to_string()),
        finished_at: run.finished_at.map(|t| t.to_string()),
        user_name: name,
        issues_count,
        queued_behind,
        undo_reason,
        ..Default::default()
    })
}
/// Every run newest first (`None`), or one person's personal runs.
pub async fn list(c: &Ctx, uid: Option<i64>) -> Result<Vec<RunData>> {
    let now = campfire_db::Timestamp::from_jiff(c.now());
    c.app().db.read(move |conn|{
        let mut stmt=conn.prepare("SELECT id FROM slack_imports WHERE (? IS NULL OR (user_id=? AND kind='personal')) ORDER BY created_at DESC, id DESC")?;
        stmt.query_map(params![uid,uid],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?.into_iter().map(|id|data(conn,SlackImport::find(conn,id)?.unwrap(),now)).collect()
    }).await.map_err(Error::internal)
}
/// The personal page's setup state for `uid`: whether an administrator set up the import, and
/// the person's own Slack connection.
pub async fn personal_data(c: &Ctx, uid: i64) -> Result<campfire_presentation::slack::SetupData> {
    let crypto = std::sync::Arc::new(rails_compat::ar_encryption::ArEncryption::new(
        &c.app().secrets,
    ));
    c.app()
        .db
        .read(move |conn| {
            let w = campfire_db::models::slack::SlackWorkspace::current(conn)?;
            let connection = SlackConnection::for_user(conn, uid)?;
            Ok(campfire_presentation::slack::SetupData {
                team_known: w
                    .and_then(|w| w.team_id)
                    .is_some_and(|s| !campfire_richtext::ruby::is_blank(&s)),
                connection_exists: connection.is_some(),
                connected: connection
                    .as_ref()
                    .map(|c| c.connected(&crypto))
                    .transpose()?
                    .unwrap_or(false),
                disconnected_reason: connection
                    .and_then(|c| c.disconnected_reason)
                    .filter(|s| !campfire_richtext::ruby::is_blank(s)),
                ..Default::default()
            })
        })
        .await
        .map_err(Error::internal)
}
/// A page of a run's issues (50 a page, oldest first) and the next page's number, if any.
pub fn issues_page(
    conn: &rusqlite::Connection,
    id: i64,
    count: i64,
    page: Option<&str>,
) -> campfire_db::Result<(Vec<Issue>, Option<i64>)> {
    let page = Page::new(page, count, &[50]);
    let mut stmt=conn.prepare("SELECT level,slack_ref,message FROM slack_import_issues WHERE slack_import_id=? ORDER BY id LIMIT ? OFFSET ?")?;
    let issues = stmt
        .query_map(params![id, page.limit(), page.offset()], |r| {
            Ok(Issue {
                level: r.get(0)?,
                slack_ref: r.get(1)?,
                message: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok((issues, (!page.is_last()).then(|| page.next_param())))
}
/// A completed dry run's plan: the run with its samples rendered, the rooms a conversation can
/// merge into (by name), and a test import's default oldest day (two weeks back, in the
/// person's time zone).
pub async fn plan_data(
    c: &Ctx,
    user: &User,
    run: SlackImport,
) -> Result<(RunData, Vec<RoomTarget>, String)> {
    let now = campfire_db::Timestamp::from_jiff(c.now());
    let secrets = c.app().secrets.clone();
    let host = crate::presenters::page::renderer_base_url(c);
    let (data,rooms)=c.app().db.read(move |conn|{
        let mut data=data(conn,run,now)?;
        data.sample_htmls=sample_htmls(conn,&secrets,now.jiff(),data.samples(),Some(host))?;
        let mut stmt=conn.prepare("SELECT id,name FROM rooms WHERE deleted_at IS NULL AND type IN ('Rooms::Open','Rooms::Closed') ORDER BY LOWER(name)")?;
        let rooms=stmt.query_map([],|r|Ok(RoomTarget{id:r.get(0)?,name:r.get(1)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok((data,rooms))
    }).await.map_err(Error::internal)?;
    let zone = zone(c, user.id).await?;
    let oldest = c
        .now()
        .to_zoned(zone.tz().clone())
        .date()
        .checked_sub(jiff::Span::new().days(14))
        .map_err(Error::internal)?
        .to_string();
    Ok((data, rooms, oldest))
}

pub fn sample_htmls(
    conn: &rusqlite::Connection,
    secrets: &rails_compat::Secrets,
    now: jiff::Timestamp,
    samples: &[Value],
    host: Option<String>,
) -> campfire_db::Result<Vec<String>> {
    let resolver = crate::presenters::rich_text::DbResolver::new(conn, secrets, now);
    let context = resolver.render_context(host.clone());
    samples
        .iter()
        .map(|sample| {
            let source = oauth::string(&sample["markdown"]);
            let markdown = campfire_richtext::markdown::render(&source, &|_: &str| None, &resolver)
                .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            campfire_richtext::markdown::presentation(
                &markdown,
                &context,
                &resolver,
                host.as_deref(),
            )
            .map_err(|e| campfire_db::Error::Other(e.to_string()))
        })
        .collect()
}
