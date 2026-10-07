//! `/api/v1/admin/slack` and `/api/v1/slack` (S7): the Slack importer, the SPA's twin of the
//! classic `accounts/slack_imports`, `accounts/slack_import_runs` and `slack/imports` pages and
//! `slack/connections#destroy`.
//!
//! Every write runs the classic controller's own path (`campfire_controllers`'
//! `controllers::slack` functions the classic actions call too), after the classic gates in the
//! classic order: the same runs start with the same options, audits and jobs, and a refusal is
//! the classic page's alert. Only the password confirmation differs, as in [`crate::admin`]: a
//! lapsed one answers `SudoRequired`. Connecting a Slack account stays an OAuth round trip on
//! the classic routes.

use axum::Router;
use axum::routing::{get, post};
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_controllers::controllers::slack::{
    self as classic,
    runs::{self, DryRunForm, ImportForm, PersonalForm, Started},
    setup,
};
use campfire_db::{Timestamp, User, models::slack_import::SlackImport};
use campfire_kit::{Ctx, Error, Kit, Result, StatusCode, action, unparsed_action};
use campfire_views::slack::{RunData, SetupData, conversation_type, target_value};
use serde_json::Value;

use crate::admin::{administrator, body, refusal, require_sudo, viewer};
use crate::dto::time;

/// The importer's routes, merged into [`crate::routes`].
pub fn routes() -> Router<Kit> {
    Router::new()
        .route(
            "/api/v1/admin/slack",
            get(action(setup_page))
                .put(unparsed_action(save_credentials))
                .delete(action(remove_credentials)),
        )
        .route(
            "/api/v1/admin/slack/runs",
            get(action(admin_runs)).post(unparsed_action(start_dry_run)),
        )
        .route("/api/v1/admin/slack/runs/{id}", get(action(admin_run)))
        .route(
            "/api/v1/admin/slack/runs/{id}/status",
            get(action(admin_status)),
        )
        .route("/api/v1/admin/slack/runs/{id}/plan", get(action(plan)))
        .route(
            "/api/v1/admin/slack/runs/{id}/import",
            post(unparsed_action(start_import)),
        )
        .route(
            "/api/v1/admin/slack/runs/{id}/catch_up",
            post(action(catch_up)),
        )
        .route(
            "/api/v1/admin/slack/runs/{id}/cancel",
            post(action(admin_cancel)),
        )
        .route(
            "/api/v1/admin/slack/runs/{id}/undo",
            post(action(admin_undo)),
        )
        .route(
            "/api/v1/slack/imports",
            get(action(personal)).post(unparsed_action(start_personal)),
        )
        .route("/api/v1/slack/imports/{id}", get(action(personal_run)))
        .route(
            "/api/v1/slack/imports/{id}/status",
            get(action(personal_status)),
        )
        .route(
            "/api/v1/slack/imports/{id}/cancel",
            post(action(personal_cancel)),
        )
        .route(
            "/api/v1/slack/imports/{id}/undo",
            post(action(personal_undo)),
        )
        .route(
            "/api/v1/slack/connection",
            axum::routing::delete(action(disconnect)),
        )
}

endpoint!(
    /// `GET /api/v1/admin/slack`
    setup_page => show_setup
);
endpoint!(
    /// `PUT /api/v1/admin/slack`
    save_credentials => configure
);
endpoint!(
    /// `DELETE /api/v1/admin/slack`
    remove_credentials => remove
);
endpoint!(
    /// `GET /api/v1/admin/slack/runs`
    admin_runs => index_runs
);
endpoint!(
    /// `POST /api/v1/admin/slack/runs`
    start_dry_run => dry_run
);
endpoint!(
    /// `GET /api/v1/admin/slack/runs/:id`
    admin_run => show_admin_run
);
endpoint!(
    /// `GET /api/v1/admin/slack/runs/:id/status`
    admin_status => show_admin_status
);
endpoint!(
    /// `GET /api/v1/admin/slack/runs/:id/plan`
    plan => show_plan
);
endpoint!(
    /// `POST /api/v1/admin/slack/runs/:id/import`
    start_import => import
);
endpoint!(
    /// `POST /api/v1/admin/slack/runs/:id/catch_up`
    catch_up => start_catch_up
);
endpoint!(
    /// `POST /api/v1/admin/slack/runs/:id/cancel`
    admin_cancel => cancel_admin_run
);
endpoint!(
    /// `POST /api/v1/admin/slack/runs/:id/undo`
    admin_undo => undo_admin_run
);
endpoint!(
    /// `GET /api/v1/slack/imports`
    personal => show_personal
);
endpoint!(
    /// `POST /api/v1/slack/imports`
    start_personal => personal_start
);
endpoint!(
    /// `GET /api/v1/slack/imports/:id`
    personal_run => show_personal_run
);
endpoint!(
    /// `GET /api/v1/slack/imports/:id/status`
    personal_status => show_personal_status
);
endpoint!(
    /// `POST /api/v1/slack/imports/:id/cancel`
    personal_cancel => cancel_personal_run
);
endpoint!(
    /// `POST /api/v1/slack/imports/:id/undo`
    personal_undo => undo_personal_run
);
endpoint!(
    /// `DELETE /api/v1/slack/connection`
    disconnect => drop_connection
);

