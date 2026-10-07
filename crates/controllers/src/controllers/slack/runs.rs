//! `accounts/slack_import_runs_controller.rb` and `slack/imports_controller.rb`.
use super::*;
use crate::controllers::presenters::{page::framed_page, pagination::Page};
use campfire_db::{
    audit_log::{AuditLog, NewAuditLog, Target},
    models::{
        slack::SlackConnection,
        slack_import::{Kind, Mode, NewImport, SlackImport},
    },
};
use campfire_kit::StatusCode;
use campfire_views::slack::{Issue, PersonalIndex, Plan, RoomTarget, RunData, RunList, RunPage};
use rusqlite::params;
use serde_json::Value;

async fn before(c: &mut Ctx, admin: bool) -> Result<User> {
    concerns::before_actions(c, Before::default()).await?;
    if admin {
        concerns::ensure_can_administer(c)?;
    }
    Ok(concerns::require_current_user(c)?.clone())
}
fn base(admin: bool) -> &'static str {
    if admin {
        "/account/slack_import/runs"
    } else {
        "/slack/imports"
    }
}
fn path(admin: bool, id: i64) -> String {
    format!("{}/{id}", base(admin))
}
/// `set_run`: the run in the `:id` path parameter, an administrator's any run and anyone else's
/// only their own personal runs; anything else is `head :not_found`.
pub async fn find(c: &mut Ctx, user: &User, admin: bool) -> Result<SlackImport> {
    let id = concerns::ruby_to_i(&param(c, "id"));
    let uid = user.id;
    c.app()
        .db
        .read(move |conn| {
            Ok(SlackImport::find(conn, id)?
                .filter(|r| admin || (r.user_id == uid && r.kind == "personal")))
        })
        .await
        .map_err(Error::internal)?
        .ok_or_else(|| {
            // Both Rails controllers render `head :not_found` in set_run.
            Error::Halt(Box::new(c.head(StatusCode::NOT_FOUND)))
        })
}
async fn log(c: &Ctx, user: &User, id: i64, action: &str, changes: Option<Value>) -> Result<()> {
    let context = audit(c, user)?;
    let action = action.to_owned();
    c.app()
        .db
        .write(move |tx| {
            AuditLog::record(
                tx,
                NewAuditLog {
                    action,
                    target: Some(Target {
                        record_type: "SlackImport".into(),
                        id,
                        label: Some(format!("SlackImport #{id}")),
                    }),
                    changes,
                    ..Default::default()
                },
                &context,
            )?;
            Ok(())
        })
        .await
        .map_err(Error::internal)
}
async fn connection(c: &Ctx, uid: i64) -> Result<Option<SlackConnection>> {
    let crypto = service(c).encryption;
    c.app()
        .db
        .read(move |conn| {
            let connection = SlackConnection::for_user(conn, uid)?;
            match connection {
                Some(ref grant) if grant.connected(&crypto)? => Ok(connection),
                _ => Ok(None),
            }
        })
        .await
        .map_err(Error::internal)
}
/// Whether a run is queued, running or undoing: anyone's (`None`), or the person's own.
fn any_active(conn: &rusqlite::Connection, uid: Option<i64>) -> campfire_db::Result<bool> {
    Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM slack_imports WHERE status IN ('queued','running','undoing') AND (? IS NULL OR user_id=?))",params![uid,uid],|r|r.get(0))?)
}
async fn active(c: &Ctx, uid: Option<i64>) -> Result<bool> {
    c.app().db.read(move |conn| any_active(conn, uid)).await.map_err(Error::internal)
}
async fn blocker(c: &Ctx, uid: i64) -> Result<Option<&'static str>> {
    if connection(c, uid).await?.is_none() {
        Ok(Some("Connect your Slack account first."))
    } else if active(c, None).await? {
        Ok(Some(
            "Another import is already running. Wait for it to finish.",
        ))
    } else {
        Ok(None)
    }
}
/// The checked conversation ids: one id or a list, blanks dropped.
pub fn selected(value: Value) -> Vec<Value> {
    let values = match value {
        Value::Null => vec![],
        Value::Array(a) => a,
        other => vec![other],
    };
    values.into_iter().filter(oauth::present).collect()
}
fn known_selection(selected: Vec<Value>, run: &SlackImport) -> Vec<Value> {
    let known: Vec<String> = run.stats["conversations"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|e| oauth::string(&e["id"]))
        .collect();
    let mut ids = vec![];
    for id in selected {
        if known.iter().any(|k| id.as_str() == Some(k)) && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}
/// The person's saved time zone, as the run pages show times and read dates in it.
pub async fn zone(c: &Ctx, id: i64) -> Result<campfire_views::time::Zone> {
    let saved = c
        .app()
        .db
        .read(move |conn| User::saved_time_zone(conn, id))
        .await
        .map_err(Error::internal)?;
    Ok(campfire_views::time::Zone::for_user(saved.as_deref()))
}
fn bound(zone: &campfire_views::time::Zone, value: &str, latest: bool) -> Option<String> {
    let date = value.parse::<jiff::civil::Date>().ok()?;
    let time = if latest {
        jiff::civil::Time::new(23, 59, 59, 0).ok()?
    } else {
        jiff::civil::Time::midnight()
    };
    let zoned = date.to_datetime(time).to_zoned(zone.tz().clone()).ok()?;
    Some(zone.iso8601(zoned.timestamp()))
}
/// Creates the run, unless one is active by then (anyone's for a workspace run, the person's own
/// for a personal one): the check and the insert share one write, so two starts racing past the
/// callers' earlier check can't both begin. `None` is the lost race, refused as the check is.
async fn start(
    c: &Ctx,
    user: &User,
    workspace: i64,
    connection: i64,
    kind: Kind,
    mode: Mode,
    options: Value,
) -> Result<Option<SlackImport>> {
    let scope = match kind {
        Kind::Workspace => None,
        Kind::Personal => Some(user.id),
    };
    let options = crate::integrations::slack::options::normalize(&options)
        .map_err(|s| Error::internal(campfire_db::Error::Other(s)))?;
    let user_id = user.id;
    let zone = zone(c, user.id).await?;
    let stamp = zone.format(
        c.now(),
        if zone.tz().to_offset_info(c.now()).abbreviation() == "UTC" {
            "%Y-%m-%dT%H:%M:%S.%6fZ"
        } else {
            "%Y-%m-%dT%H:%M:%S.%6f%:z"
        },
    );
    c.app()
        .db
        .write(move |tx| {
            if any_active(tx.conn(), scope)? {
                return Ok(None);
            }
            SlackImport::create_with_enqueued_at(
                tx,
                NewImport {
                    workspace_id: workspace,
                    connection_id: Some(connection),
                    user_id,
                    kind,
                    mode,
                    options,
                },
                Some(&stamp),
            )
            .map(Some)
        })
        .await
        .map_err(Error::internal)
}
/// What starting a run came to: the new run and the notice the classic page flashes, or the
/// classic page's alert and the page it sends the person back to.
#[derive(Debug)]
pub enum Started {
    Run {
        run: Box<SlackImport>,
        notice: &'static str,
    },
    Refused {
        path: String,
        alert: &'static str,
    },
}

fn refused(path: impl Into<String>, alert: &'static str) -> Started {
    Started::Refused {
        path: path.into(),
        alert,
    }
}

/// A start's answer on the classic pages: the run with its notice, or back with the alert.
fn answer(c: &mut Ctx, admin: bool, started: Started) -> Result {
    match started {
        Started::Run { run, notice } => redirect(c, &path(admin, run.id), Some(notice), None),
        Started::Refused { path, alert } => redirect(c, &path, None, Some(alert)),
    }
}

/// `accounts/slack_import_runs#create`'s form: a workspace dry run. The dates are the form's
/// `YYYY-MM-DD` text, read in the person's time zone; anything else is no bound.
#[derive(Debug, Clone, Default)]
pub struct DryRunForm {
    pub include_private: bool,
    pub oldest: String,
    pub latest: String,
}

/// `slack/imports#create`'s form: a preview, or an import of the checked conversations of a
/// completed preview.
#[derive(Debug, Clone, Default)]
pub struct PersonalForm {
    pub import: bool,
    pub dry_run_id: i64,
    pub conversation_ids: Vec<Value>,
}

/// `accounts/slack_import_runs#start_import`'s form: the checked conversations, each one's
/// target (`"new"`, `"skip"` or a room id, as text), the preset, and a test import's dates.
#[derive(Debug, Clone, Default)]
pub struct ImportForm {
    pub conversation_ids: Vec<Value>,
    pub room_targets: Value,
    pub full: bool,
    pub oldest: String,
    pub latest: String,
}

/// `conversation_ids[]` as the classic forms post it.
fn selected_param(c: &Ctx) -> Vec<Value> {
    selected(
        c.params
            .get("conversation_ids")
            .map(Param::to_json)
            .unwrap_or(Value::Null),
    )
}

pub async fn admin_create(c: &mut Ctx) -> Result {
    let user = before(c, true).await?;
    let form = DryRunForm {
        include_private: param(c, "include_private") != "0",
        oldest: param(c, "oldest"),
        latest: param(c, "latest"),
    };
    let started = start_dry_run(c, &user, form).await?;
    answer(c, true, started)
}

/// `accounts/slack_import_runs#create` once the administrator is known: a workspace dry run.
pub async fn start_dry_run(c: &Ctx, user: &User, form: DryRunForm) -> Result<Started> {
    let workspace = service(c)
        .current_workspace()
        .await
        .map_err(Error::internal)?;
    let connection = connection(c, user.id).await?;
    if workspace
        .as_ref()
        .and_then(|w| w.team_id.as_deref())
        .is_none_or(campfire_richtext::ruby::is_blank)
        || connection.is_none()
    {
        return Ok(refused(
            "/account/slack_import",
            "Connect your Slack account first.",
        ));
    }
    if active(c, None).await? {
        return Ok(refused(
            base(true),
            "Another import is already running. Wait for it to finish.",
        ));
    }
    let zone = zone(c, user.id).await?;
    let options = json!({"include_private":form.include_private,"oldest":bound(&zone,&form.oldest,false),"latest":bound(&zone,&form.latest,true)});
    let Some(run) = start(
        c,
        user,
        workspace.unwrap().id,
        connection.unwrap().id,
        Kind::Workspace,
        Mode::DryRun,
        options,
    )
    .await?
    else {
        return Ok(refused(
            base(true),
            "Another import is already running. Wait for it to finish.",
        ));
    };
    log(
        c,
        user,
        run.id,
        "slack.import.start",
        Some(json!({"kind":"workspace","mode":"dry_run"})),
    )
    .await?;
    Ok(Started::Run {
        run: Box::new(run),
        notice: "Dry run started.",
    })
}

pub async fn personal_create(c: &mut Ctx) -> Result {
    let user = before(c, false).await?;
    let form = PersonalForm {
        import: param(c, "mode") == "import",
        dry_run_id: concerns::ruby_to_i(&param(c, "dry_run_id")),
        conversation_ids: selected_param(c),
    };
    let started = start_personal(c, &user, form).await?;
    answer(c, false, started)
}

/// `slack/imports#create` once the person is known: a preview, or an import from one.
pub async fn start_personal(c: &Ctx, user: &User, form: PersonalForm) -> Result<Started> {
    let workspace = service(c)
        .current_workspace()
        .await
        .map_err(Error::internal)?;
    if workspace
        .as_ref()
        .and_then(|w| w.team_id.as_deref())
        .is_none_or(campfire_richtext::ruby::is_blank)
    {
        return Ok(refused(
            base(false),
            "An administrator needs to set up Slack import first.",
        ));
    }
    let Some(connection) = connection(c, user.id).await? else {
        return Ok(refused(base(false), "Connect your Slack account first."));
    };
    if active(c, Some(user.id)).await? {
        return Ok(refused(
            base(false),
            "You already have an import running. Wait for it to finish.",
        ));
    }
    let importing = form.import;
    let options = if importing {
        let id = form.dry_run_id;
        let uid = user.id;
        let preview = c
            .app()
            .db
            .read(move |conn| {
                Ok(SlackImport::find(conn, id)?.filter(|r| {
                    r.user_id == uid
                        && r.kind == "personal"
                        && r.mode == "dry_run"
                        && r.status == "completed"
                }))
            })
            .await
            .map_err(Error::internal)?;
        let Some(preview) = preview else {
            return Ok(refused(base(false), "Run a preview first."));
        };
        let ids = form.conversation_ids;
        if ids.is_empty() {
            return Ok(refused(
                path(false, id),
                "Check at least one conversation to import.",
            ));
        }
        let ids = known_selection(ids, &preview);
        if ids.is_empty() {
            return Ok(refused(
                path(false, id),
                "Those conversations are not in the preview.",
            ));
        }
        json!({"conversation_ids":ids})
    } else {
        json!({})
    };
    let mode = if importing {
        Mode::Import
    } else {
        Mode::DryRun
    };
    let Some(run) = start(
        c,
        user,
        workspace.unwrap().id,
        connection.id,
        Kind::Personal,
        mode,
        options,
    )
    .await?
    else {
        return Ok(refused(
            base(false),
            "You already have an import running. Wait for it to finish.",
        ));
    };
    log(
        c,
        user,
        run.id,
        "slack.import.start",
        Some(json!({"kind":"personal","mode":if importing {"import"}else{"dry_run"}})),
    )
    .await?;
    Ok(Started::Run {
        run: Box::new(run),
        notice: if importing {
            "Import started."
        } else {
            "Preview started."
        },
    })
}

pub async fn start_import(c: &mut Ctx) -> Result {
    let user = before(c, true).await?;
    let run = find(c, &user, true).await?;
    let form = ImportForm {
        conversation_ids: selected_param(c),
        room_targets: c
            .params
            .get("room_targets")
            .map(Param::to_json)
            .unwrap_or(Value::Null),
        full: param(c, "preset") == "full",
        oldest: param(c, "oldest"),
        latest: param(c, "latest"),
    };
    let started = start_workspace_import(c, &user, run, form).await?;
    answer(c, true, started)
}

/// `accounts/slack_import_runs#start_import` once the run is found: a test or full import of a
/// completed dry run's checked conversations.
pub async fn start_workspace_import(
    c: &Ctx,
    user: &User,
    run: SlackImport,
    form: ImportForm,
) -> Result<Started> {
    if run.kind != "workspace" || run.mode != "dry_run" || run.status != "completed" {
        return Ok(refused(
            path(true, run.id),
            "Start from a completed dry run.",
        ));
    }
    let plan_path = format!("{}/plan", path(true, run.id));
    if let Some(reason) = blocker(c, user.id).await? {
        return Ok(refused(plan_path, reason));
    }
    let ids = form.conversation_ids;
    if ids.is_empty() {
        return Ok(refused(
            plan_path,
            "Check at least one conversation to import.",
        ));
    }
    let ids = known_selection(ids, &run);
    if ids.is_empty() {
        return Ok(refused(
            plan_path,
            "Those conversations are not in the dry run.",
        ));
    }
    let alive=c.app().db.read(|conn| {let mut stmt=conn.prepare("SELECT id FROM rooms WHERE deleted_at IS NULL AND type IN ('Rooms::Open','Rooms::Closed')")?;Ok(stmt.query_map([],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)}).await.map_err(Error::internal)?;
    let nested = form.room_targets;
    let mut targets = serde_json::Map::new();
    for id in &ids {
        let id = id.as_str().unwrap();
        if let Some(value) = nested.as_object().and_then(|m| m.get(id)) {
            let text = oauth::string(value);
            if text == "new" || text == "skip" {
                targets.insert(id.into(), json!(text));
            } else if !text.is_empty()
                && text.bytes().all(|b| b.is_ascii_digit())
                && text.parse::<i64>().ok().is_some_and(|n| alive.contains(&n))
            {
                targets.insert(id.into(), json!(text.parse::<i64>().unwrap()));
            }
        }
    }
    let full = form.full;
    let mut options = json!({"conversation_ids":ids,"include_private":run.options["include_private"]!=false,"room_targets":targets});
    if !full {
        let zone = zone(c, user.id).await?;
        let day = c
            .now()
            .to_zoned(zone.tz().clone())
            .date()
            .checked_sub(jiff::Span::new().days(14))
            .map_err(Error::internal)?;
        options["oldest"] = json!(bound(&zone, &form.oldest, false).unwrap_or_else(|| {
            zone.iso8601(
                day.at(0, 0, 0, 0)
                    .to_zoned(zone.tz().clone())
                    .unwrap()
                    .timestamp(),
            )
        }));
        options["latest"] = json!(bound(&zone, &form.latest, true));
    }
    let connection = connection(c, user.id).await?.ok_or(Error::NotFound)?;
    let Some(created) = start(
        c,
        user,
        run.slack_workspace_id,
        connection.id,
        Kind::Workspace,
        Mode::Import,
        options,
    )
    .await?
    else {
        return Ok(refused(
            plan_path,
            "Another import is already running. Wait for it to finish.",
        ));
    };
    log(
        c,
        user,
        created.id,
        "slack.import.start",
        Some(json!({"kind":"workspace","mode":"import","preset":if full {"full"}else{"test"}})),
    )
    .await?;
    Ok(Started::Run {
        run: Box::new(created),
        notice: if full {
            "Full import started."
        } else {
            "Test import started."
        },
    })
}

pub async fn catch_up(c: &mut Ctx) -> Result {
    let user = before(c, true).await?;
    let run = find(c, &user, true).await?;
    let started = start_catch_up(c, &user, run).await?;
    answer(c, true, started)
}

/// `accounts/slack_import_runs#catch_up` once the run is found: the same conversations and
/// targets again, from a completed full import.
pub async fn start_catch_up(c: &Ctx, user: &User, run: SlackImport) -> Result<Started> {
    if run.kind != "workspace"
        || run.mode != "import"
        || run.status != "completed"
        || oauth::present(&run.options["oldest"])
    {
        return Ok(refused(
            path(true, run.id),
            "Catch-up starts from a completed full import.",
        ));
    }
    if let Some(reason) = blocker(c, user.id).await? {
        return Ok(refused(path(true, run.id), reason));
    }
    let mut options = serde_json::Map::new();
    for key in ["conversation_ids", "room_targets", "include_private"] {
        if let Some(v) = run.options.get(key) {
            options.insert(key.into(), v.clone());
        }
    }
    let connection = connection(c, user.id).await?.ok_or(Error::NotFound)?;
    let Some(created) = start(
        c,
        user,
        run.slack_workspace_id,
        connection.id,
        Kind::Workspace,
        Mode::Import,
        json!(options),
    )
    .await?
    else {
        return Ok(refused(
            path(true, run.id),
            "Another import is already running. Wait for it to finish.",
        ));
    };
    log(
        c,
        user,
        created.id,
        "slack.import.start",
        Some(json!({"kind":"workspace","mode":"import","preset":"catch_up"})),
    )
    .await?;
    Ok(Started::Run {
        run: Box::new(created),
        notice: "Catch-up import started.",
    })
}

async fn mutate(c: &mut Ctx, admin: bool, undo: bool) -> Result {
    let user = before(c, admin).await?;
    let run = find(c, &user, admin).await?;
    let id = run.id;
    match change(c, &user, id, undo).await? {
        Ok(notice) => redirect(c, &path(admin, id), Some(notice), None),
        Err(reason) => redirect(c, &path(admin, id), None, Some(&reason)),
    }
}

/// `cancel` and `undo` once the run is found: the notice when it changed, else the reason it
/// couldn't.
pub async fn change(
    c: &Ctx,
    user: &User,
    id: i64,
    undo: bool,
) -> Result<std::result::Result<&'static str, String>> {
    let changed = c
        .app()
        .db
        .write(move |tx| {
            if undo {
                SlackImport::undo(tx, id)
            } else {
                SlackImport::cancel(tx, id)
            }
        })
        .await
        .map_err(Error::internal)?;
    if changed {
        log(
            c,
            user,
            id,
            if undo {
                "slack.import.undo"
            } else {
                "slack.import.cancel"
            },
            None,
        )
        .await?;
        Ok(Ok(if undo {
            "Undo started."
        } else {
            "Import cancelled."
        }))
    } else {
        let reason = if undo {
            let now = campfire_db::Timestamp::from_jiff(c.now());
            c.app()
                .db
                .read(move |conn| {
                    SlackImport::find(conn, id)?
                        .unwrap()
                        .undo_blocked_reason(conn, now)
                })
                .await
                .map_err(Error::internal)?
                .unwrap_or_else(|| "That run cannot be undone.".into())
        } else {
            "That run already finished.".into()
        };
        Ok(Err(reason))
    }
}
pub async fn admin_cancel(c: &mut Ctx) -> Result {
    mutate(c, true, false).await
}
pub async fn admin_undo(c: &mut Ctx) -> Result {
    mutate(c, true, true).await
}
pub async fn personal_cancel(c: &mut Ctx) -> Result {
    mutate(c, false, false).await
}
pub async fn personal_undo(c: &mut Ctx) -> Result {
    mutate(c, false, true).await
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
pub async fn admin_index(c: &mut Ctx) -> Result {
    let user = before(c, true).await?;
    let mut runs = list(c, None).await?;
    let zone = zone(c, user.id).await?;
    for run in &mut runs {
        run.format_times(&zone);
    }
    framed_page!(c, StatusCode::OK, |ctx| RunList { ctx, runs: &runs }).await
}
pub async fn personal_index(c: &mut Ctx) -> Result {
    let user = before(c, false).await?;
    let setup = personal_data(c, user.id).await?;
    let mut runs = list(c, Some(user.id)).await?;
    let zone = zone(c, user.id).await?;
    for run in &mut runs {
        run.format_times(&zone);
    }
    framed_page!(c, StatusCode::OK, |ctx| PersonalIndex {
        ctx,
        data: &setup,
        runs: &runs
    })
    .await
}
/// The personal page's setup state for `uid`: whether an administrator set up the import, and
/// the person's own Slack connection.
pub async fn personal_data(c: &Ctx, uid: i64) -> Result<campfire_views::slack::SetupData> {
    let crypto = service(c).encryption;
    c.app()
        .db
        .read(move |conn| {
            let w = campfire_db::models::slack::SlackWorkspace::current(conn)?;
            let connection = SlackConnection::for_user(conn, uid)?;
            Ok(campfire_views::slack::SetupData {
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
async fn show(c: &mut Ctx, admin: bool, status: bool) -> Result {
    let user = before(c, admin).await?;
    let run = find(c, &user, admin).await?;
    let now = campfire_db::Timestamp::from_jiff(c.now());
    let page_param = c.param_str("page").map(str::to_owned);
    let mut data=c.app().db.read(move |conn| {
        let mut data=data(conn,run,now)?;
        if admin && !status {
            (data.issues, data.next_page) = issues_page(conn, data.id, data.issues_count, page_param.as_deref())?;
        }
        Ok(data)
    }).await.map_err(Error::internal)?;
    data.format_times(&zone(c, user.id).await?);
    if status {
        return crate::controllers::presenters::page::content(c, StatusCode::OK, |_| {
            Ok(campfire_views::slack::status(&data))
        })
        .await;
    }
    framed_page!(c, StatusCode::OK, |ctx| RunPage {
        ctx,
        data: &data,
        admin
    })
    .await
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
pub async fn admin_show(c: &mut Ctx) -> Result {
    show(c, true, false).await
}
pub async fn personal_show(c: &mut Ctx) -> Result {
    show(c, false, false).await
}
pub async fn admin_status(c: &mut Ctx) -> Result {
    show(c, true, true).await
}
pub async fn personal_status(c: &mut Ctx) -> Result {
    show(c, false, true).await
}
pub async fn plan(c: &mut Ctx) -> Result {
    let user = before(c, true).await?;
    let run = find(c, &user, true).await?;
    if run.kind != "workspace" || run.mode != "dry_run" || run.status != "completed" {
        return redirect(
            c,
            &path(true, run.id),
            None,
            Some("The plan is ready when the dry run completes."),
        );
    }
    let (data, rooms, oldest) = plan_data(c, &user, run).await?;
    framed_page!(c, StatusCode::OK, |ctx| Plan {
        ctx,
        data: &data,
        rooms: &rooms,
        oldest: &oldest
    })
    .await
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
    let host = crate::controllers::presenters::page::renderer_base_url(c);
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
    let resolver = crate::controllers::presenters::rich_text::DbResolver::new(conn, secrets, now);
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
