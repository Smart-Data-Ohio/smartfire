//! People reads and ban writes through the JSON twins. The S7 admin harness compares every
//! database table, including audit rows and pending jobs, and every cable publication.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::{DndAllowedUser, Session, User, WorkspacePresenceLease};
use campfire_people::controllers::qr_code;
use serde_json::{Value, json};

use super::admin_tests::{
    app, assert_parity, audits, classic, dump, error, get, json_body, nothing, parse, settle, spa,
    write,
};
use crate::controllers::presenters::{
    self,
    test_support::{BENDER, BENDER_KEY, Browser, DAVID, JASON, KEVIN, Req, TestApp},
};

const RITA: i64 = 773523954;
const MALLORY: i64 = 773523955;
const UNKNOWN: i64 = 999_999_999;

/// Seed browser sessions have loopback IPs, which classic correctly refuses to ban.
async fn public_sessions(a: &TestApp, _: &mut Browser<'_>) -> Value {
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE sessions SET ip_address='203.0.113.' || (user_id % 200 + 10)",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    Value::Null
}

#[tokio::test]
async fn ban_and_remove_ban_match_classic() {
    for (method, id, status, action) in [
        (Method::POST, KEVIN, api::UserStatus::Banned, "user.ban"),
        (
            Method::DELETE,
            MALLORY,
            api::UserStatus::Active,
            "user.unban",
        ),
    ] {
        let path = format!("/users/{id}/ban");
        let api_path = format!("/api/v1/people/{id}/ban");
        let Some(outcome) = assert_parity(
            public_sessions,
            async |b, _| classic(b, method.clone(), &path, &[]).await,
            async |b, _| {
                let profile: api::PersonProfile =
                    spa(b, method.clone(), &api_path, Value::Null).await;
                assert_eq!(profile.user.id, id);
                assert_eq!(profile.user.status, status);
                assert!(profile.status.is_some());
                assert!(profile.email_address.is_some());
                assert!(profile.can_ban);
                assert_eq!(
                    profile.transfer_url.is_some(),
                    status == api::UserStatus::Active
                );
                assert_eq!(
                    profile.transfer_qr_svg,
                    profile
                        .transfer_url
                        .as_deref()
                        .and_then(qr_code::transfer_svg)
                );
                assert_eq!(
                    profile.dnd_allowed.is_some(),
                    status == api::UserStatus::Active
                );
            },
        )
        .await
        else {
            return;
        };
        assert!(audits(&outcome).contains(action), "{}", audits(&outcome));
        if method == Method::POST {
            assert!(
                outcome
                    .frames
                    .iter()
                    .any(|(stream, _)| stream == &format!("action_cable/gid://campfire/User/{id}")),
                "{:?}",
                outcome.frames
            );
        }
        assert_ne!(outcome.before, outcome.rows);
    }
}

/// Keep classic's permissive server behavior even though the page offers neither button.
#[tokio::test]
async fn self_bot_and_deactivated_targets_keep_classic_behavior() {
    for id in [DAVID, BENDER, RITA] {
        for method in [Method::POST, Method::DELETE] {
            let path = format!("/users/{id}/ban");
            let api_path = format!("/api/v1/people/{id}/ban");
            assert_parity(
                public_sessions,
                async |b, _| classic(b, method.clone(), &path, &[]).await,
                async |b, _| {
                    let profile: api::PersonProfile =
                        spa(b, method.clone(), &api_path, Value::Null).await;
                    assert_eq!(profile.user.id, id);
                    assert_eq!(
                        profile.user.status,
                        if method == Method::POST {
                            api::UserStatus::Banned
                        } else {
                            api::UserStatus::Active
                        }
                    );
                },
            )
            .await;
        }
    }
}