// --- Wire shapes -------------------------------------------------------------------------------

/// A JSON value as the classic pages print it.
fn text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Ruby's `present?` on a stats value.
fn present(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => false,
        Value::String(text) => !campfire_richtext::ruby::is_blank(text),
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        _ => true,
    }
}

/// A stats count; anything that isn't a number is none.
fn count(value: &Value) -> i64 {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
        .unwrap_or(0)
}

/// A stats text, when present.
fn presence(value: &Value) -> Option<String> {
    present(value).then(|| text(value))
}

/// A run's stored time (`RunData` holds it as the database writes it) on the wire.
fn stamp(text: &str) -> Result<String> {
    Timestamp::parse_db(text)
        .map(time)
        .ok_or_else(|| Error::internal(std::io::Error::other(format!("bad run time {text}"))))
}

fn unknown(what: &str, value: &str) -> Error {
    Error::internal(std::io::Error::other(format!(
        "unknown Slack run {what} {value}"
    )))
}

fn kind(value: &str) -> Result<api::SlackRunKind> {
    match value {
        "workspace" => Ok(api::SlackRunKind::Workspace),
        "personal" => Ok(api::SlackRunKind::Personal),
        other => Err(unknown("kind", other)),
    }
}

fn mode(value: &str) -> Result<api::SlackRunMode> {
    match value {
        "dry_run" => Ok(api::SlackRunMode::DryRun),
        "import" => Ok(api::SlackRunMode::Import),
        other => Err(unknown("mode", other)),
    }
}

fn status(value: &str) -> Result<api::SlackRunStatus> {
    use api::SlackRunStatus::*;
    Ok(match value {
        "queued" => Queued,
        "running" => Running,
        "undoing" => Undoing,
        "completed" => Completed,
        "failed" => Failed,
        "cancelled" => Cancelled,
        "undone" => Undone,
        other => return Err(unknown("status", other)),
    })
}

/// A Slack connection as the classic pages describe it.
fn connection(data: &SetupData) -> api::SlackConnectionState {
    if data.connected {
        api::SlackConnectionState::Connected
    } else if data.connection_exists {
        api::SlackConnectionState::Rejected {
            reason: data.disconnected_reason.clone(),
        }
    } else {
        api::SlackConnectionState::None
    }
}

fn setup_wire(data: SetupData) -> Result<api::SlackSetup> {
    let active_run = data
        .active_run
        .as_ref()
        .map(|run| {
            Ok::<_, Error>(api::SlackRunSummary {
                id: run.id,
                kind: kind(&run.kind)?,
                mode: mode(&run.mode)?,
                status: status(&run.status)?,
            })
        })
        .transpose()?;
    Ok(api::SlackSetup {
        connection: connection(&data),
        client_id: data.client_id,
        configured: data.configured,
        configured_by: data.configured_by,
        team_name: data.team_name,
        team_known: data.team_known,
        active_run,
        manifest: data.manifest,
        connect_path: classic::connect_path(true).into(),
    })
}

fn conversation(value: &Value) -> api::SlackConversation {
    api::SlackConversation {
        id: text(&value["id"]),
        name: text(&value["name"]),
        kind: conversation_type(value),
        archived: !matches!(value["archived"], Value::Null | Value::Bool(false)),
        members: count(&value["members"]),
        messages: count(&value["messages"]),
        threads: count(&value["threads"]),
    }
}

