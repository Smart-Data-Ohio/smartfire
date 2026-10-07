//! `/api/v1/admin/slack` and `/api/v1/slack` (S7) over the seeded app with `SPA_ENABLED`. As in
//! the admin tests, each write runs on two apps frozen at the same instant, once through the
//! classic Slack import pages and once through the API, and the whole database and every
//! publication must come out the same.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::models::slack::{NewConnection, SlackConnection, SlackWorkspace};
use rusqlite::params;
use serde_json::{Value, json};

use super::admin_tests::{
    app, assert_parity, audits, classic, dump, error, get, nothing, parse, spa, write,
};
use crate::controllers::presenters::test_support::{Browser, DAVID, KEVIN, Req, TestApp};

/// The conversations a finished dry run found.
fn found() -> Value {
    json!({
        "conversations": [
            {"id": "C1", "name": "general", "type": "public_channel", "members": 4, "messages": 12, "threads": 2},
            {"id": "G2", "name": "secret", "type": "private_channel", "members": 2, "messages": 3},
            {"id": "D3", "name": "Kevin", "type": "im", "messages": 1}
        ],
        "users": {"total": 3, "matched": 2, "placeholders": 1},
        "counts": {"messages": 16}
    })
}

/// The Slack app set up by David, with or without his own Slack connection.
async fn set_up(a: &TestApp, connected: bool) {
    let crypto = rails_compat::ar_encryption::ArEncryption::new(&a.booted.app.secrets);
    a.db()
        .write(move |tx| {
            let workspace = SlackWorkspace::create(
                tx,
                &crypto,
                "fixture-client",
                "fixture-secret",
                Some(DAVID),
            )?;
            tx.conn().execute(
                "UPDATE slack_workspaces SET team_id='TFIXTURE', team_name='Fixture' WHERE id=?",
                [workspace.id],
            )?;
            if connected {
                SlackConnection::create(
                    tx,
                    &crypto,
                    NewConnection {
                        workspace_id: workspace.id,
                        user_id: DAVID,
                        slack_user_id: "UFIXTURE",
                        access_token: Some("fixture-user-grant"),
                        scopes: None,
                    },
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
}

/// A run of David's, as the importer leaves it, by id.
async fn run(a: &TestApp, kind: &str, mode: &str, status: &str, options: Value) -> i64 {
    let (kind, mode, status) = (kind.to_owned(), mode.to_owned(), status.to_owned());
    a.db()
        .write(move |tx| {
            let (workspace, connection): (i64, Option<i64>) = tx.conn().query_row(
                "SELECT w.id, c.id FROM slack_workspaces w LEFT JOIN slack_connections c ON c.slack_workspace_id = w.id",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            let finished = (status == "completed").then(|| tx.now());
            tx.conn().execute(
                "INSERT INTO slack_imports(slack_workspace_id,slack_connection_id,user_id,kind,mode,status,options,stats,started_at,finished_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
                params![workspace, connection, DAVID, kind, mode, status, options.to_string(), found().to_string(), tx.now(), finished, tx.now(), tx.now()],
            )?;
            Ok(tx.conn().last_insert_rowid())
        })
        .await
        .unwrap()
}

async fn connected(a: &TestApp, _: &mut Browser<'_>) -> Value {
    set_up(a, true).await;
    Value::Null
}

async fn a_finished_dry_run(a: &TestApp, _: &mut Browser<'_>) -> Value {
    set_up(a, true).await;
    json!(
        run(
            a,
            "workspace",
            "dry_run",
            "completed",
            json!({"include_private": true})
        )
        .await
    )
}

async fn a_running_dry_run(a: &TestApp, _: &mut Browser<'_>) -> Value {
    set_up(a, true).await;
    json!(run(a, "workspace", "dry_run", "running", json!({})).await)
}

async fn a_finished_full_import(a: &TestApp, _: &mut Browser<'_>) -> Value {
    set_up(a, true).await;
    let options = json!({
        "conversation_ids": ["C1"],
        "room_targets": {"C1": "new"},
        "include_private": true
    });
    json!(run(a, "workspace", "import", "completed", options).await)
}

async fn a_finished_preview(a: &TestApp, _: &mut Browser<'_>) -> Value {
    set_up(a, true).await;
    json!(run(a, "personal", "dry_run", "completed", json!({})).await)
}

async fn a_finished_personal_import(a: &TestApp, _: &mut Browser<'_>) -> Value {
    set_up(a, true).await;
    json!(
        run(
            a,
            "personal",
            "import",
            "completed",
            json!({"conversation_ids": ["D3"]})
        )
        .await
    )
}

async fn a_running_personal_import(a: &TestApp, _: &mut Browser<'_>) -> Value {
    set_up(a, true).await;
    json!(run(a, "personal", "import", "running", json!({})).await)
}

/// A refused write leaves nothing behind and answers the classic alert as a 422.
async fn refused(b: &mut Browser<'_>, method: Method, path: &str, body: Value, alert: &str) {
    let reply = write(b, method, path, body).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{path}: {}",
        reply.text()
    );
    let error = error(&reply);
    assert_eq!(error["_tag"], "Validation", "{path}");
    assert_eq!(error["message"], alert, "{path}");
}

/// A classic redirect's flash, read from the page it lands on.
async fn flash(b: &mut Browser<'_>, method: Method, path: &str, fields: &[(&str, &str)]) -> String {
    let reply = b.write(Req::new(method, path).form(fields)).await;
    assert!(
        reply.status.is_redirection(),
        "{path}: {}: {}",
        reply.status,
        reply.text()
    );
    let location = reply.header("location").expect("a redirect").to_owned();
    let page = b.send(Req::new(Method::GET, &location)).await;
    page.text()
}

// --- Reads and gates ---------------------------------------------------------------------------

#[tokio::test]
async fn members_reach_only_their_own_imports() {
    let Some(a) = app().await else { return };
    set_up(&a, true).await;
    let davids = run(&a, "personal", "import", "completed", json!({})).await;
    let mut kevin = a.sign_in(KEVIN).await;
    for path in [
        "/api/v1/admin/slack".to_string(),
        "/api/v1/admin/slack/runs".to_string(),
        format!("/api/v1/admin/slack/runs/{davids}"),
    ] {
        let reply = kevin.send(get(&path)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{path}");
    }
    let before = dump(&a).await;
    let reply = write(
        &mut kevin,
        Method::POST,
        "/api/v1/admin/slack/runs",
        json!({"includePrivate": true, "oldest": null, "latest": null}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(dump(&a).await, before);

    let personal: api::SlackPersonal = parse(&kevin.send(get("/api/v1/slack/imports")).await);
    assert!(personal.team_known);
    assert_eq!(personal.connection, api::SlackConnectionState::None);
    assert_eq!(
        personal.connect_path,
        "/slack/oauth/start?return_to=%2Fslack%2Fimports"
    );
    assert!(personal.runs.is_empty(), "only their own runs");
    for path in [
        format!("/api/v1/slack/imports/{davids}"),
        format!("/api/v1/slack/imports/{davids}/status"),
    ] {
        let reply = kevin.send(get(&path)).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
    }
    let reply = write(
        &mut kevin,
        Method::POST,
        &format!("/api/v1/slack/imports/{davids}/undo"),
        Value::Null,
    )
    .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(dump(&a).await, before);
}

#[tokio::test]
async fn signed_out_requests_are_unauthorized() {
    let Some(a) = app().await else { return };
    let mut nobody = a.anonymous();
    for path in ["/api/v1/admin/slack", "/api/v1/slack/imports"] {
        let reply = nobody.send(get(path)).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{path}");
    }
}

#[tokio::test]
async fn guarded_writes_change_nothing_without_the_password() {
    let Some(a) = app().await else { return };
    set_up(&a, true).await;
    let mut b = a.sign_in(DAVID).await;
    let before = dump(&a).await;
    for (method, path, body) in [
        (
            Method::PUT,
            "/api/v1/admin/slack",
            json!({"clientId": "other", "clientSecret": null}),
        ),
        (Method::DELETE, "/api/v1/admin/slack", Value::Null),
        (Method::DELETE, "/api/v1/slack/connection", Value::Null),
    ] {
        let reply = write(&mut b, method.clone(), path, body).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {path}");
        assert_eq!(error(&reply)["_tag"], "SudoRequired", "{method} {path}");
        assert_eq!(dump(&a).await, before, "{method} {path} changed something");
    }
}

#[tokio::test]
async fn the_setup_reads_as_the_classic_page() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    let blank: api::SlackSetup = parse(&david.send(get("/api/v1/admin/slack")).await);
    assert!(!blank.configured);
    assert!(!blank.team_known);
    assert_eq!(blank.client_id, None);
    assert_eq!(blank.connection, api::SlackConnectionState::None);
    assert!(
        blank.manifest.contains("display_information"),
        "{}",
        blank.manifest
    );
    assert_eq!(
        blank.connect_path,
        "/slack/oauth/start?return_to=%2Faccount%2Fslack_import"
    );

    set_up(&a, true).await;
    let id = run(&a, "workspace", "dry_run", "running", json!({})).await;
    let setup: api::SlackSetup = parse(&david.send(get("/api/v1/admin/slack")).await);
    assert!(setup.configured);
    assert!(setup.team_known);
    assert_eq!(setup.client_id.as_deref(), Some("fixture-client"));
    assert_eq!(setup.configured_by.as_deref(), Some("David"));
    assert_eq!(setup.team_name.as_deref(), Some("Fixture"));
    assert_eq!(setup.connection, api::SlackConnectionState::Connected);
    let active = setup.active_run.expect("the running dry run");
    assert_eq!(active.id, id);
    assert_eq!(active.status, api::SlackRunStatus::Running);
}

#[tokio::test]
async fn a_run_reads_as_its_classic_page() {
    let Some(a) = app().await else { return };
    set_up(&a, true).await;
    let id = run(&a, "workspace", "dry_run", "completed", json!({})).await;
    let mut david = a.sign_in(DAVID).await;
    let page: api::SlackRunPage = parse(
        &david
            .send(get(&format!("/api/v1/admin/slack/runs/{id}")))
            .await,
    );
    let run = page.run;
    assert_eq!(run.id, id);
    assert_eq!(run.kind, api::SlackRunKind::Workspace);
    assert_eq!(run.mode, api::SlackRunMode::DryRun);
    assert_eq!(run.status, api::SlackRunStatus::Completed);
    assert!(run.plan_ready);
    assert!(!run.active);
    assert!(!run.cancellable);
    assert_eq!(run.started_by, "David");
    let people = run.people.expect("the people tally");
    assert_eq!(
        (people.total, people.matched, people.placeholders),
        (3, 2, 1)
    );
    assert_eq!(run.counts.expect("the counts").messages, 16);
    assert!(page.issues.is_empty());
    assert_eq!(page.next_page, None);

    let status: api::SlackRun = parse(
        &david
            .send(get(&format!("/api/v1/admin/slack/runs/{id}/status")))
            .await,
    );
    assert_eq!(status.id, id);
    assert_eq!(status.title, run.title);

    let list: api::SlackRunList = parse(&david.send(get("/api/v1/admin/slack/runs")).await);
    assert_eq!(list.runs.iter().map(|row| row.id).collect::<Vec<_>>(), [id]);

    let plan: api::SlackPlan = parse(
        &david
            .send(get(&format!("/api/v1/admin/slack/runs/{id}/plan")))
            .await,
    );
    assert_eq!(plan.run_id, id);
    let kinds: Vec<_> = plan
        .conversations
        .iter()
        .map(|each| {
            (
                each.conversation.id.as_str(),
                each.conversation.kind.as_str(),
            )
        })
        .collect();
    assert_eq!(
        kinds,
        [
            ("C1", "Public channel"),
            ("G2", "Private channel"),
            ("D3", "Direct message")
        ]
    );
    assert!(
        !plan.rooms.is_empty(),
        "the rooms a conversation can merge into"
    );
    assert!(!plan.default_oldest.is_empty());

    let running = running_dry_run(&a).await;
    let early = david
        .send(get(&format!("/api/v1/admin/slack/runs/{running}/plan")))
        .await;
    assert_eq!(early.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        error(&early)["message"],
        "The plan is ready when the dry run completes."
    );
    let missing = david.send(get("/api/v1/admin/slack/runs/999999")).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}

/// A second dry run, still running, beside whatever the test set up.
pub(super) async fn running_dry_run(a: &TestApp) -> i64 {
    run(a, "workspace", "dry_run", "running", json!({})).await
}

#[tokio::test]
async fn a_preview_reads_with_its_conversations() {
    let Some(a) = app().await else { return };
    set_up(&a, true).await;
    let id = run(&a, "personal", "dry_run", "completed", json!({})).await;
    let mut david = a.sign_in(DAVID).await;
    let preview: api::SlackRun = parse(
        &david
            .send(get(&format!("/api/v1/slack/imports/{id}")))
            .await,
    );
    assert!(!preview.plan_ready, "a preview has no workspace plan");
    let ids: Vec<_> = preview
        .conversations
        .iter()
        .map(|each| each.id.as_str())
        .collect();
    assert_eq!(ids, ["C1", "G2", "D3"]);

    // The administrator's run page never lists a person's conversations, as classic's doesn't.
    let page: api::SlackRunPage = parse(
        &david
            .send(get(&format!("/api/v1/admin/slack/runs/{id}")))
            .await,
    );
    assert_eq!(page.run.conversations, []);
    let status: api::SlackRun = parse(
        &david
            .send(get(&format!("/api/v1/admin/slack/runs/{id}/status")))
            .await,
    );
    assert_eq!(status.conversations, []);

    let personal: api::SlackPersonal = parse(&david.send(get("/api/v1/slack/imports")).await);
    assert_eq!(personal.connection, api::SlackConnectionState::Connected);
    assert_eq!(
        personal.runs.iter().map(|row| row.id).collect::<Vec<_>>(),
        [id]
    );
}

// --- Setup writes ------------------------------------------------------------------------------

#[tokio::test]
async fn credentials_save_as_the_classic_form_does() {
    let Some(spa_side) = assert_parity(
        nothing,
        async |b, _| {
            classic(
                b,
                Method::PATCH,
                "/account/slack_import",
                &[("client_id", " 123.456 "), ("client_secret", "shh")],
            )
            .await
        },
        async |b, _| {
            let change: api::SlackSetupChange = spa(
                b,
                Method::PUT,
                "/api/v1/admin/slack",
                json!({"clientId": " 123.456 ", "clientSecret": "shh"}),
            )
            .await;
            assert_eq!(change.notice, "Slack app credentials saved.");
            assert_eq!(change.setup.client_id.as_deref(), Some("123.456"));
            assert!(change.setup.configured);
            assert_eq!(change.setup.configured_by.as_deref(), Some("David"));
        },
    )
    .await
    else {
        return;
    };
    assert!(
        audits(&spa_side).contains("slack.workspace.configure"),
        "{}",
        audits(&spa_side)
    );
}

#[tokio::test]
async fn a_blank_secret_keeps_the_saved_one_as_the_classic_form_does() {
    assert_parity(
        connected,
        async |b, _| {
            classic(
                b,
                Method::PATCH,
                "/account/slack_import",
                &[("client_id", "renamed"), ("client_secret", " ")],
            )
            .await
        },
        async |b, _| {
            let change: api::SlackSetupChange = spa(
                b,
                Method::PUT,
                "/api/v1/admin/slack",
                json!({"clientId": "renamed", "clientSecret": " "}),
            )
            .await;
            assert!(change.setup.configured, "the saved secret stays");
        },
    )
    .await;
}

#[tokio::test]
async fn a_new_secret_is_stored_as_the_classic_form_stores_it() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    david.grant_sudo().await;
    let _: api::SlackSetupChange = spa(
        &mut david,
        Method::PUT,
        "/api/v1/admin/slack",
        json!({"clientId": "123.456", "clientSecret": " shh "}),
    )
    .await;
    let crypto = rails_compat::ar_encryption::ArEncryption::new(&a.booted.app.secrets);
    let secret = a
        .db()
        .read(move |conn| {
            SlackWorkspace::current(conn)?
                .expect("saved")
                .client_secret(&crypto)
        })
        .await
        .unwrap();
    assert_eq!(secret.as_deref(), Some("shh"));
}

#[tokio::test]
async fn blank_credentials_are_refused_as_the_classic_form_refuses_them() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    david.grant_sudo().await;
    let before = dump(&a).await;
    let page = david
        .write(Req::new(Method::PATCH, "/account/slack_import").form(&[("client_id", " ")]))
        .await;
    assert_eq!(page.status, StatusCode::UNPROCESSABLE_ENTITY);
    let text = page.text();
    let reply = write(
        &mut david,
        Method::PUT,
        "/api/v1/admin/slack",
        json!({"clientId": " ", "clientSecret": null}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    let message = error(&reply)["message"].as_str().unwrap().to_owned();
    for each in message.split(", ") {
        assert!(
            text.contains(&askama_escape(each)),
            "{each} is on the classic page"
        );
    }
    assert!(message.contains("can't be blank"), "{message}");
    assert_eq!(dump(&a).await, before);
}

#[tokio::test]
async fn credentials_go_as_the_classic_page_removes_them() {
    let Some(spa_side) = assert_parity(
        connected,
        async |b, _| classic(b, Method::DELETE, "/account/slack_import", &[]).await,
        async |b, _| {
            let change: api::SlackSetupChange =
                spa(b, Method::DELETE, "/api/v1/admin/slack", Value::Null).await;
            assert_eq!(change.notice, "Slack credentials removed.");
            assert!(!change.setup.configured);
            assert_eq!(change.setup.connection, api::SlackConnectionState::None);
        },
    )
    .await
    else {
        return;
    };
    assert!(
        audits(&spa_side).contains("slack.workspace.remove_credentials"),
        "{}",
        audits(&spa_side)
    );
}

#[tokio::test]
async fn credentials_stay_while_a_run_is_active_as_the_classic_page_keeps_them() {
    assert_parity(
        a_running_dry_run,
        async |b, _| {
            let text = flash(b, Method::DELETE, "/account/slack_import", &[]).await;
            assert!(text.contains("Finish or cancel the running import first."));
        },
        async |b, _| {
            refused(
                b,
                Method::DELETE,
                "/api/v1/admin/slack",
                Value::Null,
                "Finish or cancel the running import first.",
            )
            .await
        },
    )
    .await;
}

#[tokio::test]
async fn disconnecting_matches_the_classic_button() {
    let Some(spa_side) = assert_parity(
        connected,
        async |b, _| classic(b, Method::DELETE, "/slack/connection", &[]).await,
        async |b, _| {
            let done: api::SlackDisconnected =
                spa(b, Method::DELETE, "/api/v1/slack/connection", Value::Null).await;
            assert_eq!(done.notice, "Slack disconnected.");
        },
    )
    .await
    else {
        return;
    };
    assert_eq!(spa_side.rows["slack_connections"], json!([]));
}

#[tokio::test]
async fn disconnecting_waits_for_a_running_import_as_the_classic_button_does() {
    assert_parity(
        a_running_personal_import,
        async |b, _| {
            let text = flash(b, Method::DELETE, "/slack/connection", &[]).await;
            assert!(text.contains("Finish or cancel your running Slack import first."));
        },
        async |b, _| {
            refused(
                b,
                Method::DELETE,
                "/api/v1/slack/connection",
                Value::Null,
                "Finish or cancel your running Slack import first.",
            )
            .await
        },
    )
    .await;
}

// --- Workspace runs ----------------------------------------------------------------------------

#[tokio::test]
async fn a_dry_run_starts_as_the_classic_form_does() {
    let Some(spa_side) = assert_parity(
        connected,
        async |b, _| {
            classic(
                b,
                Method::POST,
                "/account/slack_import/runs",
                &[
                    ("include_private", "0"),
                    ("oldest", "2026-01-02"),
                    ("latest", "2026-01-03"),
                ],
            )
            .await
        },
        async |b, _| {
            let change: api::SlackRunChange = spa(
                b,
                Method::POST,
                "/api/v1/admin/slack/runs",
                json!({"includePrivate": false, "oldest": "2026-01-02", "latest": "2026-01-03"}),
            )
            .await;
            assert_eq!(change.notice, "Dry run started.");
            assert_eq!(change.run.mode, api::SlackRunMode::DryRun);
            // The job runner may already have picked the run up, so its status isn't pinned.
            assert_eq!(change.run.kind, api::SlackRunKind::Workspace);
        },
    )
    .await
    else {
        return;
    };
    assert!(
        audits(&spa_side).contains("slack.import.start"),
        "{}",
        audits(&spa_side)
    );
    assert_ne!(spa_side.rows["slack_imports"], json!([]), "the new run");
}

#[tokio::test]
async fn a_dry_run_with_no_dates_starts_as_the_classic_form_does() {
    assert_parity(
        connected,
        async |b, _| classic(b, Method::POST, "/account/slack_import/runs", &[]).await,
        async |b, _| {
            let _: api::SlackRunChange = spa(
                b,
                Method::POST,
                "/api/v1/admin/slack/runs",
                json!({"includePrivate": true, "oldest": null, "latest": null}),
            )
            .await;
        },
    )
    .await;
}

#[tokio::test]
async fn a_dry_run_waits_for_another_run_as_the_classic_form_does() {
    assert_parity(
        a_running_dry_run,
        async |b, _| {
            let text = flash(b, Method::POST, "/account/slack_import/runs", &[]).await;
            assert!(text.contains("Another import is already running. Wait for it to finish."));
        },
        async |b, _| {
            refused(
                b,
                Method::POST,
                "/api/v1/admin/slack/runs",
                json!({"includePrivate": true, "oldest": null, "latest": null}),
                "Another import is already running. Wait for it to finish.",
            )
            .await
        },
    )
    .await;
}

#[tokio::test]
async fn a_dry_run_needs_a_connection_as_the_classic_form_does() {
    async fn unconnected(a: &TestApp, _: &mut Browser<'_>) -> Value {
        set_up(a, false).await;
        Value::Null
    }
    assert_parity(
        unconnected,
        async |b, _| {
            let text = flash(b, Method::POST, "/account/slack_import/runs", &[]).await;
            assert!(text.contains("Connect your Slack account first."));
        },
        async |b, _| {
            refused(
                b,
                Method::POST,
                "/api/v1/admin/slack/runs",
                json!({"includePrivate": true, "oldest": null, "latest": null}),
                "Connect your Slack account first.",
            )
            .await
        },
    )
    .await;
}

/// A room the plan offers, from the plan (reading it writes nothing).
async fn a_plan_room(b: &mut Browser<'_>, run: &Value) -> String {
    let plan: api::SlackPlan = parse(
        &b.send(get(&format!("/api/v1/admin/slack/runs/{run}/plan")))
            .await,
    );
    plan.rooms.first().expect("a room").id.to_string()
}

#[tokio::test]
async fn a_test_import_starts_as_the_classic_form_does() {
    let Some(spa_side) = assert_parity(
        a_finished_dry_run,
        async |b, run| {
            let room = a_plan_room(b, &run).await;
            classic(
                b,
                Method::POST,
                &format!("/account/slack_import/runs/{run}/import"),
                &[
                    ("preset", "test"),
                    ("conversation_ids[]", "C1"),
                    ("conversation_ids[]", "G2"),
                    ("room_targets[C1]", &room),
                    ("room_targets[G2]", "new"),
                    ("room_targets[D3]", "skip"),
                    ("oldest", "2026-01-05"),
                    ("latest", "2026-01-06"),
                ],
            )
            .await
        },
        async |b, run| {
            let room = a_plan_room(b, &run).await;
            let change: api::SlackRunChange = spa(
                b,
                Method::POST,
                &format!("/api/v1/admin/slack/runs/{run}/import"),
                json!({
                    "conversationIds": ["C1", "G2"],
                    "roomTargets": {"C1": room, "G2": "new", "D3": "skip"},
                    "preset": "test",
                    "oldest": "2026-01-05",
                    "latest": "2026-01-06"
                }),
            )
            .await;
            assert_eq!(change.notice, "Test import started.");
            assert_eq!(change.run.mode, api::SlackRunMode::Import);
        },
    )
    .await
    else {
        return;
    };
    assert!(
        audits(&spa_side)
            .replace('\\', "")
            .contains("\"preset\":\"test\""),
        "{}",
        audits(&spa_side)
    );
}

#[tokio::test]
async fn a_full_import_starts_as_the_classic_form_does() {
    assert_parity(
        a_finished_dry_run,
        async |b, run| {
            classic(
                b,
                Method::POST,
                &format!("/account/slack_import/runs/{run}/import"),
                &[
                    ("preset", "full"),
                    ("conversation_ids[]", "C1"),
                    ("conversation_ids[]", "D3"),
                    ("room_targets[C1]", "new"),
                    ("room_targets[D3]", "new"),
                    ("oldest", "2026-01-05"),
                ],
            )
            .await
        },
        async |b, run| {
            let change: api::SlackRunChange = spa(
                b,
                Method::POST,
                &format!("/api/v1/admin/slack/runs/{run}/import"),
                json!({
                    "conversationIds": ["C1", "D3"],
                    "roomTargets": {"C1": "new", "D3": "new"},
                    "preset": "full",
                    "oldest": "2026-01-05",
                    "latest": null
                }),
            )
            .await;
            assert_eq!(change.notice, "Full import started.");
        },
    )
    .await;
}

#[tokio::test]
async fn an_import_needs_a_checked_conversation_as_the_classic_form_does() {
    assert_parity(
        a_finished_dry_run,
        async |b, run| {
            let text = flash(
                b,
                Method::POST,
                &format!("/account/slack_import/runs/{run}/import"),
                &[("preset", "full")],
            )
            .await;
            assert!(text.contains("Check at least one conversation to import."));
        },
        async |b, run| {
            refused(
                b,
                Method::POST,
                &format!("/api/v1/admin/slack/runs/{run}/import"),
                json!({"conversationIds": [], "roomTargets": {}, "preset": "full", "oldest": null, "latest": null}),
                "Check at least one conversation to import.",
            )
            .await
        },
    )
    .await;
}

#[tokio::test]
async fn an_import_takes_only_the_dry_runs_conversations_as_the_classic_form_does() {
    assert_parity(
        a_finished_dry_run,
        async |b, run| {
            let text = flash(
                b,
                Method::POST,
                &format!("/account/slack_import/runs/{run}/import"),
                &[("conversation_ids[]", "NOPE")],
            )
            .await;
            assert!(text.contains("Those conversations are not in the dry run."));
        },
        async |b, run| {
            refused(
                b,
                Method::POST,
                &format!("/api/v1/admin/slack/runs/{run}/import"),
                json!({"conversationIds": ["NOPE"], "roomTargets": {}, "preset": "test", "oldest": null, "latest": null}),
                "Those conversations are not in the dry run.",
            )
            .await
        },
    )
    .await;
}

#[tokio::test]
async fn a_catch_up_starts_as_the_classic_button_does() {
    let Some(spa_side) = assert_parity(
        a_finished_full_import,
        async |b, run| {
            classic(
                b,
                Method::POST,
                &format!("/account/slack_import/runs/{run}/catch_up"),
                &[],
            )
            .await
        },
        async |b, run| {
            let change: api::SlackRunChange = spa(
                b,
                Method::POST,
                &format!("/api/v1/admin/slack/runs/{run}/catch_up"),
                Value::Null,
            )
            .await;
            assert_eq!(change.notice, "Catch-up import started.");
        },
    )
    .await
    else {
        return;
    };
    assert!(
        audits(&spa_side).contains("catch_up"),
        "{}",
        audits(&spa_side)
    );
}

#[tokio::test]
async fn a_catch_up_needs_a_full_import_as_the_classic_button_does() {
    assert_parity(
        a_finished_dry_run,
        async |b, run| {
            let text = flash(
                b,
                Method::POST,
                &format!("/account/slack_import/runs/{run}/catch_up"),
                &[],
            )
            .await;
            assert!(text.contains("Catch-up starts from a completed full import."));
        },
        async |b, run| {
            refused(
                b,
                Method::POST,
                &format!("/api/v1/admin/slack/runs/{run}/catch_up"),
                Value::Null,
                "Catch-up starts from a completed full import.",
            )
            .await
        },
    )
    .await;
}

#[tokio::test]
async fn a_run_cancels_as_the_classic_button_cancels_it() {
    let Some(spa_side) = assert_parity(
        a_running_dry_run,
        async |b, run| {
            classic(
                b,
                Method::POST,
                &format!("/account/slack_import/runs/{run}/cancel"),
                &[],
            )
            .await
        },
        async |b, run| {
            let change: api::SlackRunChange = spa(
                b,
                Method::POST,
                &format!("/api/v1/admin/slack/runs/{run}/cancel"),
                Value::Null,
            )
            .await;
            assert_eq!(change.notice, "Import cancelled.");
            assert_eq!(change.run.status, api::SlackRunStatus::Cancelled);
        },
    )
    .await
    else {
        return;
    };
    assert!(
        audits(&spa_side).contains("slack.import.cancel"),
        "{}",
        audits(&spa_side)
    );
}

#[tokio::test]
async fn a_finished_run_cannot_cancel_as_the_classic_button_says() {
    assert_parity(
        a_finished_dry_run,
        async |b, run| {
            let text = flash(
                b,
                Method::POST,
                &format!("/account/slack_import/runs/{run}/cancel"),
                &[],
            )
            .await;
            assert!(text.contains("That run already finished."));
        },
        async |b, run| {
            refused(
                b,
                Method::POST,
                &format!("/api/v1/admin/slack/runs/{run}/cancel"),
                Value::Null,
                "That run already finished.",
            )
            .await
        },
    )
    .await;
}

#[tokio::test]
async fn an_import_undoes_as_the_classic_button_undoes_it() {
    let Some(spa_side) = assert_parity(
        a_finished_full_import,
        async |b, run| {
            classic(
                b,
                Method::POST,
                &format!("/account/slack_import/runs/{run}/undo"),
                &[],
            )
            .await
        },
        async |b, run| {
            let change: api::SlackRunChange = spa(
                b,
                Method::POST,
                &format!("/api/v1/admin/slack/runs/{run}/undo"),
                Value::Null,
            )
            .await;
            assert_eq!(change.notice, "Undo started.");
            assert_eq!(change.run.status, api::SlackRunStatus::Undoing);
        },
    )
    .await
    else {
        return;
    };
    assert!(
        audits(&spa_side).contains("slack.import.undo"),
        "{}",
        audits(&spa_side)
    );
}

#[tokio::test]
async fn a_dry_run_cannot_undo_as_the_classic_button_says() {
    let Some(a) = app().await else { return };
    set_up(&a, true).await;
    let id = run(&a, "workspace", "dry_run", "completed", json!({})).await;
    let mut david = a.sign_in(DAVID).await;
    let page: api::SlackRunPage = parse(
        &david
            .send(get(&format!("/api/v1/admin/slack/runs/{id}")))
            .await,
    );
    assert!(!page.run.undoable);
    let before = dump(&a).await;
    let text = flash(
        &mut david,
        Method::POST,
        &format!("/account/slack_import/runs/{id}/undo"),
        &[],
    )
    .await;
    let reply = write(
        &mut david,
        Method::POST,
        &format!("/api/v1/admin/slack/runs/{id}/undo"),
        Value::Null,
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    let message = error(&reply)["message"].as_str().unwrap().to_owned();
    assert!(
        text.contains(&askama_escape(&message)),
        "{message} is the classic alert"
    );
    assert_eq!(dump(&a).await, before);
}

/// Text as the classic page escapes it (as Rails does).
fn askama_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// --- Personal runs -----------------------------------------------------------------------------

#[tokio::test]
async fn a_preview_starts_as_the_classic_form_does() {
    let Some(spa_side) = assert_parity(
        connected,
        async |b, _| classic(b, Method::POST, "/slack/imports", &[]).await,
        async |b, _| {
            let change: api::SlackRunChange = spa(
                b,
                Method::POST,
                "/api/v1/slack/imports",
                json!({"mode": "dry_run", "dryRunId": null, "conversationIds": []}),
            )
            .await;
            assert_eq!(change.notice, "Preview started.");
            assert_eq!(change.run.kind, api::SlackRunKind::Personal);
        },
    )
    .await
    else {
        return;
    };
    assert_ne!(spa_side.rows["slack_imports"], json!([]), "the new run");
}

#[tokio::test]
async fn a_personal_import_starts_from_the_preview_as_the_classic_form_does() {
    assert_parity(
        a_finished_preview,
        async |b, run| {
            let run = run.to_string();
            classic(
                b,
                Method::POST,
                "/slack/imports",
                &[
                    ("mode", "import"),
                    ("dry_run_id", &run),
                    ("conversation_ids[]", "D3"),
                    ("conversation_ids[]", "C1"),
                ],
            )
            .await
        },
        async |b, run| {
            let change: api::SlackRunChange = spa(
                b,
                Method::POST,
                "/api/v1/slack/imports",
                json!({"mode": "import", "dryRunId": run, "conversationIds": ["D3", "C1"]}),
            )
            .await;
            assert_eq!(change.notice, "Import started.");
            assert_eq!(change.run.mode, api::SlackRunMode::Import);
        },
    )
    .await;
}

#[tokio::test]
async fn a_personal_import_needs_a_preview_as_the_classic_form_does() {
    assert_parity(
        connected,
        async |b, _| {
            let text = flash(
                b,
                Method::POST,
                "/slack/imports",
                &[("mode", "import"), ("conversation_ids[]", "D3")],
            )
            .await;
            assert!(text.contains("Run a preview first."));
        },
        async |b, _| {
            refused(
                b,
                Method::POST,
                "/api/v1/slack/imports",
                json!({"mode": "import", "dryRunId": null, "conversationIds": ["D3"]}),
                "Run a preview first.",
            )
            .await
        },
    )
    .await;
}

#[tokio::test]
async fn a_personal_import_waits_for_your_own_as_the_classic_form_does() {
    assert_parity(
        a_running_personal_import,
        async |b, _| {
            let text = flash(b, Method::POST, "/slack/imports", &[]).await;
            assert!(text.contains("You already have an import running. Wait for it to finish."));
        },
        async |b, _| {
            refused(
                b,
                Method::POST,
                "/api/v1/slack/imports",
                json!({"mode": "dry_run", "dryRunId": null, "conversationIds": []}),
                "You already have an import running. Wait for it to finish.",
            )
            .await
        },
    )
    .await;
}

#[tokio::test]
async fn a_personal_import_cancels_as_the_classic_button_cancels_it() {
    assert_parity(
        a_running_personal_import,
        async |b, run| {
            classic(
                b,
                Method::POST,
                &format!("/slack/imports/{run}/cancel"),
                &[],
            )
            .await
        },
        async |b, run| {
            let change: api::SlackRunChange = spa(
                b,
                Method::POST,
                &format!("/api/v1/slack/imports/{run}/cancel"),
                Value::Null,
            )
            .await;
            assert_eq!(change.notice, "Import cancelled.");
        },
    )
    .await;
}

#[tokio::test]
async fn a_personal_import_undoes_as_the_classic_button_undoes_it() {
    assert_parity(
        a_finished_personal_import,
        async |b, run| classic(b, Method::POST, &format!("/slack/imports/{run}/undo"), &[]).await,
        async |b, run| {
            let change: api::SlackRunChange = spa(
                b,
                Method::POST,
                &format!("/api/v1/slack/imports/{run}/undo"),
                Value::Null,
            )
            .await;
            assert_eq!(change.notice, "Undo started.");
        },
    )
    .await;
}

/// Someone who uses the new UI lands on the SPA's Slack pages from the classic URLs, while the
/// classic forms still post and redirect as before. A page a classic write left a notice for
/// stays classic once so the notice shows (so does Slack's OAuth return, "Slack connected."); the
/// next visit goes to the SPA. Someone who hasn't opted in keeps the classic pages.
#[tokio::test]
async fn the_slack_pages_redirect_but_their_forms_stay_classic() {
    use campfire_db::models::user::ui_preference::{self, UiPreference};

    let Some(a) = app().await else { return };
    set_up(&a, true).await;
    let dry_run = run(&a, "workspace", "dry_run", "completed", json!({})).await;
    let preview = run(&a, "personal", "dry_run", "completed", json!({})).await;
    a.db()
        .write(|tx| ui_preference::store(tx, DAVID, UiPreference::Next))
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    for (classic, spa) in [
        (
            "/account/slack_import".to_string(),
            "/app/admin/slack".to_string(),
        ),
        (
            "/account/slack_import/runs".into(),
            "/app/admin/slack/runs".into(),
        ),
        (
            format!("/account/slack_import/runs/{dry_run}"),
            format!("/app/admin/slack/runs/{dry_run}"),
        ),
        (
            format!("/account/slack_import/runs/{dry_run}/plan"),
            format!("/app/admin/slack/runs/{dry_run}/plan"),
        ),
        ("/slack/imports".into(), "/app/settings/slack".into()),
        (
            format!("/slack/imports/{preview}"),
            format!("/app/settings/slack/{preview}"),
        ),
    ] {
        let reply = david.get(&classic).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{classic}");
        assert_eq!(
            reply.location(),
            Some(format!("http://campfire.test{spa}").as_str()),
            "{classic}"
        );
    }

    let kept = david
        .get(&format!("/account/slack_import/runs/{dry_run}?classic=1"))
        .await;
    assert_eq!(kept.status, StatusCode::OK);

    // The classic form's start runs as before and answers with the classic redirect.
    let save = david
        .write(
            Req::new(Method::POST, "/account/slack_import/runs")
                .header("accept", "text/html")
                .form(&[("include_private", "1")]),
        )
        .await;
    assert!(save.status.is_redirection(), "{:?}", save.status);
    let location = save.location().expect("the classic redirect").to_owned();
    assert!(!location.contains("/app/"), "{location}");

    // The page the notice waits for shows it, classic; the next visit goes to the SPA.
    let shown = david.get(&location).await;
    assert_eq!(shown.status, StatusCode::OK, "{:?}", shown.location());
    assert!(shown.text().contains("Dry run started."));
    let again = david.get(&location).await;
    assert_eq!(again.status, StatusCode::FOUND);
    assert!(
        again
            .location()
            .is_some_and(|spa| spa.starts_with("http://campfire.test/app/admin/slack/runs/")),
        "{:?}",
        again.location()
    );

    // A HEAD navigation redirects as a GET does.
    let head = david
        .send(Req::new(Method::HEAD, "/slack/imports").header("accept", "text/html"))
        .await;
    assert_eq!(head.status, StatusCode::FOUND);
    assert_eq!(
        head.location(),
        Some("http://campfire.test/app/settings/slack")
    );

    // Kevin hasn't opted in: the classic page.
    let mut kevin = a.sign_in(KEVIN).await;
    assert_eq!(kevin.get("/slack/imports").await.status, StatusCode::OK);
}

// --- Undo blocked, credentials of agents, racing starts ---------------------------------------

/// A finished workspace import of `C1` by `user`, started at `started` (database time text).
async fn an_import_of_c1(a: &TestApp, user: i64, started: &str, state: Value) -> i64 {
    let started = started.to_owned();
    a.db()
        .write(move |tx| {
            let workspace: i64 =
                tx.conn()
                    .query_row("SELECT id FROM slack_workspaces", [], |row| row.get(0))?;
            let stats = json!({"conversations": [
                {"id": "C1", "name": "general", "type": "public_channel", "target": {"action": "create"}}
            ]});
            tx.conn().execute(
                "INSERT INTO slack_imports(slack_workspace_id,user_id,kind,mode,status,options,stats,state,started_at,finished_at,created_at,updated_at) VALUES(?,?,'workspace','import','completed',?,?,?,?,?,?,?)",
                params![workspace, user, json!({"conversation_ids": ["C1"]}).to_string(), stats.to_string(), state.to_string(), started, started, started, started],
            )?;
            Ok(tx.conn().last_insert_rowid())
        })
        .await
        .unwrap()
}

/// Why the run can't be undone, as its page says, and the undo the API refuses with it.
async fn blocked(b: &mut Browser<'_>, id: i64) -> String {
    let run: api::SlackRun = parse(
        &b.send(get(&format!("/api/v1/admin/slack/runs/{id}/status")))
            .await,
    );
    assert!(!run.undoable, "a blocked undo isn't offered");
    let reason = run.undo_blocked_reason.expect("a reason");
    let reply = write(
        b,
        Method::POST,
        &format!("/api/v1/admin/slack/runs/{id}/undo"),
        Value::Null,
    )
    .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert_eq!(error(&reply)["_tag"], "Validation");
    reason
}

#[tokio::test]
async fn an_undo_waits_for_a_later_import_of_the_same_conversations() {
    let Some(a) = app().await else { return };
    set_up(&a, true).await;
    let first = an_import_of_c1(&a, DAVID, "2026-01-01 10:00:00", json!({})).await;
    let later = an_import_of_c1(&a, DAVID, "2026-01-02 10:00:00", json!({})).await;
    let mut david = a.sign_in(DAVID).await;
    assert_eq!(
        blocked(&mut david, first).await,
        format!(
            "A later import (#{later}) also imported some of these conversations; undo that one first."
        )
    );
}

#[tokio::test]
async fn an_undo_names_who_ran_the_later_import() {
    let Some(a) = app().await else { return };
    set_up(&a, true).await;
    let first = an_import_of_c1(&a, DAVID, "2026-01-01 10:00:00", json!({})).await;
    an_import_of_c1(&a, KEVIN, "2026-01-02 10:00:00", json!({})).await;
    let kevin: String = a
        .db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT name FROM users WHERE id = ?", [KEVIN], |row| {
                    row.get(0)
                })?,
            )
        })
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    assert_eq!(
        blocked(&mut david, first).await,
        format!(
            "A later import by {kevin} also imported some of these conversations. It has to be undone first; ask them or an administrator."
        )
    );
}

#[tokio::test]
async fn an_undo_waits_for_the_import_to_finish_its_last_step() {
    let Some(a) = app().await else { return };
    set_up(&a, true).await;
    let stamp: String = a
        .db()
        .write(|tx| Ok(campfire_db::models::slack_import::lease_stamp(tx.now())))
        .await
        .unwrap();
    let id = an_import_of_c1(
        &a,
        DAVID,
        "2026-01-01 10:00:00",
        json!({"step_started_at": stamp}),
    )
    .await;
    let mut david = a.sign_in(DAVID).await;
    assert_eq!(
        blocked(&mut david, id).await,
        "This import is still finishing. Wait for it to finish, then undo."
    );
}

#[tokio::test]
async fn an_undo_waits_for_a_queued_run() {
    let Some(a) = app().await else { return };
    set_up(&a, true).await;
    let id = an_import_of_c1(&a, DAVID, "2026-01-01 10:00:00", json!({})).await;
    run(&a, "workspace", "dry_run", "queued", json!({})).await;
    let mut david = a.sign_in(DAVID).await;
    assert_eq!(
        blocked(&mut david, id).await,
        "Another import is queued or running. Wait for it to finish, then undo."
    );
}

#[tokio::test]
async fn bot_keys_and_agent_tokens_are_refused() {
    use crate::controllers::agent_http_tests::{SECRET, initialize};
    use crate::controllers::presenters::test_support::BENDER_KEY;

    let Some(a) = app().await else { return };
    set_up(&a, true).await;
    initialize(&a).await;
    let id = run(&a, "personal", "dry_run", "completed", json!({})).await;
    let token = |method: Method, path: &str| {
        Req::new(method, path)
            .header("accept", "application/json")
            .header("content-type", "application/json")
            .header("authorization", &format!("Bearer {SECRET}"))
    };
    // Authenticating stamps the credential's last use; the snapshot starts after that.
    a.anonymous()
        .send(token(Method::GET, "/api/v1/slack/imports"))
        .await;
    let before = dump(&a).await;
    let paths = [
        (Method::GET, "/api/v1/admin/slack".to_string()),
        (Method::GET, "/api/v1/admin/slack/runs".to_string()),
        (Method::GET, format!("/api/v1/admin/slack/runs/{id}/status")),
        (Method::POST, "/api/v1/admin/slack/runs".to_string()),
        (Method::GET, "/api/v1/slack/imports".to_string()),
        (Method::GET, format!("/api/v1/slack/imports/{id}")),
        (Method::POST, "/api/v1/slack/imports".to_string()),
        (Method::DELETE, "/api/v1/slack/connection".to_string()),
    ];
    for (method, path) in paths {
        let keyed = a
            .anonymous()
            .send(
                Req::new(method.clone(), &format!("{path}?bot_key={BENDER_KEY}"))
                    .header("accept", "application/json"),
            )
            .await;
        assert_eq!(
            keyed.status,
            StatusCode::FORBIDDEN,
            "bot key {method} {path}"
        );
        let bearer = a.anonymous().send(token(method.clone(), &path)).await;
        assert_eq!(
            bearer.status,
            StatusCode::FORBIDDEN,
            "agent token {method} {path}"
        );
    }
    assert_eq!(dump(&a).await, before, "nothing changed");
}

#[tokio::test]
async fn two_racing_starts_begin_one_run() {
    let Some(a) = app().await else { return };
    set_up(&a, true).await;
    let mut first = a.sign_in(DAVID).await;
    let mut second = a.sign_in(DAVID).await;
    let body = json!({"includePrivate": true, "oldest": null, "latest": null});
    let (one, two) = tokio::join!(
        write(
            &mut first,
            Method::POST,
            "/api/v1/admin/slack/runs",
            body.clone()
        ),
        write(
            &mut second,
            Method::POST,
            "/api/v1/admin/slack/runs",
            body.clone()
        ),
    );
    let mut statuses = [one.status, two.status];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::UNPROCESSABLE_ENTITY],
        "{} / {}",
        one.text(),
        two.text()
    );
    let refusal = if one.status == StatusCode::OK {
        &two
    } else {
        &one
    };
    assert_eq!(
        error(refusal)["message"],
        "Another import is already running. Wait for it to finish."
    );
    let runs: i64 = a
        .db()
        .read(
            |conn| Ok(conn.query_row("SELECT COUNT(*) FROM slack_imports", [], |row| row.get(0))?),
        )
        .await
        .unwrap();
    assert_eq!(runs, 1);
}