#[tokio::test]
async fn ban_refusals_keep_classic_order_and_change_nothing() {
    for method in [Method::POST, Method::DELETE] {
        for (viewer, sudo, id, status, tag, message) in [
            (
                DAVID,
                false,
                KEVIN,
                StatusCode::FORBIDDEN,
                "SudoRequired",
                "Confirm your password to continue",
            ),
            (
                KEVIN,
                true,
                KEVIN,
                StatusCode::FORBIDDEN,
                "Forbidden",
                "Not allowed",
            ),
            // Permission precedes lookup; lookup precedes sudo.
            (
                KEVIN,
                false,
                UNKNOWN,
                StatusCode::FORBIDDEN,
                "Forbidden",
                "Not allowed",
            ),
            (
                DAVID,
                false,
                UNKNOWN,
                StatusCode::NOT_FOUND,
                "NotFound",
                "Not found",
            ),
            (
                DAVID,
                true,
                UNKNOWN,
                StatusCode::NOT_FOUND,
                "NotFound",
                "Not found",
            ),
        ] {
            let Some(a) = app().await else { return };
            let mut b = a.sign_in(viewer).await;
            if sudo {
                b.grant_sudo().await;
            }
            let before = dump(&a).await;
            let capture = a.booted.app.cable.capture_every_publication();
            let classic_reply = b
                .write(Req::new(method.clone(), &format!("/users/{id}/ban")))
                .await;
            if tag == "SudoRequired" {
                assert_eq!(classic_reply.status, StatusCode::FOUND);
                assert_eq!(
                    classic_reply.location(),
                    Some("http://campfire.test/sudo/new")
                );
            } else {
                assert_eq!(classic_reply.status, status);
            }
            assert_eq!(dump(&a).await, before);
            let reply = write(
                &mut b,
                method.clone(),
                &format!("/api/v1/people/{id}/ban"),
                Value::Null,
            )
            .await;
            assert_eq!(reply.status, status, "{}", reply.text());
            assert_eq!(error(&reply), json!({"_tag": tag, "message": message}));
            assert_eq!(dump(&a).await, before);
            assert!(settle(&capture).await.is_empty());
        }
    }
}