fn row(data: &RunData) -> Result<api::SlackRunRow> {
    Ok(api::SlackRunRow {
        id: data.id,
        kind: kind(&data.kind)?,
        mode: mode(&data.mode)?,
        status: status(&data.status)?,
        started_by: data.user_name.clone(),
        created_at: stamp(&data.created_at)?,
    })
}

/// A run as its classic page's status section and buttons show it.
fn run_wire(data: &RunData) -> Result<api::SlackRun> {
    let stats = &data.stats;
    let users = &stats["users"];
    let people = users
        .as_object()
        .is_some_and(|fields| !fields.is_empty())
        .then(|| api::SlackPeople {
            total: count(&users["total"]),
            matched: count(&users["matched"]),
            placeholders: count(&users["placeholders"]),
            deactivated: count(&users["deactivated"]),
            bots: count(&users["bots"]),
        });
    let counts = &stats["counts"];
    let counts = counts
        .as_object()
        .is_some_and(|fields| !fields.is_empty())
        .then(|| api::SlackCounts {
            rooms_created: count(&counts["rooms_created"]),
            rooms_merged: count(&counts["rooms_merged"]),
            messages: count(&counts["messages"]),
            replies: count(&counts["replies"]),
            threads: count(&counts["threads"]),
            reactions: count(&counts["reactions"]),
            pins: count(&counts["pins"]),
            files_linked: count(&counts["files_linked"]),
            skipped: count(&counts["skipped"]),
        });
    // `issues_label`: the run's own tally while it has one, else the recorded issues.
    let issues_count = if matches!(stats["issues_count"], Value::Null | Value::Bool(false)) {
        data.issues_count
    } else {
        count(&stats["issues_count"])
    };
    let personal_preview = data.kind == "personal" && data.preview();
    Ok(api::SlackRun {
        id: data.id,
        kind: kind(&data.kind)?,
        mode: mode(&data.mode)?,
        status: status(&data.status)?,
        title: data.title(),
        started_by: data.user_name.clone(),
        created_at: stamp(&data.created_at)?,
        started_at: data.started_at.as_deref().map(stamp).transpose()?,
        finished_at: data.finished_at.as_deref().map(stamp).transpose()?,
        phase: presence(&stats["phase"]),
        current: presence(&stats["current"]),
        queued_behind: data.queued_behind,
        people,
        counts,
        api_calls: (!matches!(stats["api_calls"], Value::Null | Value::Bool(false)))
            .then(|| count(&stats["api_calls"])),
        issues_count,
        error: data
            .error
            .clone()
            .filter(|error| !campfire_richtext::ruby::is_blank(error)),
        active: data.active(),
        cancellable: data.cancellable(),
        undoable: data.undoable(),
        undo_blocked_reason: data.undo_reason.clone(),
        plan_ready: data.kind == "workspace" && data.preview(),
        catch_up: data.catch_up(),
        conversations: if personal_preview {
            data.conversations().iter().map(conversation).collect()
        } else {
            vec![]
        },
    })
}

// --- Helpers -----------------------------------------------------------------------------------

/// A run read afresh, as its status section shows it now.
async fn run_data(c: &Ctx, id: i64) -> Result<RunData> {
    let now = Timestamp::from_jiff(c.now());
    c.app()
        .db
        .read(move |conn| {
            let run = SlackImport::find(conn, id)?
                .ok_or(campfire_db::Error::RecordNotFound("SlackImport"))?;
            runs::data(conn, run, now)
        })
        .await
        .map_err(Error::internal)
}

/// A run and the classic page's notice, as a run write answers.
async fn reply_run(c: &mut Ctx, id: i64, notice: &str) -> Result {
    let run = run_wire(&run_data(c, id).await?)?;
    c.json(
        StatusCode::OK,
        &api::SlackRunChange {
            run,
            notice: notice.into(),
        },
    )
}

/// A start's answer: the new run with its notice, or the classic alert as a 422.
async fn reply_started(c: &mut Ctx, started: Started) -> Result {
    match started {
        Started::Run { run, notice } => reply_run(c, run.id, notice).await,
        Started::Refused { alert, .. } => Err(refusal(c, alert)),
    }
}

