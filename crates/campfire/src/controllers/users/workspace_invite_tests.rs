use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::models::workspace_invite::{InviteExpiry, WorkspaceInvite};
use campfire_db::{NewUser, User};
use serde_json::{Value, json};

fn frozen_clock() -> campfire_kit::SharedClock {
    std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ))
}

async fn invite(app: &TestApp) -> (WorkspaceInvite, String) {
    app.db()
        .write(|tx| WorkspaceInvite::create(tx, DAVID, InviteExpiry::OneHour, Some(1)))
        .await
        .unwrap()
}

#[tokio::test]
async fn workspace_invite_enrollment_keeps_join_form_memberships_sessions_and_duplicate_redirect() {
    let app = TestApp::boot_frozen()
        .await
        .expect("seed required")
        .without_job_runner()
        .await;
    let (invite, token) = invite(&app).await;
    let path = format!("/invite/{token}");
    let before = app.db().read(User::count).await.unwrap();
    let mut browser = app.anonymous();
    let page = browser.get(&path).await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.text().contains(&format!("action=\"{path}\"")));
    assert!(
        page.text()
            .contains("Create your account to start chatting.")
    );
    assert_eq!(
        app.david().get(&path).await.location(),
        Some("http://campfire.test/")
    );
    let duplicate = browser
        .write(Req::new(Method::POST, &path).form(&[
            ("user[name]", "Duplicate"),
            ("user[email_address]", "david@37signals.com"),
            ("user[password]", "secret123456"),
        ]))
        .await;
    assert_eq!(
        duplicate.location(),
        Some("http://campfire.test/session/new?email_address=david%4037signals.com")
    );
    assert_eq!(
        app.db()
            .read(move |c| WorkspaceInvite::find(c, invite.id))
            .await
            .unwrap()
            .unwrap()
            .uses,
        0
    );
    let signup = browser
        .write(Req::new(Method::POST, &path).form(&[
            ("user[name]", "Invited"),
            ("user[email_address]", "invited@example.com"),
            ("user[password]", "secret123456"),
            ("user[role]", "administrator"),
        ]))
        .await;
    assert_eq!(signup.location(), Some("http://campfire.test/"));
    let user = app
        .db()
        .read(|c| {
            Ok(c.query_row(
                "SELECT id FROM users WHERE email_address='invited@example.com'",
                [],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    let enrolled = browser.get("/two_factor_setup").await;
    assert!(
        enrolled
            .text()
            .contains(&format!("name=\"current-user-id\" content=\"{user}\""))
    );
    app.db()
        .read(move |c| {
            assert_eq!(User::find(c, user)?.role, campfire_db::Role::Member);
            assert_eq!(User::count(c)?, before + 1);
            assert_eq!(WorkspaceInvite::find(c, invite.id)?.unwrap().uses, 1);
            let memberships: i64 = c.query_row(
                "SELECT COUNT(*) FROM memberships WHERE user_id=?",
                [user],
                |r| r.get(0),
            )?;
            let open: i64 = c.query_row(
                "SELECT COUNT(*) FROM rooms WHERE type='Rooms::Open'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(memberships, open);
            assert_eq!(campfire_db::Session::for_user(c, user)?.len(), 1);
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn workspace_invite_dead_and_unknown_gets_and_posts_render_clear_states() {
    let app = TestApp::boot_frozen()
        .await
        .expect("seed required")
        .without_job_runner()
        .await;
    let before = app.db().read(User::count).await.unwrap();
    for state in ["expired", "exhausted", "revoked", "unknown"] {
        let (invite, token) = invite(&app).await;
        let redemption_token = token.clone();
        app.db()
            .write(move |tx| {
                match state {
                    "expired" => {
                        tx.conn().execute(
                            "UPDATE workspace_invites SET expires_at=? WHERE id=?",
                            rusqlite::params![tx.now(), invite.id],
                        )?;
                    }
                    "exhausted" => {
                        WorkspaceInvite::redeem(
                            tx,
                            &redemption_token,
                            NewUser {
                                name: "Last use".into(),
                                ..Default::default()
                            },
                        )?;
                    }
                    "revoked" => {
                        WorkspaceInvite::revoke(tx, invite.id)?;
                    }
                    _ => {}
                }
                Ok(())
            })
            .await
            .unwrap();
        let digest = WorkspaceInvite::digest(&token);
        assert_eq!(
            app.anonymous()
                .get(&format!("/invite/{digest}"))
                .await
                .status,
            StatusCode::NOT_FOUND
        );
        let token = if state == "unknown" {
            "unknown".to_string()
        } else {
            token
        };
        let status = if state == "unknown" {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::GONE
        };
        let path = format!("/invite/{token}");
        let mut browser = app.anonymous();
        for reply in [
            browser.get(&path).await,
            browser
                .write(Req::new(Method::POST, &path).form(&[("user[name]", "Must not create")]))
                .await,
        ] {
            assert_eq!(reply.status, status);
            assert!(reply.text().contains("This invite is no longer valid."));
            assert!(!reply.text().contains("Create account</button>"));
            assert!(!reply.text().contains("auth-form"));
        }
    }
    assert_eq!(app.db().read(User::count).await.unwrap(), before + 1);
}

fn json_request(method: Method, path: &str, body: Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(body.to_string())
}

#[tokio::test]
async fn workspace_invite_concurrent_signups_render_the_losing_join_state() {
    let app = TestApp::boot_frozen()
        .await
        .expect("seed required")
        .without_job_runner()
        .await;
    let (invite, token) = invite(&app).await;
    let path = format!("/invite/{token}");
    let mut first = app.anonymous();
    let mut second = app.anonymous();
    first.get(&path).await;
    second.get(&path).await;
    let before = app.db().read(User::count).await.unwrap();
    let (first, second) = tokio::join!(
        first.write(Req::new(Method::POST, &path).form(&[
            ("user[name]", "First signup"),
            ("user[password]", "secret123456")
        ])),
        second.write(Req::new(Method::POST, &path).form(&[
            ("user[name]", "Second signup"),
            ("user[password]", "secret123456")
        ])),
    );
    let mut statuses = [first.status, second.status];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::FOUND, StatusCode::GONE]);
    let loser = if first.status == StatusCode::GONE {
        first
    } else {
        second
    };
    assert!(loser.text().contains("This invite is no longer valid."));
    assert!(!loser.text().contains("auth-form"));
    assert_eq!(app.db().read(User::count).await.unwrap(), before + 1);
    assert_eq!(
        app.db()
            .read(move |c| WorkspaceInvite::find(c, invite.id))
            .await
            .unwrap()
            .unwrap()
            .uses,
        1
    );
}

#[tokio::test]
async fn workspace_invite_api_accepts_every_choice_and_lists_live_states() {
    let app = TestApp::boot_seed_with_env("default", frozen_clock(), &[("SPA_ENABLED", "1")])
        .await
        .expect("seed required")
        .without_job_runner()
        .await;
    let mut admin = app.david();
    let path = "/api/v1/admin/invites";
    for (choice, seconds, cap) in [
        ("30m", Some(1800), Some(1)),
        ("1h", Some(3600), Some(5)),
        ("6h", Some(21600), Some(10)),
        ("12h", Some(43200), Some(25)),
        ("1d", Some(86400), Some(50)),
        ("7d", Some(604800), Some(100)),
        ("never", None, None),
    ] {
        let reply = admin
            .write(json_request(
                Method::POST,
                path,
                json!({"expiry":choice,"maxUses":cap}),
            ))
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let created: Value = serde_json::from_slice(&reply.body).unwrap();
        assert_eq!(created["invite"]["maxUses"], json!(cap));
        let now = campfire_db::Timestamp::from_jiff(SEED_NOW.parse().unwrap());
        let expiry = seconds.map(|s| now.since(jiff::SignedDuration::from_secs(s)).to_wire());
        assert_eq!(created["invite"]["expiresAt"], json!(expiry));
        let id = created["invite"]["id"].as_i64().unwrap();
        match choice {
            "30m" => {
                app.db()
                    .write(move |tx| {
                        tx.conn().execute(
                            "UPDATE workspace_invites SET expires_at=? WHERE id=?",
                            rusqlite::params![tx.now(), id],
                        )?;
                        Ok(())
                    })
                    .await
                    .unwrap();
            }
            "1h" => {
                app.db()
                    .write(move |tx| {
                        tx.conn().execute(
                            "UPDATE workspace_invites SET uses=max_uses WHERE id=?",
                            [id],
                        )?;
                        Ok(())
                    })
                    .await
                    .unwrap();
            }
            "6h" => {
                app.db()
                    .write(move |tx| WorkspaceInvite::revoke(tx, id))
                    .await
                    .unwrap();
            }
            _ => {}
        }
    }
    let reply = admin.get(path).await;
    assert_eq!(reply.status, StatusCode::OK);
    let listed: Value = serde_json::from_slice(&reply.body).unwrap();
    let rows = listed["invites"].as_array().unwrap();
    assert_eq!(rows.len(), 7);
    assert_eq!(
        rows.iter()
            .map(|r| r["state"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "active",
            "active",
            "active",
            "active",
            "revoked",
            "exhausted",
            "expired"
        ]
    );
}

#[tokio::test]
async fn workspace_invite_api_is_admin_only_and_exposes_token_once_with_audits() {
    let app = TestApp::boot_seed_with_env("default", frozen_clock(), &[("SPA_ENABLED", "1")])
        .await
        .expect("seed required")
        .without_job_runner()
        .await;
    let path = "/api/v1/admin/invites";
    for (mut browser, status) in [
        (app.anonymous(), StatusCode::UNAUTHORIZED),
        (app.sign_in(KEVIN).await, StatusCode::FORBIDDEN),
        (app.sign_in(BENDER).await, StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            browser
                .send(Req::new(Method::GET, path).header("accept", "application/json"))
                .await
                .status,
            status
        );
        assert_eq!(
            browser
                .write(json_request(
                    Method::POST,
                    path,
                    json!({"expiry":"1h","maxUses":1})
                ))
                .await
                .status,
            status
        );
        assert_eq!(
            browser
                .write(Req::new(Method::DELETE, "/api/v1/admin/invites/1"))
                .await
                .status,
            status
        );
    }
    let mut admin = app.david();
    for invalid in [
        json!({"expiry":"2h","maxUses":1}),
        json!({"expiry":"1h","maxUses":2}),
        json!({"expiry":"1h","maxUses":0}),
    ] {
        assert_eq!(
            admin
                .write(json_request(Method::POST, path, invalid))
                .await
                .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let created = admin
        .write(json_request(
            Method::POST,
            path,
            json!({"expiry":"1h","maxUses":1}),
        ))
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let created: Value = serde_json::from_slice(&created.body).unwrap();
    let token = created["token"].as_str().unwrap();
    let id = created["invite"]["id"].as_i64().unwrap();
    assert_eq!(created["invite"]["state"], "active");
    assert_eq!(created["invite"]["creator"]["id"], DAVID);
    assert_eq!(created["invite"]["creator"]["name"], "David");
    assert_eq!(created["invite"]["uses"], 0);
    assert_eq!(created["invite"]["maxUses"], 1);
    assert_eq!(created["invite"]["expiresAt"], "2026-03-02T17:00:00.000Z");
    assert_eq!(
        created["url"],
        format!("http://campfire.test/invite/{token}")
    );
    let listed = admin.get(path).await;
    assert_eq!(listed.status, StatusCode::OK);
    assert!(!listed.text().contains(token));
    assert!(!listed.text().contains("tokenDigest"));
    let revoke = format!("{path}/{id}");
    let revoked = admin.write(Req::new(Method::DELETE, &revoke)).await;
    assert_eq!(revoked.status, StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<Value>(&revoked.body).unwrap()["state"],
        "revoked"
    );
    assert_eq!(
        admin.write(Req::new(Method::DELETE, &revoke)).await.status,
        StatusCode::OK
    );
    assert_eq!(
        admin
            .write(Req::new(Method::DELETE, "/api/v1/admin/invites/999999"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let audit_page = admin
        .get("/api/v1/admin/audit_log?action=workspace_invite.create&targetType=WorkspaceInvite")
        .await;
    assert_eq!(audit_page.status, StatusCode::OK);
    let audit_page: Value = serde_json::from_slice(&audit_page.body).unwrap();
    assert_eq!(audit_page["filters"]["action"], "workspace_invite.create");
    assert_eq!(audit_page["filters"]["targetType"], "WorkspaceInvite");
    assert_eq!(audit_page["entries"].as_array().unwrap().len(), 1);
    let token = token.to_string();
    app.db().read(move |c| {
        let audits: Vec<(String, String)> = c.prepare("SELECT action, details FROM audit_logs WHERE target_type='WorkspaceInvite' AND target_id=? ORDER BY id")?.query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
        assert_eq!(audits.iter().map(|a| a.0.as_str()).collect::<Vec<_>>(), ["workspace_invite.create", "workspace_invite.revoke"]);
        assert!(!format!("{audits:?}").contains(&token));
        Ok(())
    }).await.unwrap();
}
