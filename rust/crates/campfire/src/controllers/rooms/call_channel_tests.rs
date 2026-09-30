pub(super) fn configured() -> crate::huddle::Config {
    crate::huddle::Config {
        public_url: Some("wss://public.example.test".into()),
        internal_url: Some("http://internal.example.test:7880".into()),
        api_key: Some("ws13-fixture-api-key".into()),
        api_secret: Some("ws13-fixture-api-secret".into()),
        gateway_secret: Some("ws13-fixture-gateway-secret".into()),
    }
}
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN, Req, TestApp};
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Room, RoomType};

#[tokio::test]
async fn call_channel_updates_deny_unprivileged_members_and_wrong_namespaces() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    for (kind, namespace) in [(RoomType::Voice, "voices"), (RoomType::Stage, "stages")] {
        let room = test
            .db()
            .write(move |tx| {
                Room::create_for(tx, kind, Some("Private"), DAVID, &[DAVID, JASON, KEVIN])
            })
            .await
            .unwrap();
        let path = format!("/rooms/{namespace}/{}", room.id);
        let mut member = test.sign_in(KEVIN).await;
        let reply = member
            .write(Req::new(Method::PUT, &path).form(&[("room[name]", "Exposed")]))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::FORBIDDEN,
            "{namespace}: {}",
            reply.text()
        );
        let mut admin = test.sign_in(DAVID).await;
        for wrong in [
            "opens",
            "closeds",
            if namespace == "voices" {
                "stages"
            } else {
                "voices"
            },
        ] {
            let reply = admin
                .write(
                    Req::new(Method::PUT, &format!("/rooms/{wrong}/{}", room.id))
                        .form(&[("room[name]", "Exposed")]),
                )
                .await;
            assert_eq!(
                reply.status,
                StatusCode::FOUND,
                "{namespace} via {wrong}: {}",
                reply.text()
            );
            assert_eq!(reply.location(), Some("http://campfire.test/"));
        }
        let id = room.id;
        let unchanged = test
            .db()
            .read(move |conn| Room::find(conn, id))
            .await
            .unwrap();
        assert_eq!(unchanged.room_type, kind);
        assert_eq!(unchanged.name.as_deref(), Some("Private"));
    }
}

#[tokio::test]
async fn stage_member_edit_cannot_remove_the_sole_host_or_commit_the_rename() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let room = test
        .db()
        .write(|tx| {
            Room::create_for(
                tx,
                RoomType::Stage,
                Some("Town Hall"),
                DAVID,
                &[DAVID, JASON],
            )
        })
        .await
        .unwrap();
    let mut browser = test.sign_in(DAVID).await;
    let reply = browser
        .write(
            Req::new(Method::PUT, &format!("/rooms/stages/{}", room.id)).form(&[
                ("room[name]", "Stranded"),
                ("user_ids[]", &JASON.to_string()),
            ]),
        )
        .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert!(
        reply
            .text()
            .contains("Promote another host before removing David")
    );
    assert_eq!(
        test.db()
            .read(move |conn| Ok((
                Room::find(conn, room.id)?.name,
                conn.query_row_cached(
                    "SELECT COUNT(*) FROM memberships WHERE room_id=?",
                    [room.id],
                    |r| r.get::<_, i64>(0)
                )?
            )))
            .await
            .unwrap(),
        (Some("Town Hall".into()), 2)
    );
}