/// Optional day text as the classic form posts it: absent is blank.
fn day(value: Option<String>) -> String {
    value.unwrap_or_default()
}

/// Conversation ids as the classic `conversation_ids[]` gives them.
fn conversation_ids(ids: Vec<String>) -> Vec<Value> {
    runs::selected(Value::Array(ids.into_iter().map(Value::String).collect()))
}

async fn reply_setup(c: &mut Ctx, user: &User, notice: Option<&str>) -> Result {
    let setup = setup_wire(setup::data(c, user.id).await?)?;
    match notice {
        Some(notice) => c.json(
            StatusCode::OK,
            &api::SlackSetupChange {
                setup,
                notice: notice.into(),
            },
        ),
        None => c.json(StatusCode::OK, &setup),
    }
}

// --- Setup -------------------------------------------------------------------------------------

/// `accounts/slack_imports#show`.
async fn show_setup(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    reply_setup(c, &user, None).await
}

/// `accounts/slack_imports#update`: administrators, with the password confirmed.
async fn configure(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    require_sudo(c)?;
    let form: api::SaveSlackCredentials = body(c).await?;
    let client_id = campfire_richtext::ruby::strip(&form.client_id).to_owned();
    let secret = form
        .client_secret
        .filter(|secret| !campfire_richtext::ruby::is_blank(secret))
        .map(|secret| campfire_richtext::ruby::strip(&secret).to_owned());
    if let Err(errors) = setup::configure(c, &user, client_id, secret).await? {
        return Err(refusal(c, &errors.join(", ")));
    }
    reply_setup(c, &user, Some("Slack app credentials saved.")).await
}

/// `accounts/slack_imports#destroy`: administrators, with the password confirmed.
async fn remove(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    require_sudo(c)?;
    if !setup::remove(c, &user).await? {
        return Err(refusal(c, "Finish or cancel the running import first."));
    }
    reply_setup(c, &user, Some("Slack credentials removed.")).await
}

/// `slack/connections#destroy`: the signed-in person's own connection, with the password
/// confirmed.
async fn drop_connection(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    require_sudo(c)?;
    if !classic::disconnect_user(c, user).await? {
        return Err(refusal(
            c,
            "Finish or cancel your running Slack import first.",
        ));
    }
    c.json(
        StatusCode::OK,
        &api::SlackDisconnected {
            notice: "Slack disconnected.".into(),
        },
    )
}

// --- Workspace runs ----------------------------------------------------------------------------

/// `accounts/slack_import_runs#index`.
async fn index_runs(c: &mut Ctx) -> Result {
    administrator(c).await?;
    let runs = runs::list(c, None)
        .await?
        .iter()
        .map(row)
        .collect::<Result<_>>()?;
    c.json(StatusCode::OK, &api::SlackRunList { runs })
}

/// `accounts/slack_import_runs#create`.
async fn dry_run(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    let form: api::StartSlackDryRun = body(c).await?;
    let form = DryRunForm {
        include_private: form.include_private,
        oldest: day(form.oldest),
        latest: day(form.latest),
    };
    let started = runs::start_dry_run(c, &user, form).await?;
    reply_started(c, started).await
}

/// `accounts/slack_import_runs#show`: the run and a page of its issues.
async fn show_admin_run(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    let run = runs::find(c, &user, true).await?;
    let data = run_data(c, run.id).await?;
    let page = c.param_str("page").map(str::to_owned);
    let (id, count) = (data.id, data.issues_count);
    let (issues, next_page) = c
        .app()
        .db
        .read(move |conn| runs::issues_page(conn, id, count, page.as_deref()))
        .await
        .map_err(Error::internal)?;
    let page = api::SlackRunPage {
        run: run_wire(&data)?,
        issues: issues
            .into_iter()
            .map(|issue| api::SlackIssue {
                level: issue.level,
                slack_ref: issue
                    .slack_ref
                    .filter(|reference| !campfire_richtext::ruby::is_blank(reference)),
                message: issue.message,
            })
            .collect(),
        next_page,
    };
    c.json(StatusCode::OK, &page)
}