#[tokio::test]
async fn invalid_ban_is_bare_422_in_classic_and_validation_in_api() {
    for method in [Method::POST, Method::DELETE] {
        let prepare = async |a: &TestApp, b: &mut Browser<'_>| {
            public_sessions(a, b).await;
            a.db()
                .write(|tx| {
                    tx.conn().execute(
                        "UPDATE users SET time_zone='Invalid/People' WHERE id=?",
                        [KEVIN],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
            Value::Null
        };
        let Some(outcome) = assert_parity(
            prepare,
            async |b, _| {
                let reply = b
                    .write(Req::new(method.clone(), &format!("/users/{KEVIN}/ban")))
                    .await;
                assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
                assert_eq!(
                    reply.body,
                    include_bytes!("../../../../web/public/422.html")
                );
            },
            async |b, _| {
                let reply = write(
                    b,
                    method.clone(),
                    &format!("/api/v1/people/{KEVIN}/ban"),
                    Value::Null,
                )
                .await;
                assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
                assert_eq!(
                    error(&reply),
                    json!({"_tag": "Validation", "message": "Time zone is not a valid time zone",
                    "fields": {"timeZone": ["is not a valid time zone"]}})
                );
            },
        )
        .await
        else {
            return;
        };
        assert_eq!(outcome.rows, outcome.before);
        assert!(outcome.frames.is_empty());
    }
}

#[tokio::test]
async fn directory_matches_classic_presenters_in_order() {
    let Some(a) = app().await else { return };
    a.db().write(|tx| {
        tx.conn().execute("DELETE FROM user_stars WHERE user_id=?", [DAVID])?;
        tx.conn().execute("INSERT INTO user_stars(user_id,starred_user_id,created_at,updated_at) VALUES(?,?,?,?)",
            rusqlite::params![DAVID, KEVIN, tx.now(), tx.now()])?;
        let session = Session::start(tx, KEVIN, Some("People"), Some("203.0.113.5"))?;
        WorkspacePresenceLease::establish(tx, KEVIN, session.id)?;
        // The default seed already has agents; make one visibly online.
        tx.conn().execute("UPDATE agents SET suspended_at=NULL,last_seen_at=? WHERE user_id=?",
            rusqlite::params![tx.now(), BENDER])?;
        Ok(())
    }).await.unwrap();
    let mut b = a.sign_in(DAVID).await;
    let before = dump(&a).await;
    let reply = b.send(get("/api/v1/people")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    let directory: api::PeopleDirectory = parse(&reply);
    let secrets = a.booted.app.secrets.clone();
    let now = a.db().env().now();
    let expected = a
        .db()
        .read(move |conn| {
            Ok(
                campfire_db::models::user::presentation::directory(conn, DAVID, now)?
                    .into_iter()
                    .map(|p| presenters::people::person(&secrets, p))
                    .collect::<Vec<_>>(),
            )
        })
        .await
        .unwrap();
    let actual: Vec<_> = directory
        .people
        .iter()
        .map(|p| (p.user_id, p.online, p.starred, p.agent))
        .collect();
    assert_eq!(
        actual,
        expected
            .iter()
            .map(|p| (p.user.id, p.online, p.starred, p.agent))
            .collect::<Vec<_>>()
    );
    assert_eq!(directory.people[0].user_id, KEVIN);
    assert!(directory.people.iter().any(|p| p.online));
    assert!(directory.people.iter().any(|p| p.agent));
    assert!(
        !directory
            .people
            .iter()
            .any(|p| [DAVID, RITA, MALLORY].contains(&p.user_id))
    );
    let mut ids: Vec<_> = directory.people.iter().map(|p| p.user_id).collect();
    ids.sort_unstable();
    assert_eq!(
        directory.users.iter().map(|u| u.id).collect::<Vec<_>>(),
        ids
    );
    for user in &directory.users {
        let source = expected.iter().find(|p| p.user.id == user.id).unwrap();
        assert_eq!(user.name, source.user.name);
        assert_eq!(user.avatar_url, source.user.avatar_path);
    }
    let ids = ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
    let users: api::UserList = parse(&b.send(get(&format!("/api/v1/users?ids={ids}"))).await);
    assert_eq!(directory.users, users.users);
    assert_eq!(dump(&a).await, before);
}

#[tokio::test]
async fn private_ip_bans_keep_classic_validation_and_roll_back() {
    let Some(outcome) = assert_parity(
        nothing,
        async |b, _| {
            let reply = b
                .write(Req::new(Method::POST, &format!("/users/{KEVIN}/ban")))
                .await;
            assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(
                reply.body,
                include_bytes!("../../../../web/public/422.html")
            );
        },
        async |b, _| {
            let reply = write(
                b,
                Method::POST,
                &format!("/api/v1/people/{KEVIN}/ban"),
                Value::Null,
            )
            .await;
            assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(
                error(&reply),
                json!({"_tag": "Validation",
                "message": "Ip address cannot be a private or internal IP address",
                "fields": {"ipAddress": ["cannot be a private or internal IP address"]}})
            );
        },
    )
    .await
    else {
        return;
    };
    assert_eq!(outcome.rows, outcome.before);
    assert!(outcome.frames.is_empty());
}

async fn check_profile(
    a: &TestApp,
    b: &mut Browser<'_>,
    viewer: i64,
    id: i64,
) -> api::PersonProfile {
    let reply = b.send(get(&format!("/api/v1/people/{id}"))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    let profile: api::PersonProfile = parse(&reply);
    let secrets = a.booted.app.secrets.clone();
    let now = a.db().env().now();
    let (user, viewer_user, status) = a
        .db()
        .read(move |conn| {
            let user = User::find(conn, id)?;
            let viewer_user = User::find(conn, viewer)?;
            let settings = campfire_db::UserStatusSettings::find(conn, viewer)?;
            let status = presenters::status_settings::profile_status_in_zone(
                conn,
                &secrets,
                id,
                viewer,
                now,
                &settings.zone(),
            )?;
            Ok((user, viewer_user, status))
        })
        .await
        .unwrap();
    let person = !user.is_bot() && !user.is_deactivated();
    let administrator = viewer_user.can_administer(None, false);
    assert_eq!(profile.user.id, user.id);
    assert_eq!(profile.user.name, user.name);
    assert_eq!(profile.user.bio, user.bio);
    let users: api::UserList = parse(&b.send(get(&format!("/api/v1/users?ids={id}"))).await);
    assert_eq!(profile.user, users.users[0]);
    // Conditions in `users/show.html:21,45-49,51,58-65`.
    assert_eq!(profile.status.is_some(), person);
    if let Some(actual) = &profile.status {
        assert_eq!(
            serde_json::to_value(actual.presence).unwrap(),
            json!(status.presence)
        );
        assert_eq!(actual.status_text, status.status_text);
    }
    assert_eq!(
        profile.dnd_allowed,
        if person && user.is_active() && viewer != id {
            Some(status.dnd_allowed)
        } else {
            None
        }
    );
    assert_eq!(
        profile.email_address,
        if person && administrator {
            user.email_address.clone()
        } else {
            None
        }
    );
    let transfer =
        presenters::accounts::transfer_id(&a.booted.app.secrets, id, a.booted.app.clock.now());
    assert_eq!(
        profile.transfer_url,
        if person && user.is_active() && administrator {
            Some(format!(
                "http://campfire.test{}",
                campfire_routes::session_transfer(&transfer)
            ))
        } else {
            None
        }
    );
    assert_eq!(profile.can_ban, person && administrator && viewer != id);
    assert_eq!(
        profile.transfer_qr_svg,
        profile
            .transfer_url
            .as_deref()
            .and_then(qr_code::transfer_svg)
    );
    profile
}

#[tokio::test]
async fn profile_fields_follow_classic_visibility_and_request_zone() {
    let Some(a) = app().await else { return };
    a.db().write(|tx| {
        tx.conn().execute("UPDATE users SET time_zone='America/Los_Angeles',time_zone_explicit=1 WHERE id=? OR name='JZ'",
            [DAVID])?;
        tx.conn().execute("UPDATE users SET time_zone=NULL,ooo_until=?,ooo_note=NULL,custom_status_text=NULL,custom_status_emoji=NULL WHERE id=?",
            rusqlite::params![campfire_db::Timestamp::from_jiff("2026-03-03T02:00:00Z".parse().unwrap()), KEVIN])?;
        tx.conn().execute("DELETE FROM dnd_allowed_users WHERE user_id=? AND allowed_user_id=?", [DAVID, KEVIN])?;
        Ok(())
    }).await.unwrap();
    let mut b = a.sign_in(DAVID).await;
    let before = dump(&a).await;
    let member = check_profile(&a, &mut b, DAVID, KEVIN).await;
    assert_eq!(member.dnd_allowed, Some(false));
    assert!(member.email_address.is_some() && member.transfer_url.is_some() && member.can_ban);
    assert_eq!(
        member.transfer_qr_svg,
        member
            .transfer_url
            .as_deref()
            .and_then(qr_code::transfer_svg)
    );
    let secrets = a.booted.app.secrets.clone();
    let now = a.db().env().now();
    let utc = a
        .db()
        .read(move |conn| {
            presenters::status_settings::profile_status_in_zone(
                conn,
                &secrets,
                KEVIN,
                DAVID,
                now,
                &jiff::tz::TimeZone::UTC,
            )
        })
        .await
        .unwrap();
    assert_ne!(
        member.status.as_ref().unwrap().status_text,
        utc.status_text,
        "zone-sensitive status fixture"
    );
    let own = check_profile(&a, &mut b, DAVID, DAVID).await;
    assert_eq!(own.dnd_allowed, None);
    assert!(!own.can_ban && own.transfer_url.is_some());
    assert_eq!(
        own.transfer_qr_svg,
        own.transfer_url.as_deref().and_then(qr_code::transfer_svg)
    );
    for id in [MALLORY, RITA, BENDER] {
        check_profile(&a, &mut b, DAVID, id).await;
    }
    assert_eq!(dump(&a).await, before);
    a.db()
        .write(|tx| DndAllowedUser::create(tx, DAVID, KEVIN))
        .await
        .unwrap();
    assert_eq!(
        check_profile(&a, &mut b, DAVID, KEVIN).await.dnd_allowed,
        Some(true)
    );
    let member_id = a
        .db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT id FROM users WHERE name='JZ'", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    let mut member = a.sign_in(member_id).await;
    let before = dump(&a).await;
    for id in [KEVIN, member_id, JASON, MALLORY, RITA, BENDER] {
        let profile = check_profile(&a, &mut member, member_id, id).await;
        assert_eq!(profile.email_address, None);
        assert_eq!(profile.transfer_url, None);
        assert_eq!(
            profile.transfer_qr_svg,
            profile
                .transfer_url
                .as_deref()
                .and_then(qr_code::transfer_svg)
        );
        assert!(!profile.can_ban);
    }
    assert_eq!(dump(&a).await, before);
    let missing = b.send(get(&format!("/api/v1/people/{UNKNOWN}"))).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert_eq!(
        error(&missing),
        json!({"_tag": "NotFound", "message": "Not found"})
    );
}

fn actions() -> [(Method, String); 4] {
    [
        (Method::GET, "/api/v1/people".into()),
        (Method::GET, format!("/api/v1/people/{KEVIN}")),
        (Method::POST, format!("/api/v1/people/{KEVIN}/ban")),
        (Method::DELETE, format!("/api/v1/people/{KEVIN}/ban")),
    ]
}

#[tokio::test]
async fn every_people_route_refuses_signed_out_bot_keys_and_warmed_agent_tokens() {
    use crate::controllers::agent_http_tests::{SECRET, initialize};
    let Some(a) = app().await else { return };
    initialize(&a).await;
    let authorization = format!("Bearer {SECRET}");
    let mut b = a.anonymous();
    b.send(get("/api/v1/me").header("authorization", &authorization))
        .await;
    let before = dump(&a).await;
    let capture = a.booted.app.cable.capture_every_publication();
    for (method, path) in actions() {
        for (kind, status, tag) in [
            ("anonymous", StatusCode::UNAUTHORIZED, "Unauthorized"),
            ("bot", StatusCode::FORBIDDEN, "Forbidden"),
            ("agent", StatusCode::FORBIDDEN, "Forbidden"),
        ] {
            let path = if kind == "bot" {
                format!("{path}?bot_key={BENDER_KEY}")
            } else {
                path.clone()
            };
            let mut request = json_body(method.clone(), &path, &Value::Null);
            if kind == "agent" {
                request = request.header("authorization", &authorization);
            }
            let reply = b.send(request).await;
            assert_eq!(reply.status, status, "{kind} {path}: {}", reply.text());
            let message = if kind == "anonymous" {
                "Sign in to continue"
            } else {
                "Not allowed"
            };
            assert_eq!(error(&reply), json!({"_tag": tag, "message": message}));
            assert_eq!(dump(&a).await, before);
        }
    }
    assert!(settle(&capture).await.is_empty());
}

#[tokio::test]
async fn both_ban_writes_require_csrf() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    b.grant_sudo().await;
    let before = dump(&a).await;
    let capture = a.booted.app.cable.capture_every_publication();
    for method in [Method::POST, Method::DELETE] {
        let reply = b
            .send(json_body(
                method,
                &format!("/api/v1/people/{KEVIN}/ban"),
                &Value::Null,
            ))
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error(&reply)["_tag"], "InvalidAuthenticityToken");
        assert_eq!(dump(&a).await, before);
    }
    assert!(settle(&capture).await.is_empty());
}