#[tokio::test]
async fn call_forms_rows_and_huddle_layouts_match_parity_seed_rails_bytes() {
    use crate::controllers::presenters::page;
    use askama::Template;
    use campfire_views::{
        helpers::request_forgery::{AuthenticityTokens, RequestSecrets, rendering_with},
        rooms::{
            FormRoom,
            calls::{CallForm, CallRow, StageForm, StagesNew, VoiceForm, VoicesNew},
        },
    };
    struct Tokens;
    impl AuthenticityTokens for Tokens {
        fn global(&self) -> String {
            "GLOBAL".into()
        }
        fn for_form(&self, action: &str, method: &str) -> String {
            format!("{method}:{action}")
        }
    }
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("call_view_vectors.json")).unwrap();
    for case in vectors["cases"].as_array().unwrap() {
        let input = &case["input"];
        let actual =
            page::render_detached_at(&test.booted.app, None, "http://campfire.test", |ctx| {
                if input["kind"] == "row" || input["kind"] == "header" {
                    let row = CallRow {
                        id: input["id"].as_i64().unwrap(),
                        name: input["name"].as_str().unwrap().into(),
                        stage: input["stage"].as_bool().unwrap(),
                        icon: None,
                        participants: serde_json::from_value(input["participants"].clone())
                            .unwrap(),
                        live: input["live"].as_bool().unwrap(),
                        live_name: "David".into(),
                        unread: input["unread"].as_bool().unwrap(),
                        muted: false,
                        membership: input["membership"].as_bool().unwrap(),
                        favorited: false,
                        favorite_position: None,
                        category_id: None,
                        can_delete: input["membership"].as_bool().unwrap(),
                    };
                    if input["kind"] == "header" {
                        row.header(ctx)
                    } else {
                        row.render(ctx)
                    }
                } else {
                    let form = CallForm {
                        room: FormRoom {
                            id: input["id"].as_i64(),
                            name: input["name"].as_str().map(str::to_string),
                        },
                        stage: input["stage"].as_bool().unwrap(),
                        can_administer: true,
                        current_user_id: DAVID,
                        selected_users: serde_json::from_value(input["selected_users"].clone())
                            .unwrap(),
                        unselected_users: serde_json::from_value(input["unselected_users"].clone())
                            .unwrap(),
                        icon_name: None,
                        icon: None,
                        errors: Vec::new(),
                    };
                    rendering_with(
                        RequestSecrets {
                            tokens: Box::new(Tokens),
                            csp_nonce: Some("NONCE".into()),
                        },
                        || {
                            if input["kind"] == "new_page" {
                                if form.stage {
                                    Ok(StagesNew { ctx, form: &form }.as_content().to_string())
                                } else {
                                    Ok(VoicesNew { ctx, form: &form }.as_content().to_string())
                                }
                            } else if form.stage {
                                StageForm { ctx, form: &form }.render()
                            } else {
                                VoiceForm { ctx, form: &form }.render()
                            }
                        },
                    )
                    .unwrap()
                }
            });
        if actual != case["html"].as_str().unwrap() {
            let scratch = std::env::var_os("TMPDIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    std::path::PathBuf::from(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../.scratch"
                    ))
                });
            std::fs::create_dir_all(&scratch).unwrap();
            std::fs::write(scratch.join("call-view-actual.html"), &actual).unwrap();
            std::fs::write(
                scratch.join("call-view-expected.html"),
                case["html"].as_str().unwrap(),
            )
            .unwrap();
        }
        assert_eq!(actual, case["html"].as_str().unwrap(), "{}", case["name"]);
    }
    let user = test
        .db()
        .read(|conn| campfire_db::User::find(conn, DAVID))
        .await
        .unwrap();
    let current =
        crate::controllers::presenters::view_context::current_user(&test.booted.app.secrets, &user);
    page::render_detached_at(&test.booted.app, None, "http://campfire.test", |ctx| {
        assert_eq!(
            campfire_views::layouts::Huddle {
                ctx,
                user: &current
            }
            .render()
            .unwrap(),
            vectors["layouts"]["huddle"].as_str().unwrap()
        );
        assert_eq!(
            campfire_views::layouts::HuddleInvitation.render().unwrap(),
            vectors["layouts"]["huddle_invitation"].as_str().unwrap()
        );
        assert_eq!(
            campfire_views::layouts::HuddleJoinNotice { ctx }
                .render()
                .unwrap(),
            vectors["layouts"]["huddle_join_notice"].as_str().unwrap()
        );
    });
}

