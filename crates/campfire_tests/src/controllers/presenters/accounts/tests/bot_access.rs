//! Seeded HTTP equivalents of Rails credential/grant administration cases.
use super::*;

#[tokio::test]
async fn demoted_administrator_keeps_owner_reads_but_loses_credential_and_grant_creation() {
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let david: i64 = test.label("users.david").parse().unwrap();
    let mut viewer = test.browser("198.51.100.158");
    viewer.sign_in(&test.label("emails.david")).await;
    viewer.grant_sudo_access();
    test.booted
        .app
        .db
        .write(move |tx| {
            campfire_db::User::find(tx.conn(), david)?.update(
                tx,
                campfire_db::UserChanges {
                    role: Some(campfire_db::Role::Member),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let path = format!("/account/bots/{bot}");
    for area in ["credentials", "grants"] {
        let response = viewer.get(&format!("{path}/{area}")).await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        let back = if area == "credentials" {
            format!("/account/bots/{bot}/edit")
        } else {
            format!("/users/{bot}")
        };
        assert!(response.text().contains(&format!("href=\"{back}\"")));
    }
    assert_eq!(
        viewer
            .form(
                "post",
                &format!("{path}/credentials"),
                &[("agent_credential[name]", "Must not issue")]
            )
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        viewer
            .form(
                "post",
                &format!("{path}/grants"),
                &[("agent_grant[capability]", "react")]
            )
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    test.booted
        .app
        .db
        .read(|conn| {
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM agent_credentials", [], |r| r
                    .get::<_, i64>(0))?,
                1
            );
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM agent_grants", [], |r| r
                    .get::<_, i64>(0))?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn grant_scopes_name_direct_participants_and_keep_deleted_room_history() {
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let room: i64 = test.label("rooms.watercooler").parse().unwrap();
    let direct: i64 = test.label("rooms.bender_and_kevin").parse().unwrap();
    test.booted
        .app
        .db
        .write(move |tx| {
            let agent = campfire_db::Agent::for_user(tx.conn(), bot)?.unwrap();
            for room_id in [room, direct] {
                campfire_db::AgentGrant::create(
                    tx,
                    campfire_db::NewGrant {
                        agent_id: agent.id,
                        room_id: Some(room_id),
                        capability: "post_messages".into(),
                        granted_by_id: 127326141,
                        ..Default::default()
                    },
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let mut admin = test.browser("198.51.100.159");
    admin.sign_in(&test.label("emails.david")).await;
    let path = format!("/account/bots/{bot}/grants");
    let response = admin.get(&path).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    assert!(
        response.text().contains("Bender, Kevin"),
        "{}",
        response.text()
    );
    assert!(!response.text().contains("Deleted room"));
    test.booted
        .app
        .db
        .write(move |tx| campfire_db::Room::find(tx.conn(), room)?.destroy(tx))
        .await
        .unwrap();
    let response = admin.get(&path).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    assert!(response.text().contains("Deleted room"));
    assert!(response.text().contains("Revoked"));
    assert!(response.text().contains("Bender, Kevin"));
}
async fn owner(test: &Test, bot_id: i64) {
    let owner_id: i64 = test.label("users.kevin").parse().unwrap();
    test.booted
        .app
        .db
        .write(move |tx| {
            let mut agent = campfire_db::Agent::for_user(tx.conn(), bot_id)?.unwrap();
            agent.update(
                tx,
                campfire_db::AgentChanges {
                    owner_id: Some(Some(owner_id)),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn credentials_index_exposes_last_four_and_local_datetime_only() {
    let test = boot_seed("default").await.expect("default seed");
    let mut admin = test.browser("198.51.100.221");
    admin.sign_in(&test.label("emails.david")).await;
    let response = admin
        .get(&format!(
            "/account/bots/{}/credentials",
            test.label("users.bender")
        ))
        .await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("Main"));
    assert!(response.text().contains("f4f0"));
    assert!(
        response
            .text()
            .contains("data-local-time-target=\"datetime\"")
    );
    assert!(!response.text().contains("bender-test-secret-1234"));
}
#[tokio::test]
async fn credentials_create_requires_sudo_and_reveals_digest_backed_secret_once() {
    let test = boot_seed("default").await.expect("default seed");
    let mut admin = test.browser("198.51.100.222");
    admin.sign_in(&test.label("emails.david")).await;
    let path = format!("/account/bots/{}/credentials", test.label("users.bender"));
    let fields = [("agent_credential[name]", "CI runner")];
    assert_redirect(
        &admin.form("post", &path, &fields).await,
        "http://campfire.test/sudo/new",
    );
    admin.grant_sudo_access();
    let response = admin.form("post", &path, &fields).await;
    assert_eq!(response.status, StatusCode::CREATED);
    let html = response.text();
    let secret = html
        .split("aria-label=\"New credential secret\"")
        .next()
        .unwrap()
        .rsplit("value=\"")
        .next()
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned();
    assert!(!secret.is_empty());
    let captured = secret.clone();
    test.booted.app.db.read(move |conn| {
        let credential=campfire_db::AgentCredential::authenticate(conn,&captured,campfire_db::Timestamp::from_jiff(seed_clock().now()))?.unwrap();
        assert_eq!(credential.name,"CI runner"); assert_eq!(credential.created_by_id,127326141);
        assert_eq!(credential.token_last_four,credential.token_digest[..4]);
        let audit:String=conn.query_row("SELECT details FROM audit_logs WHERE action='agent.credential.create' ORDER BY id DESC LIMIT 1",[],|r|r.get(0))?;
        assert!(!audit.contains(&captured)); Ok(())
    }).await.unwrap();
    assert!(!admin.get(&path).await.text().contains(&secret));
}
#[tokio::test]
async fn credentials_owner_lists_and_revokes_but_cannot_issue() {
    let test = boot_seed("default").await.expect("default seed");
    let id: i64 = test.label("users.bender").parse().unwrap();
    owner(&test, id).await;
    let credential_id:i64=test.booted.app.db.read(move |conn| Ok(conn.query_row("SELECT c.id FROM agent_credentials c JOIN agents a ON a.id=c.agent_id WHERE a.user_id=? LIMIT 1",[id],|r|r.get(0))?)).await.unwrap();
    let mut viewer = test.browser("198.51.100.223");
    viewer.sign_in(&test.label("emails.kevin")).await;
    let path = format!("/account/bots/{id}/credentials");
    let index = viewer.get(&path).await;
    assert_eq!(index.status, StatusCode::OK);
    assert!(!index.text().contains("name=\"agent_credential[name]\""));
    assert_eq!(
        viewer
            .form("post", &path, &[("agent_credential[name]", "Sneaky")])
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let revoke = format!("{path}/{credential_id}");
    assert_redirect(
        &viewer.form("delete", &revoke, &[]).await,
        "http://campfire.test/sudo/new",
    );
    viewer.grant_sudo_access();
    for _ in 0..2 {
        assert_redirect(
            &viewer.form("delete", &revoke, &[]).await,
            &format!("http://campfire.test{path}"),
        );
    }
    test.booted.app.db.read(move |conn| {assert!(campfire_db::AgentCredential::find(conn,credential_id)?.unwrap().revoked_at.is_some());assert!(campfire_db::AgentCredential::authenticate(conn,"bender-test-secret-1234",campfire_db::Timestamp::from_jiff(seed_clock().now()))?.is_none()); assert_eq!(conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='agent.credential.revoke' AND target_id=?",[credential_id],|r|r.get::<_,i64>(0))?,1);Ok(())}).await.unwrap();
}
#[tokio::test]
async fn credentials_invalid_name_and_nonowner_do_not_write() {
    let test = boot_seed("default").await.expect("default seed");
    let path = format!("/account/bots/{}/credentials", test.label("users.bender"));
    let mut admin = test.browser("198.51.100.224");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    let response = admin
        .form("post", &path, &[("agent_credential[name]", "")])
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.text().contains("Name can&#39;t be blank"));
    let mut viewer = test.browser("198.51.100.225");
    viewer.sign_in(&test.label("emails.kevin")).await;
    assert_eq!(viewer.get(&path).await.status, StatusCode::FORBIDDEN);
    assert_eq!(
        viewer
            .form("post", &path, &[("agent_credential[name]", "Sneaky")])
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        test.booted
            .app
            .db
            .read(|conn| Ok(
                conn.query_row("SELECT COUNT(*) FROM agent_credentials", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn legacy_admin_visits_create_agent_once_after_authorization() {
    let test = boot_seed("default").await.expect("default seed");
    let id = test
        .booted
        .app
        .db
        .write(|tx| Ok(campfire_db::User::create_bot(tx, "Legacy access management", None)?.id))
        .await
        .unwrap();
    let mut viewer = test.browser("198.51.100.226");
    viewer.sign_in(&test.label("emails.kevin")).await;
    assert_eq!(
        viewer
            .get(&format!("/account/bots/{id}/credentials"))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert!(
        test.booted
            .app
            .db
            .read(move |conn| campfire_db::Agent::for_user(conn, id))
            .await
            .unwrap()
            .is_none()
    );
    let mut admin = test.browser("198.51.100.227");
    admin.sign_in(&test.label("emails.david")).await;
    for kind in ["credentials", "grants", "credentials"] {
        assert_eq!(
            admin
                .get(&format!("/account/bots/{id}/{kind}"))
                .await
                .status,
            StatusCode::OK
        );
    }
    test.booted
        .app
        .db
        .read(move |conn| {
            let agent = campfire_db::Agent::for_user(conn, id)?.unwrap();
            assert_eq!(agent.owner_id, Some(127326141));
            assert_eq!(agent.kind, campfire_db::AgentKind::Workspace);
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action='agent.create' AND target_id=?",
                    [agent.id],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn grants_create_is_idempotent_and_revocation_disables_legacy_fallback() {
    let test = boot_seed("default").await.expect("default seed");
    let id: i64 = test.label("users.bender").parse().unwrap();
    let mut admin = test.browser("198.51.100.228");
    admin.sign_in(&test.label("emails.david")).await;
    let path = format!("/account/bots/{id}/grants");
    assert!(admin.get(&path).await.text().contains("Legacy access"));
    let fields = [
        ("agent_grant[capability]", "post_messages"),
        ("agent_grant[room_id]", ""),
    ];
    assert_redirect(
        &admin.form("post", &path, &fields).await,
        "http://campfire.test/sudo/new",
    );
    admin.grant_sudo_access();
    for _ in 0..2 {
        assert_redirect(
            &admin.form("post", &path, &fields).await,
            &format!("http://campfire.test{path}"),
        );
    }
    assert!(!admin.get(&path).await.text().contains("Legacy access"));
    let grant_id: i64 = test
        .booted
        .app
        .db
        .read(|conn| Ok(conn.query_row("SELECT id FROM agent_grants LIMIT 1", [], |r| r.get(0))?))
        .await
        .unwrap();
    for _ in 0..2 {
        assert_redirect(
            &admin
                .form("delete", &format!("{path}/{grant_id}"), &[])
                .await,
            &format!("http://campfire.test{path}"),
        );
    }
    test.booted
        .app
        .db
        .read(move |conn| {
            let agent = campfire_db::Agent::for_user(conn, id)?.unwrap();
            assert!(!agent.legacy_capabilities(conn)?);
            assert!(!agent.has_capability_anywhere(conn, "post_messages")?);
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM agent_grants", [], |r| r
                    .get::<_, i64>(0))?,
                1
            );
            for action in ["agent.grant.create", "agent.grant.revoke"] {
                assert_eq!(
                    conn.query_row(
                        "SELECT COUNT(*) FROM audit_logs WHERE action=?",
                        [action],
                        |r| r.get::<_, i64>(0)
                    )?,
                    1
                );
            }
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn grants_owner_views_and_revokes_but_cannot_widen() {
    let test = boot_seed("default").await.expect("default seed");
    let id: i64 = test.label("users.bender").parse().unwrap();
    owner(&test, id).await;
    let grant_id = test
        .booted
        .app
        .db
        .write(move |tx| {
            let agent = campfire_db::Agent::for_user(tx.conn(), id)?.unwrap();
            Ok(campfire_db::AgentGrant::create(
                tx,
                campfire_db::NewGrant {
                    agent_id: agent.id,
                    granted_by_id: 127326141,
                    capability: "read_messages".into(),
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap();
    let path = format!("/account/bots/{id}/grants");
    let mut viewer = test.browser("198.51.100.229");
    viewer.sign_in(&test.label("emails.kevin")).await;
    viewer.grant_sudo_access();
    let page = viewer.get(&path).await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.text().contains(&format!("href=\"/users/{id}\"")));
    assert!(!page.text().contains("name=\"agent_grant[capability]\""));
    for capability in ["read_messages", "external_action"] {
        for room in ["", &test.label("rooms.watercooler")] {
            assert_eq!(
                viewer
                    .form(
                        "post",
                        &path,
                        &[
                            ("agent_grant[capability]", capability),
                            ("agent_grant[room_id]", room)
                        ]
                    )
                    .await
                    .status,
                StatusCode::FORBIDDEN
            );
        }
    }
    assert_redirect(
        &viewer
            .form("delete", &format!("{path}/{grant_id}"), &[])
            .await,
        &format!("http://campfire.test{path}"),
    );
}
#[tokio::test]
async fn grants_invalid_scope_capability_and_missing_room_render_errors() {
    let test = boot_seed("default").await.expect("default seed");
    let path = format!("/account/bots/{}/grants", test.label("users.bender"));
    let mut admin = test.browser("198.51.100.230");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    for (capability, room, expected) in [
        (
            "dm_anyone",
            test.label("rooms.watercooler"),
            "dm_anyone is granted workspace-wide only",
        ),
        (
            "unknown",
            String::new(),
            "Capability is not included in the list",
        ),
        (
            "post_messages",
            "999999999".into(),
            "Room must be an existing room",
        ),
    ] {
        let reply = admin
            .form(
                "post",
                &path,
                &[
                    ("agent_grant[capability]", capability),
                    ("agent_grant[room_id]", &room),
                ],
            )
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(reply.text().contains(expected));
    }
    assert_eq!(
        test.booted
            .app
            .db
            .read(|conn| Ok(
                conn.query_row("SELECT COUNT(*) FROM agent_grants", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        0
    );
}
#[tokio::test]
async fn credential_local_expiry_uses_the_viewer_time_zone() {
    let test = boot_seed("default").await.expect("default seed");
    test.booted
        .app
        .db
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET time_zone='America/New_York' WHERE id=127326141",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut admin = test.browser("198.51.100.231");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    let path = format!("/account/bots/{}/credentials", test.label("users.bender"));
    assert_eq!(
        admin
            .form(
                "post",
                &path,
                &[
                    ("agent_credential[name]", "Local expiry"),
                    ("agent_credential[expires_at]", "2026-07-01T14:30")
                ]
            )
            .await
            .status,
        StatusCode::CREATED
    );
    test.booted
        .app
        .db
        .read(|conn| {
            let at: campfire_db::Timestamp = conn.query_row(
                "SELECT expires_at FROM agent_credentials WHERE name='Local expiry'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(
                at.jiff(),
                "2026-07-01T18:30:00Z".parse::<jiff::Timestamp>().unwrap()
            );
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        admin
            .get(&path)
            .await
            .text()
            .contains("datetime=\"2026-07-01T14:30:00-04:00\"")
    );
}
#[tokio::test]
async fn credential_string_expiry_matches_pinned_rails_in_both_viewer_zones() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/bot-input-contract.json"
    )))
    .unwrap();
    let test = boot_seed("default").await.expect("default seed");
    let viewer: i64 = test.label("users.david").parse().unwrap();
    let mut admin = test.browser("198.51.100.232");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    let path = format!("/account/bots/{}/credentials", test.label("users.bender"));
    let mut checked = 0;
    for (index, row) in corpus["expiry"].as_array().unwrap().iter().enumerate() {
        let Some(input) = row["input"].as_str() else {
            continue;
        };
        let zone = row["zone"].as_str().unwrap().to_owned();
        test.booted
            .app
            .db
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE users SET time_zone=? WHERE id=?",
                    rusqlite::params![zone, viewer],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let name = format!("Expiry contract {index}");
        let created = admin
            .form(
                "post",
                &path,
                &[
                    ("agent_credential[name]", &name),
                    ("agent_credential[expires_at]", input),
                ],
            )
            .await;
        assert_eq!(
            created.status,
            StatusCode::CREATED,
            "{} / {input:?}",
            row["zone"]
        );
        let actual = test
            .booted
            .app
            .db
            .read(move |conn| {
                let value: Option<campfire_db::Timestamp> = conn.query_row(
                    "SELECT expires_at FROM agent_credentials WHERE name=?",
                    [name],
                    |r| r.get(0),
                )?;
                Ok(value.map(|v| v.jiff()))
            })
            .await
            .unwrap();
        let expected = row["value"]
            .as_str()
            .map(|s| s.parse::<jiff::Timestamp>().unwrap());
        assert_eq!(actual, expected, "{} / {input:?}", row["zone"]);
        checked += 1;
    }
    assert_eq!(
        checked, 28,
        "every committed string case must reach the HTTP/domain stack"
    );
}

#[tokio::test]
async fn concurrent_grant_creation_writes_one_grant_and_audit() {
    let test = boot_seed("default").await.expect("default seed");
    let mut first = test.browser("198.51.100.232");
    first.sign_in(&test.label("emails.david")).await;
    first.grant_sudo_access();
    let mut second = test.browser("198.51.100.233");
    // This test needs two verified sessions, not a replay of one frozen TOTP.
    // Use Rails' committed seed device for the second browser's real password login.
    second.cookies.insert(
        rails_compat::cookies::TWO_FACTOR_REMEMBER.into(),
        test.label("two_factor_cookies.david"),
    );
    second.sign_in(&test.label("emails.david")).await;
    second.grant_sudo_access();
    let path = format!("/account/bots/{}/grants", test.label("users.bender"));
    let fields = [
        ("agent_grant[capability]", "react"),
        ("agent_grant[room_id]", ""),
    ];
    let (a, b) = tokio::join!(
        first.form("post", &path, &fields),
        second.form("post", &path, &fields)
    );
    for response in [a, b] {
        assert_redirect(&response, &format!("http://campfire.test{path}"));
    }
    test.booted
        .app
        .db
        .read(|conn| {
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM agent_grants", [], |r| r
                    .get::<_, i64>(0))?,
                1
            );
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action='agent.grant.create'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn credential_owner_with_sudo_still_cannot_issue() {
    let test = boot_seed("default").await.expect("default seed");
    let id: i64 = test.label("users.bender").parse().unwrap();
    owner(&test, id).await;
    let mut viewer = test.browser("198.51.100.234");
    viewer.sign_in(&test.label("emails.kevin")).await;
    viewer.grant_sudo_access();
    assert_eq!(
        viewer
            .form(
                "post",
                &format!("/account/bots/{id}/credentials"),
                &[("agent_credential[name]", "Widen")]
            )
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        test.booted
            .app
            .db
            .read(|conn| Ok(
                conn.query_row("SELECT COUNT(*) FROM agent_credentials", [], |r| r
                    .get::<_, i64>(0))?
            ))
            .await
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn credential_and_grant_revocation_are_scoped_to_requested_bot() {
    let test = boot_seed("default").await.expect("default seed");
    let (credential_id, grant_id) = test
        .booted
        .app
        .db
        .write(|tx| {
            let bot = campfire_db::User::create_bot(tx, "Other scoped bot", None)?;
            let agent = campfire_db::Agent::create(
                tx,
                campfire_db::NewAgent {
                    user_id: bot.id,
                    owner_id: Some(127326141),
                    kind: campfire_db::AgentKind::Workspace,
                    ..Default::default()
                },
            )?;
            let credential = campfire_db::AgentCredential::create_with_secret(
                tx,
                agent.id,
                "Other credential",
                127326141,
                None,
            )?
            .0;
            let grant = campfire_db::AgentGrant::create(
                tx,
                campfire_db::NewGrant {
                    agent_id: agent.id,
                    capability: "react".into(),
                    granted_by_id: 127326141,
                    ..Default::default()
                },
            )?;
            Ok((credential.id, grant.id))
        })
        .await
        .unwrap();
    let mut admin = test.browser("198.51.100.235");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    for (kind, id) in [("credentials", credential_id), ("grants", grant_id)] {
        assert_eq!(
            admin
                .form(
                    "delete",
                    &format!("/account/bots/{}/{kind}/{id}", test.label("users.bender")),
                    &[]
                )
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
    test.booted
        .app
        .db
        .read(move |conn| {
            assert!(
                campfire_db::AgentCredential::find(conn, credential_id)?
                    .unwrap()
                    .revoked_at
                    .is_none()
            );
            assert!(
                campfire_db::AgentGrant::find(conn, grant_id)?
                    .unwrap()
                    .revoked_at
                    .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn failed_creation_writes_roll_back_credential_and_grant() {
    let test = boot_seed("default").await.expect("default seed");
    // Pinned access_boundaries.rb separately proves owner-write rollback and
    // audit-failure persistence. These triggers fail the actual owner inserts.
    test.booted.app.db.write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_management_credential BEFORE INSERT ON agent_credentials BEGIN SELECT RAISE(FAIL,'credential rejected'); END; CREATE TRIGGER reject_management_grant BEFORE INSERT ON agent_grants BEGIN SELECT RAISE(FAIL,'grant rejected'); END;")?;Ok(())}).await.unwrap();
    let mut admin = test.browser("198.51.100.236");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    let path = format!("/account/bots/{}", test.label("users.bender"));
    assert_eq!(
        admin
            .form(
                "post",
                &format!("{path}/credentials"),
                &[("agent_credential[name]", "Rolled back")]
            )
            .await
            .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        admin
            .form(
                "post",
                &format!("{path}/grants"),
                &[("agent_grant[capability]", "react")]
            )
            .await
            .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    test.booted
        .app
        .db
        .read(|conn| {
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM agent_credentials", [], |r| r
                    .get::<_, i64>(0))?,
                1
            );
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM agent_grants", [], |r| r
                    .get::<_, i64>(0))?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}
