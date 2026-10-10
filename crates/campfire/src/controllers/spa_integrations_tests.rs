//! Personal integration writes run through both transports from the same frozen seed. Reuse
//! the admin suite's whole-database, audit and all-publication comparison, with the classic
//! controllers' GitHub TLS fake, inherited Fizzy HTTP listener and recorded Google client.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use rails_compat::ar_encryption::ArEncryption;
use serde_json::{Value, json};

use super::admin_tests::{
    app, assert_parity, assert_parity_with_app, audits, dump, error, get, json_body, parse, settle,
    write,
};
use crate::app::google_api_tests as google;
use crate::controllers::presenters::test_support::{
    BENDER_KEY, Browser, DAVID, JASON, KEVIN, Req, TestApp,
};
use crate::integrations::{
    fizzy::{accounts, cards},
    github::{accounts as github_accounts, tests::fake},
    test_support::{FakeServer, Route, ws15e_http_case, ws15e_http_case_listener},
};

const TOKEN: &str = "  new-personal-token  ";
const GITHUB_COLLISION: &str = "GitHub connected as octocat. Another member's linked GitHub account already uses that username, so your profile username was left unchanged.";

/// Remove connections carried by the seed so each branch specifies its complete start state.
async fn clear(a: &TestApp) {
    a.db()
        .write(|tx| {
            tx.conn().execute_batch(
                "DELETE FROM github_connected_accounts; DELETE FROM fizzy_connected_accounts;
                 DELETE FROM google_accounts; DELETE FROM calendar_push_channels;
                 DELETE FROM calendar_meeting_caches; DELETE FROM event_calendar_entries;
                 DELETE FROM background_jobs; DELETE FROM audit_logs;",
            )?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn classic_change(
    b: &mut Browser<'_>,
    method: Method,
    service: &str,
    token: &str,
    notice: bool,
    message: &str,
) {
    let reply = b
        .write(Req::new(method, &format!("/{service}/connection")).form(&[("access_token", token)]))
        .await;
    assert_eq!(reply.status, StatusCode::FOUND, "{}", reply.text());
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/users/me/profile")
    );
    assert_eq!(
        b.flash(),
        json!({if notice { "notice" } else { "alert" }: message})
    );
}

async fn api_change(
    b: &mut Browser<'_>,
    method: Method,
    service: &str,
    token: &str,
    notice: bool,
    message: &str,
) -> Option<api::IntegrationSettings> {
    let path = format!("/api/v1/settings/{service}_connection");
    let reply = if method == Method::PUT {
        write(b, method, &path, json!({"accessToken": token})).await
    } else {
        b.write(Req::new(method, &path).header("accept", "application/json"))
            .await
    };
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    if !notice {
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert_eq!(
            error(&reply),
            json!({"_tag": "Validation", "message": message, "fields": {}})
        );
        return None;
    }
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let changed: api::IntegrationChange = parse(&reply);
    assert_eq!(changed.notice, message);
    assert_eq!(
        serde_json::from_slice::<Value>(&reply.body).unwrap(),
        json!({"integrations": changed.integrations, "notice": message})
    );
    let settings: api::Settings = parse(&b.send(get("/api/v1/settings")).await);
    assert_eq!(
        changed.integrations, settings.integrations,
        "same settings reader"
    );
    assert_eq!(changed.integrations.manage_path, "/users/me/profile");
    assert_eq!(changed.integrations.slack_import_path, "/slack/imports");
    assert!(!reply.text().contains("personal-token"));
    Some(changed.integrations)
}

async fn github_preset(a: &TestApp, user: i64, source: &str, rejected: bool) {
    let crypto = ArEncryption::new(&a.booted.app.secrets);
    let source = source.to_owned();
    a.db()
        .write(move |tx| {
            let account = github_accounts::Account::create(
                tx,
                &crypto,
                &github_accounts::AccountInput {
                    user_id: user,
                    github_login: if user == JASON {
                        "octocat"
                    } else {
                        "old-login"
                    },
                    access_token: "old-token",
                    refresh_token: (source == "app").then_some("old-refresh"),
                    token_expires_at: (source == "app")
                        .then(|| tx.now().since(jiff::SignedDuration::from_hours(1))),
                    token_source: &source,
                },
            )?;
            if rejected {
                github_accounts::Account::mark_disconnected(tx, account.id, "Token rejected")?;
            }
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn github_connections_match_every_notice_and_alert() {
    for case in [
        "blank",
        "rejected",
        "refused",
        "unreachable",
        "connect",
        "reconnect",
        "collision",
        "disconnect",
        "disconnect_app",
        "missing",
    ] {
        let status = match case {
            "rejected" => 401,
            "refused" => 403,
            "unreachable" => 500,
            _ => 200,
        };
        let (server, network) = fake(vec![
            Route::new("GET", "api.github.com", "/user", status).body(r#"{"login":"octocat"}"#),
            Route::new(
                "DELETE",
                "api.github.com",
                "/applications/fixture-client/token",
                204,
            ),
            Route::new(
                "DELETE",
                "api.github.com",
                "/applications/fixture-client/grant",
                204,
            ),
        ])
        .await;
        let disconnect = matches!(case, "disconnect" | "disconnect_app" | "missing");
        let token = if case == "blank" { " \t\n " } else { TOKEN };
        let (notice, message) = match case {
            "blank" => (false, "Paste a token to connect GitHub."),
            "rejected" => (false, "GitHub rejected that token. Check it and try again."),
            "refused" | "unreachable" => (false, "Could not reach GitHub. Try again."),
            "collision" => (true, GITHUB_COLLISION),
            _ if disconnect => (true, "GitHub disconnected."),
            _ => (true, "GitHub connected as octocat."),
        };
        let boot = async || {
            let mut a =
                TestApp::boot_with_github_network_and_env(network.clone(), &[("SPA_ENABLED", "1")])
                    .await?;
            a.booted.jobs.stop(Duration::from_secs(1)).await;
            Some(a)
        };
        let Some(outcome) = assert_parity_with_app(
            boot,
            async |a, _| {
                clear(a).await;
                if matches!(case, "reconnect" | "disconnect" | "disconnect_app") {
                    github_preset(
                        a,
                        DAVID,
                        if case == "disconnect_app" {
                            "app"
                        } else {
                            "pat"
                        },
                        case == "reconnect",
                    )
                    .await;
                }
                if case == "collision" {
                    github_preset(a, JASON, "pat", false).await;
                }
                a.db()
                    .write(|tx| {
                        tx.conn().execute(
                            "UPDATE users SET github_login='old-profile' WHERE id=?",
                            [DAVID],
                        )?;
                        Ok(())
                    })
                    .await
                    .unwrap();
                Value::Null
            },
            async |b, _| {
                classic_change(
                    b,
                    if disconnect {
                        Method::DELETE
                    } else {
                        Method::POST
                    },
                    "github",
                    token,
                    notice,
                    message,
                )
                .await;
            },
            async |b, _| {
                if let Some(integrations) = api_change(
                    b,
                    if disconnect {
                        Method::DELETE
                    } else {
                        Method::PUT
                    },
                    "github",
                    token,
                    notice,
                    message,
                )
                .await
                {
                    assert_eq!(
                        integrations.github,
                        if disconnect {
                            api::Connection::Missing
                        } else {
                            api::Connection::Connected {
                                name: "octocat".into(),
                                workspace: None,
                                app_token: false,
                            }
                        }
                    );
                    assert!(integrations.github_app_configured);
                }
            },
        )
        .await
        else {
            return;
        };
        let audit = audits(&outcome);
        assert_eq!(
            audit.contains("github.account.connect"),
            notice && !disconnect,
            "{case}"
        );
        assert_eq!(
            audit.contains("github.account.disconnect"),
            matches!(case, "disconnect" | "disconnect_app"),
            "{case}"
        );
        let requests = server.received();
        assert_eq!(
            requests.iter().filter(|r| r.method == "GET").count(),
            if disconnect || case == "blank" { 0 } else { 2 },
            "{case}"
        );
        assert_eq!(
            requests.iter().filter(|r| r.method == "DELETE").count(),
            if case == "disconnect_app" { 2 } else { 0 },
            "{case}"
        );
        for request in requests.iter().filter(|r| r.method == "GET") {
            assert!(
                request
                    .headers
                    .iter()
                    .any(|(name, value)| name.eq_ignore_ascii_case("authorization")
                        && value == "Bearer new-personal-token")
            );
        }
        if !notice {
            assert_eq!(
                outcome.rows, outcome.before,
                "{case}: rejected change writes nothing"
            );
            assert!(
                outcome.rows["github_connected_accounts"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            assert!(outcome.rows["audit_logs"].as_array().unwrap().is_empty());
        }
    }
}

async fn fizzy_preset(a: &TestApp, rejected: bool) {
    let crypto = ArEncryption::new(&a.booted.app.secrets);
    a.db()
        .write(move |tx| {
            let account = accounts::Account::relink(
                tx,
                &crypto,
                &accounts::Input {
                    user_id: DAVID,
                    account_id: "897362094",
                    account_name: Some("Old workspace"),
                    fizzy_user_id: Some("old-user"),
                    fizzy_user_name: Some("Old name"),
                    token: "old-token",
                },
            )?;
            if rejected {
                account.mark_disconnected(tx, "Token rejected")?;
            }
            let card = cards::Card::for_reference(tx, "897362094", 579)?;
            cards::Cache::for_viewer(tx, &card, DAVID)?;
            cards::Cache::for_viewer(tx, &card, JASON)?;
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn fizzy_connections_match_every_notice_and_alert() {
    const MARKER: &str = "SPA_INTEGRATIONS_FIZZY_CASE";
    if let Ok(case) = std::env::var(MARKER) {
        fizzy_case(&case).await;
        return;
    }
    for case in [
        "blank",
        "rejected",
        "unreachable",
        "no_account",
        "blank_slug",
        "connect",
        "reconnect",
        "disconnect",
        "missing",
    ] {
        let output = ws15e_http_case(
            MARKER,
            case,
            "controllers::spa::integrations_tests::fizzy_connections_match_every_notice_and_alert",
        )
        .await;
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{case}: {stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains("1 passed; 0 failed"),
            "child must execute: {stdout}"
        );
        println!("SPA Fizzy parity {case}: 1 passed; 0 failed");
    }
}

async fn fizzy_case(case: &str) {
    let status = match case {
        "rejected" => 401,
        "unreachable" => 500,
        _ => 200,
    };
    let identity = match case {
        "no_account" => json!({"accounts": []}),
        "blank_slug" => json!({"accounts": [{"slug": " "}]}),
        _ => {
            json!({"accounts": [{"slug": "/897362094", "name": "Smart Data", "user": {"id": "03user1", "name": "David"}}]})
        }
    };
    let server = FakeServer::on_listener(
        vec![
            Route::new("GET", "127.0.0.1", "/my/identity.json", status).body(identity.to_string()),
        ],
        None,
        ws15e_http_case_listener(),
    )
    .await;
    let disconnect = matches!(case, "disconnect" | "missing");
    let token = if case == "blank" { " \t\n " } else { TOKEN };
    let (notice, message) = match case {
        "blank" => (false, "Paste a token to connect Fizzy."),
        "rejected" => (false, "Fizzy rejected that token. Check it and try again."),
        "unreachable" => (false, "Could not reach Fizzy. Try again."),
        "no_account" | "blank_slug" => (false, "That token has no Fizzy account to use."),
        _ if disconnect => (true, "Fizzy disconnected."),
        _ => (true, "Fizzy connected as David (Smart Data)."),
    };
    let Some(outcome) = assert_parity(
        async |a, _| {
            clear(a).await;
            if matches!(case, "reconnect" | "disconnect") {
                fizzy_preset(a, case == "reconnect").await;
            }
            Value::Null
        },
        async |b, _| {
            classic_change(
                b,
                if disconnect {
                    Method::DELETE
                } else {
                    Method::POST
                },
                "fizzy",
                token,
                notice,
                message,
            )
            .await;
        },
        async |b, _| {
            if let Some(integrations) = api_change(
                b,
                if disconnect {
                    Method::DELETE
                } else {
                    Method::PUT
                },
                "fizzy",
                token,
                notice,
                message,
            )
            .await
            {
                assert_eq!(
                    integrations.fizzy,
                    if disconnect {
                        api::Connection::Missing
                    } else {
                        api::Connection::Connected {
                            name: "David".into(),
                            workspace: Some("Smart Data".into()),
                            app_token: false,
                        }
                    }
                );
            }
        },
    )
    .await
    else {
        return;
    };
    let audit = audits(&outcome);
    assert_eq!(
        audit.contains("fizzy.account.connect"),
        notice && !disconnect,
        "{case}"
    );
    assert_eq!(
        audit.contains("fizzy.account.disconnect"),
        case == "disconnect",
        "{case}"
    );
    assert_eq!(
        server.received.lock().unwrap().len(),
        if disconnect || case == "blank" { 0 } else { 2 }
    );
    for request in server.received.lock().unwrap().iter() {
        assert!(
            request
                .headers
                .iter()
                .any(|(name, value)| name.eq_ignore_ascii_case("authorization")
                    && value == "Bearer new-personal-token")
        );
    }
    if case == "disconnect" {
        let rows = outcome.rows["fizzy_card_caches"].to_string();
        assert!(!rows.contains(&format!(r#"\"user_id\":{DAVID}"#)));
        assert!(rows.contains(&format!(r#"\"user_id\":{JASON}"#)));
    }
    if !notice {
        assert_eq!(
            outcome.rows, outcome.before,
            "{case}: rejected change writes nothing"
        );
        assert!(
            outcome.rows["fizzy_connected_accounts"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(outcome.rows["audit_logs"].as_array().unwrap().is_empty());
    }
}

async fn stopped_app() -> Option<TestApp> {
    let mut a = app().await?;
    a.booted.jobs.stop(Duration::from_secs(1)).await;
    Some(a)
}

#[tokio::test]
async fn google_disconnect_matches_entries_jobs_audits_and_publications() {
    for case in [
        "connected",
        "remote_failure",
        "rejected",
        "missing",
        "unconfigured",
    ] {
        let recorded = google::Recorded::new(vec![]);
        let prepare = async |a: &TestApp, _: &mut Browser<'_>| {
            clear(a).await;
            recorded.calls.lock().unwrap().clear();
            google::install(a, recorded.clone()).await;
            if case == "unconfigured" {
                a.booted
                    .app
                    .google
                    .install_api(crate::integrations::google::api::Api::new(
                        crate::integrations::google::api::Config {
                            client_id: String::new(),
                            ..google::config()
                        },
                        recorded.clone(),
                    ));
            } else if case != "missing" {
                google::grant(
                    a,
                    DAVID,
                    a.db()
                        .env()
                        .now()
                        .since(jiff::SignedDuration::from_hours(1)),
                    true,
                )
                .await;
                a.db().write(move |tx| {
                    tx.conn().execute("UPDATE google_accounts SET scopes=?,disconnected_reason=? WHERE user_id=?", rusqlite::params![campfire_db::models::google_account::CALENDAR_SCOPE, (case == "rejected").then_some("Token rejected"), DAVID])?;
                    tx.conn().execute("INSERT INTO calendar_push_channels(user_id,channel_id,token_digest,resource_id,created_at,updated_at) VALUES(?,'spa-channel','fixture','spa-resource',?,?)", rusqlite::params![DAVID,tx.now(),tx.now()])?;
                    tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,fetched_at,in_meeting_broadcast,created_at,updated_at) VALUES(?,'[]',?, ?,1,?,?)", rusqlite::params![DAVID,json!([[tx.now().ago(jiff::SignedDuration::from_hours(1)).jiff().to_string(),tx.now().since(jiff::SignedDuration::from_hours(1)).jiff().to_string()]]).to_string(),tx.now(),tx.now(),tx.now()])?;
                    tx.conn().execute("UPDATE users SET ooo_calendar_enabled=1,ooo_broadcast=1 WHERE id=?", [DAVID])?;
                    tx.conn().execute("UPDATE events SET meet_link='https://meet.google.test/spa' WHERE organizer_id=?", [DAVID])?;
                    tx.conn().execute("INSERT INTO event_calendar_entries(event_id,user_id,google_event_id,synced_at,created_at,updated_at) SELECT id,?,'spa-event-' || id,?,?,? FROM events WHERE organizer_id=?", rusqlite::params![DAVID,tx.now(),tx.now(),tx.now(),DAVID])?;
                    Ok(())
                }).await.unwrap();
                if case == "remote_failure" {
                    recorded.fail_for(Method::POST, "/calendar/v3/channels/stop");
                } else if case == "connected" {
                    recorded.answer_for(Method::POST, "/calendar/v3/channels/stop", 200, json!({}));
                }
            }
            Value::Null
        };
        let Some(outcome) = assert_parity_with_app(
            stopped_app,
            prepare,
            async |b, _| {
                if case == "unconfigured" {
                    assert_eq!(
                        b.write(Req::new(Method::DELETE, "/google/connection"))
                            .await
                            .status,
                        StatusCode::NOT_FOUND
                    );
                } else {
                    classic_change(
                        b,
                        Method::DELETE,
                        "google",
                        "",
                        true,
                        "Google Calendar disconnected.",
                    )
                    .await;
                }
            },
            async |b, _| {
                if case == "unconfigured" {
                    let reply = b
                        .write(Req::new(
                            Method::DELETE,
                            "/api/v1/settings/google_connection",
                        ))
                        .await;
                    assert_eq!(reply.status, StatusCode::NOT_FOUND);
                    assert_eq!(
                        error(&reply),
                        json!({"_tag": "NotFound", "message": "Not found"})
                    );
                } else {
                    let integrations = api_change(
                        b,
                        Method::DELETE,
                        "google",
                        "",
                        true,
                        "Google Calendar disconnected.",
                    )
                    .await
                    .unwrap();
                    assert_eq!(
                        integrations.google,
                        api::GoogleIntegration {
                            sign_in_configured: false,
                            identity_email: None,
                            calendar_configured: true,
                            connected: false,
                            calendar: false,
                            drive: false,
                            email: None,
                        }
                    );
                }
                let calls = recorded.calls.lock().unwrap();
                assert_eq!(
                    calls.len(),
                    usize::from(matches!(case, "connected" | "remote_failure"))
                );
                if let Some(call) = calls.first() {
                    assert_eq!(call["path"], "/calendar/v3/channels/stop");
                    assert_eq!(
                        serde_json::from_str::<Value>(call["body"].as_str().unwrap()).unwrap(),
                        json!({"id": "spa-channel", "resourceId": "spa-resource"})
                    );
                }
            },
        )
        .await
        else {
            return;
        };
        assert_eq!(
            audits(&outcome).contains("google.account.disconnect"),
            !matches!(case, "missing" | "unconfigured"),
            "{case}"
        );
        if matches!(case, "connected" | "remote_failure" | "rejected") {
            for table in [
                "google_accounts",
                "calendar_push_channels",
                "calendar_meeting_caches",
                "event_calendar_entries",
            ] {
                assert!(
                    outcome.rows[table].as_array().unwrap().is_empty(),
                    "{case}: {table}"
                );
            }
            let jobs: Vec<Value> = outcome.rows["background_jobs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| serde_json::from_str(r.as_str().unwrap()).unwrap())
                .collect();
            assert_eq!(jobs.len(), 1);
            if let Some(job) = jobs.first() {
                assert_eq!(job["job_class"], "Calendar::DisconnectCleanupJob");
                let args: Value = serde_json::from_str(job["arguments"].as_str().unwrap()).unwrap();
                assert!(
                    !args[0].as_array().unwrap().is_empty(),
                    "published event IDs stay in cleanup"
                );
                assert_eq!(args[1], "<encrypted snapshot>");
                assert!(args[2].is_number());
            }
        }
    }
}

fn actions() -> [(Method, &'static str); 5] {
    [
        (Method::PUT, "/api/v1/settings/github_connection"),
        (Method::DELETE, "/api/v1/settings/github_connection"),
        (Method::PUT, "/api/v1/settings/fizzy_connection"),
        (Method::DELETE, "/api/v1/settings/fizzy_connection"),
        (Method::DELETE, "/api/v1/settings/google_connection"),
    ]
}

#[tokio::test]
async fn integration_writes_refuse_signed_out_bot_keys_and_agent_tokens_without_changes() {
    use crate::controllers::agent_http_tests::{SECRET, initialize};
    let Some(a) = stopped_app().await else { return };
    clear(&a).await;
    google::install(&a, google::Recorded::new(vec![])).await;
    initialize(&a).await;
    let authorization = format!("Bearer {SECRET}");
    let mut anonymous = a.anonymous();
    // Credential authentication stamps last_used_at once before comparing refused writes.
    anonymous
        .send(get("/api/v1/me").header("authorization", &authorization))
        .await;
    let capture = a.booted.app.cable.capture_every_publication();
    let before = dump(&a).await;
    for (method, path) in actions() {
        for (kind, expected_status, tag) in [
            ("anonymous", StatusCode::UNAUTHORIZED, "Unauthorized"),
            ("bot", StatusCode::FORBIDDEN, "Forbidden"),
            ("agent", StatusCode::FORBIDDEN, "Forbidden"),
        ] {
            let path = if kind == "bot" {
                format!("{path}?bot_key={BENDER_KEY}")
            } else {
                path.into()
            };
            let mut request = json_body(method.clone(), &path, &json!({"accessToken": TOKEN}));
            if kind == "agent" {
                request = request.header("authorization", &authorization);
            }
            let reply = anonymous.send(request).await;
            assert_eq!(
                reply.status,
                expected_status,
                "{kind} {path}: {}",
                reply.text()
            );
            assert_eq!(error(&reply)["_tag"], tag);
            assert_eq!(dump(&a).await, before, "{kind} {path}");
        }
    }
    assert!(settle(&capture).await.is_empty());
}

#[tokio::test]
async fn integration_writes_require_csrf_and_sudo_and_allow_members_own_connections() {
    let Some(a) = stopped_app().await else { return };
    clear(&a).await;
    google::install(&a, google::Recorded::new(vec![])).await;
    let mut b = a.sign_in(KEVIN).await;
    b.authenticity_token().await;
    let before = dump(&a).await;
    for (method, path) in actions() {
        let reply = b
            .send(json_body(method.clone(), path, &json!({"accessToken": ""})))
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error(&reply)["_tag"], "InvalidAuthenticityToken");
        let reply = b
            .write(
                json_body(method, path, &json!({"accessToken": ""}))
                    .header("referer", "http://campfire.test/app/settings"),
            )
            .await;
        assert_eq!(
            reply.status,
            StatusCode::FORBIDDEN,
            "{path}: {}",
            reply.text()
        );
        assert_eq!(error(&reply)["_tag"], "SudoRequired");
    }
    assert_eq!(dump(&a).await, before);
    b.grant_sudo().await;
    for (method, path) in actions() {
        let reply = b
            .write(json_body(method.clone(), path, &json!({"accessToken": ""})))
            .await;
        assert_eq!(
            reply.status,
            if method == Method::PUT {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::OK
            },
            "{path}: {}",
            reply.text()
        );
        if method == Method::PUT {
            assert_eq!(error(&reply)["_tag"], "Validation");
        }
    }
    assert_eq!(
        dump(&a).await,
        before,
        "members affect only their own missing connections"
    );
}

#[tokio::test]
async fn integration_replies_do_not_repair_unrelated_unreadable_tokens() {
    let Some(outcome) = assert_parity_with_app(
        stopped_app,
        async |a, _| {
            clear(a).await;
            google::install(a, google::Recorded::new(vec![])).await;
            github_preset(a, DAVID, "pat", false).await;
            fizzy_preset(a, false).await;
            a.db().write(|tx| {
                tx.conn().execute("UPDATE github_connected_accounts SET access_token='unreadable' WHERE user_id=?", [DAVID])?;
                tx.conn().execute("UPDATE fizzy_connected_accounts SET access_token='unreadable' WHERE user_id=?", [DAVID])?;
                Ok(())
            }).await.unwrap();
            Value::Null
        },
        async |b, _| {
            classic_change(b, Method::DELETE, "google", "", true, "Google Calendar disconnected.").await;
        },
        async |b, _| {
            let integrations = api_change(b, Method::DELETE, "google", "", true, "Google Calendar disconnected.").await.unwrap();
            let rejected = api::Connection::Rejected { reason: Some(github_accounts::UNREADABLE_TOKEN_REASON.into()) };
            assert_eq!(integrations.github, rejected);
            assert_eq!(integrations.fizzy, rejected);
        },
    ).await else { return };
    assert_eq!(
        outcome.rows, outcome.before,
        "reply and settings read add no writes"
    );
    assert!(outcome.frames.is_empty());
}