#[tokio::test]
async fn call_channel_http_matches_production_rails() {
    use super::call_lifecycle_tests::insert;
    use campfire_db::{Membership, Role};
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("call_channel_vectors.json")).unwrap();
    for case in vectors["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let Some(test) = TestApp::boot().await else {
            return;
        };
        let input = case["input"].clone();
        test.db().write(move |tx| {
            insert(tx,"rooms",&input["room"])?;
            for row in input["memberships"].as_array().unwrap() {insert(tx,"memberships",row)?;}
            for u in input["users"].as_array().unwrap() {tx.conn().execute_cached("UPDATE users SET role=? WHERE id=?",rusqlite::params![Role::from_name(u["role"].as_str().unwrap()).unwrap(),u["id"].as_i64()])?;}
            tx.conn().execute_cached("UPDATE accounts SET settings=?",[serde_json::json!({"restrict_room_creation_to_administrators":input["restricted"]}).to_string()])?;
            Ok(())
        }).await.unwrap();
        let mut browser = test.sign_in(case["actor_id"].as_i64().unwrap()).await;
        let method =
            Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap();
        let req = Req::new(method.clone(), case["path"].as_str().unwrap())
            .header("x-forwarded-proto", "https")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(case["request_body"].as_str().unwrap_or_default().as_bytes());
        let reply = if method == Method::GET {
            browser.send(req).await
        } else {
            browser.write(req).await
        };
        assert_eq!(
            reply.status.as_u16() as u64,
            case["status"].as_u64().unwrap(),
            "{name}: {}",
            reply.text()
        );
        let id = if case["action"] == "create" && reply.status == StatusCode::FOUND {
            let location = reply.location().unwrap();
            assert!(
                location.starts_with("https://campfire.test/rooms/"),
                "{name}: {location}"
            );
            location.rsplit('/').next().unwrap().parse::<i64>().unwrap()
        } else {
            assert_eq!(reply.location(), case["location"].as_str(), "{name}");
            9001
        };
        if let Some(error) = case["error"].as_str() {
            assert!(reply.text().contains(error), "{name}: {error}");
        }
        let actual=test.db().read(move |conn| {
            let room=Room::find(conn,id)?;
            let mut members=Membership::for_room(conn,id)?;members.sort_by_key(|m|m.user_id);
            let members:Vec<_>=members.into_iter().map(|m|serde_json::json!({"user_id":m.user_id,"stage_role":m.stage_role.map(|r|r.name())})).collect();
            let mut stmt=conn.prepare_cached("SELECT action,details FROM audit_logs WHERE target_type='Room' AND target_id=? ORDER BY id")?;
            let audits=stmt.query_map([id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?.into_iter().map(|(action,details)|serde_json::json!({"action":action,"details":serde_json::from_str::<serde_json::Value>(&details).unwrap()})).collect::<Vec<_>>();
            Ok(serde_json::json!({"room":{"type":room.room_type.class_name(),"name":room.name,"icon_name":room.icon_name,"creator_id":room.creator_id},"members":members,"audits":audits,"notes":conn.query_row_cached("SELECT COUNT(*) FROM messages WHERE room_id=? AND system_note=1",[id],|r|r.get::<_,i64>(0))?}))
        }).await.unwrap();
        assert_eq!(
            actual,
            serde_json::json!({"room":case["room"],"members":case["members"],"audits":case["audits"],"notes":case["notes"]}),
            "{name}"
        );
    }
}