/// `accounts/slack_import_runs#status`.
async fn show_admin_status(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    let run = runs::find(c, &user, true).await?;
    let run = run_wire(&run_data(c, run.id).await?)?;
    c.json(StatusCode::OK, &run)
}

/// `accounts/slack_import_runs#plan`: a completed workspace dry run's plan.
async fn show_plan(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    let run = runs::find(c, &user, true).await?;
    if run.kind != "workspace" || run.mode != "dry_run" || run.status != "completed" {
        return Err(refusal(c, "The plan is ready when the dry run completes."));
    }
    let (data, rooms, oldest) = runs::plan_data(c, &user, run).await?;
    let plan = api::SlackPlan {
        run_id: data.id,
        conversations: data
            .conversations()
            .iter()
            .map(|each| api::SlackPlanConversation {
                conversation: conversation(each),
                target: target_value(each, &rooms),
            })
            .collect(),
        samples: data
            .samples()
            .iter()
            .zip(&data.sample_htmls)
            .map(|(sample, html)| api::SlackSample {
                conversation: text(&sample["conversation"]),
                slack_text: text(&sample["slack_text"]),
                html: html.clone(),
            })
            .collect(),
        rooms: rooms
            .into_iter()
            .map(|room| api::SlackRoomTarget {
                id: room.id,
                name: room.name,
            })
            .collect(),
        default_oldest: oldest,
    };
    c.json(StatusCode::OK, &plan)
}

/// `accounts/slack_import_runs#start_import`.
async fn import(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    let run = runs::find(c, &user, true).await?;
    let form: api::StartSlackImport = body(c).await?;
    let form = ImportForm {
        conversation_ids: conversation_ids(form.conversation_ids),
        room_targets: Value::Object(
            form.room_targets
                .into_iter()
                .map(|(id, target)| (id, Value::String(target)))
                .collect(),
        ),
        full: form.preset == api::SlackPreset::Full,
        oldest: day(form.oldest),
        latest: day(form.latest),
    };
    let started = runs::start_workspace_import(c, &user, run, form).await?;
    reply_started(c, started).await
}

/// `accounts/slack_import_runs#catch_up`.
async fn start_catch_up(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    let run = runs::find(c, &user, true).await?;
    let started = runs::start_catch_up(c, &user, run).await?;
    reply_started(c, started).await
}

/// `cancel` and `undo` on either page.
async fn change(c: &mut Ctx, user: User, admin: bool, undo: bool) -> Result {
    let run = runs::find(c, &user, admin).await?;
    match runs::change(c, &user, run.id, undo).await? {
        Ok(notice) => reply_run(c, run.id, notice).await,
        Err(reason) => Err(refusal(c, &reason)),
    }
}

async fn cancel_admin_run(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    change(c, user, true, false).await
}

async fn undo_admin_run(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    change(c, user, true, true).await
}

// --- Personal runs -----------------------------------------------------------------------------

/// `slack/imports#index`.
async fn show_personal(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let data = runs::personal_data(c, user.id).await?;
    let runs = runs::list(c, Some(user.id))
        .await?
        .iter()
        .map(row)
        .collect::<Result<_>>()?;
    let page = api::SlackPersonal {
        team_known: data.team_known,
        connection: connection(&data),
        connect_path: classic::connect_path(false).into(),
        runs,
    };
    c.json(StatusCode::OK, &page)
}

/// `slack/imports#create`.
async fn personal_start(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let form: api::StartPersonalSlackImport = body(c).await?;
    let form = PersonalForm {
        import: form.mode == api::SlackRunMode::Import,
        dry_run_id: form.dry_run_id.unwrap_or(0),
        conversation_ids: conversation_ids(form.conversation_ids),
    };
    let started = runs::start_personal(c, &user, form).await?;
    reply_started(c, started).await
}

/// `slack/imports#show` and `#status`: one of the person's own runs.
async fn show_personal_run(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let run = runs::find(c, &user, false).await?;
    let run = run_wire(&run_data(c, run.id).await?)?;
    c.json(StatusCode::OK, &run)
}

async fn show_personal_status(c: &mut Ctx) -> Result {
    show_personal_run(c).await
}

async fn cancel_personal_run(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    change(c, user, false, false).await
}

async fn undo_personal_run(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    change(c, user, false, true).await
}