#[tokio::test]
async fn deleting_a_call_channel_uses_ws8a_marking_and_ends_grants_and_streams_before_reply() {
    use campfire_db::models::{huddle_grant::HuddleGrant, room_delete::HuddleConfig};
    use campfire_db::{Membership, NewSession, Session};
    use tower::ServiceExt;
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("call_channel_vectors.json")).unwrap();
    for case in vectors["deletions"].as_array().unwrap() {
        let Some(test) = TestApp::boot_with_huddle(configured()).await else {
            return;
        };
        let input = vectors["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| {
                c["name"]
                    == if case["stage"] == true {
                        "stages_update"
                    } else {
                        "voices_update"
                    }
            })
            .unwrap()["input"]
            .clone();
        test.db()
            .write(move |tx| {
                super::call_lifecycle_tests::insert(tx, "rooms", &input["room"])?;
                tx.conn()
                    .execute_cached("UPDATE rooms SET name='Delete me' WHERE id=9001", [])?;
                for m in input["memberships"].as_array().unwrap() {
                    super::call_lifecycle_tests::insert(tx, "memberships", m)?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let session = test
            .db()
            .write(|tx| {
                Session::start_with(
                    tx,
                    DAVID,
                    NewSession {
                        user_agent: None,
                        ip_address: None,
                        device_id: None,
                        two_factor_verified: true,
                    },
                )
            })
            .await
            .unwrap();
        let mut browser = test.sign_in(DAVID).await;
        let csrf = browser.authenticity_token().await;
        let cookies = browser.cookie_header();
        let app = test.booted.app.clone();
        let router = test.booted.router.clone();
        drop(browser);
        test.booted
            .jobs
            .shutdown(std::time::Duration::from_secs(1))
            .await;
        let stage = case["stage"] == true;
        app.db.write(move |tx| {
            let member=Membership::find_by_room_and_user(tx.conn(),9001,DAVID)?.unwrap();
            let mut grant=HuddleGrant::issue(tx,session.id,member.id,9001,&HuddleConfig{api_secret:Some("ws13-fixture-api-secret".into()),admin_configured:false})?;grant.record_seen(tx)?;
            if stage {tx.conn().execute_cached("INSERT INTO streams (id,room_id,user_id,membership_id,quality,started_at,created_at,updated_at) VALUES (40,9001,?,?, '1080p15',?,?,?)",rusqlite::params![DAVID,member.id,tx.now(),tx.now(),tx.now()])?;}
            Ok(())
        }).await.unwrap();
        let request = axum::http::Request::builder()
            .method(Method::DELETE)
            .uri("/rooms/9001")
            .header("host", "campfire.test")
            .header("accept", "application/json")
            .header("cookie", cookies)
            .header(campfire_kit::csrf::HEADER, csrf)
            .body(axum::body::Body::empty())
            .unwrap();
        let response = router.oneshot(request).await.unwrap();
        assert_eq!(
            response.status().as_u16() as u64,
            case["status"].as_u64().unwrap()
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            case["body"]
        );
        let state=app.db.read(move |conn| {
            let room=Room::find(conn,9001)?;
            let mut stmt=conn.prepare_cached("SELECT action,details FROM audit_logs WHERE target_type='Room' AND target_id=9001 ORDER BY id")?;
            let audits=stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?.into_iter().map(|(action,details)|serde_json::json!({"action":action,"details":serde_json::from_str::<serde_json::Value>(&details).unwrap()})).collect::<Vec<_>>();
            Ok(serde_json::json!({"stage":stage,"status":200,"body":{"deleted":true,"room_id":9001},"deleted":room.deleted(),"claimed":room.destroy_enqueued_at.is_some(),"members":Membership::for_room(conn,9001)?.len(),"revoked":conn.query_row_cached("SELECT COUNT(*)=0 FROM huddle_grants WHERE room_id=9001 AND revoked_at IS NULL",[],|r|r.get::<_,bool>(0))?,"stream_ended":if stage {Some(conn.query_row_cached("SELECT ended_at IS NOT NULL FROM streams WHERE id=40",[],|r|r.get::<_,bool>(0))?)} else {None},"audits":audits}))
        }).await.unwrap();
        assert_eq!(state, *case);
    }
}
