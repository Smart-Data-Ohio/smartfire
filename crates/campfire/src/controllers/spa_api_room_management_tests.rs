//! Room management over the frozen app: classic writes and JSON writes leave identical rows,
//! audit records, jobs and publications. Missing seeds fail rather than silently skip.
use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::{CachedStatements, Room, RoomType};
use serde_json::{Value, json};

use super::admin_tests::{
    app, assert_parity, audits, classic, dump, error, get, nothing, parse, write,
};
use crate::controllers::presenters::test_support::{
    BENDER_KEY, Browser, DAVID, JASON, KEVIN, Req, TestApp,
};

const RITA: i64 = 773523954;
const LOU: i64 = 773523958;

#[tokio::test]
async fn spa_api_rooms_creation_replays_without_any_side_effects() {
    let a = app().await.expect("frozen seeds required");
    let mut david = a.sign_in(DAVID).await;
    david.authenticity_token().await;
    let capture = a.booted.app.cable.capture_every_publication();
    for name in ["open", "closed", "board", "voice", "stage"] {
        let key = format!("room-replay-{name}");
        let mut body =
            json!({"type":name,"clientRoomId":key,"name":" Once ","iconName":" :SMILE: "});
        if name != "open" {
            body["userIds"] = json!([KEVIN, RITA]);
        }
        let first = write(&mut david, Method::POST, "/api/v1/rooms", body.clone()).await;
        assert_eq!(first.status, StatusCode::CREATED, "{}", first.text());
        let first: api::RoomMutation = parse(&first);
        assert!(!super::admin_tests::settle(&capture).await.is_empty());
        let before = dump(&a).await;
        body["iconName"] = json!("smile");
        body["clientRoomId"] = json!(format!(" {key} "));
        if name != "open" {
            body["userIds"] = if name == "stage" {
                json!([RITA, DAVID, KEVIN, KEVIN])
            } else {
                json!([RITA, KEVIN, KEVIN])
            };
        }
        let replay = write(&mut david, Method::POST, "/api/v1/rooms", body).await;
        assert_eq!(replay.status, StatusCode::OK, "{}", replay.text());
        assert_eq!(parse::<api::RoomMutation>(&replay), first);
        assert_eq!(
            dump(&a).await,
            before,
            "rooms, memberships, audits and jobs"
        );
        assert!(super::admin_tests::settle(&capture).await.is_empty());
    }
}

#[tokio::test]
async fn spa_api_rooms_creation_rejects_changed_parameters_without_side_effects() {
    let a = app().await.expect("frozen seeds required");
    let mut david = a.sign_in(DAVID).await;
    david.authenticity_token().await;
    let capture = a.booted.app.cable.capture_every_publication();
    for name in ["open", "closed", "board", "voice", "stage"] {
        let mut body = json!({"type":name,"clientRoomId":format!("room-conflict-{name}"),"name":"Once","iconName":"smile"});
        if name != "open" {
            body["userIds"] = json!([DAVID, KEVIN]);
        }
        let first = write(&mut david, Method::POST, "/api/v1/rooms", body.clone()).await;
        assert_eq!(first.status, StatusCode::CREATED, "{}", first.text());
        let first: api::RoomMutation = parse(&first);
        super::admin_tests::settle(&capture).await;
        let before = dump(&a).await;
        let mut changed_name = body.clone();
        changed_name["name"] = json!("Different");
        let mut changed_icon = body.clone();
        changed_icon["iconName"] = json!("fire");
        let mut invalid_icon = body.clone();
        invalid_icon["iconName"] = json!("missing_icon_s8");
        let mut changed_type = body.clone();
        if name == "open" {
            changed_type["type"] = json!("closed");
            changed_type["userIds"] = json!([DAVID, KEVIN]);
        } else {
            changed_type["type"] = json!("open");
            changed_type.as_object_mut().unwrap().remove("userIds");
        }
        let mut retries = vec![changed_name, changed_icon, invalid_icon, changed_type];
        if name != "open" {
            let mut changed_members = body.clone();
            changed_members["userIds"] = json!([DAVID, JASON]);
            retries.push(changed_members);
        }
        for retry in retries {
            let reply = write(&mut david, Method::POST, "/api/v1/rooms", retry).await;
            assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.text());
            assert_eq!(
                error(&reply),
                json!({"_tag":"Conflict","message":"clientRoomId was already used with different room parameters"})
            );
            assert_eq!(
                dump(&a).await,
                before,
                "{name}: rooms, memberships, audits and jobs"
            );
            assert!(super::admin_tests::settle(&capture).await.is_empty());
        }
        let replay = write(&mut david, Method::POST, "/api/v1/rooms", body).await;
        assert_eq!(replay.status, StatusCode::OK, "{}", replay.text());
        assert_eq!(parse::<api::RoomMutation>(&replay), first);
    }
}

#[tokio::test]
async fn spa_api_rooms_concurrent_creation_rejects_changed_parameters() {
    let a = app().await.expect("frozen seeds required");
    let mut first = a.sign_in(DAVID).await;
    let mut second = a.sign_in(DAVID).await;
    first.authenticity_token().await;
    second.authenticity_token().await;
    let capture = a.booted.app.cable.capture_every_publication();
    let body = json!({"type":"closed","clientRoomId":"room-conflict-control","name":"Once","userIds":[DAVID,KEVIN]});
    let before = dump(&a).await;
    let control = write(&mut first, Method::POST, "/api/v1/rooms", body.clone()).await;
    assert_eq!(control.status, StatusCode::CREATED, "{}", control.text());
    let after = dump(&a).await;
    let frames = super::admin_tests::settle(&capture).await;
    let mut one = body;
    one["clientRoomId"] = json!("room-conflict-race");
    let mut two = one.clone();
    two["name"] = json!("Different");
    campfire_api::test_hooks::hold_after_duplicate_check("room-conflict-race", 2);
    let (one, two) = tokio::join!(
        write(&mut first, Method::POST, "/api/v1/rooms", one),
        write(&mut second, Method::POST, "/api/v1/rooms", two),
    );
    let mut statuses = [one.status, two.status];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::CREATED, StatusCode::CONFLICT],
        "{} / {}",
        one.text(),
        two.text()
    );
    let refused = if one.status == StatusCode::CONFLICT {
        &one
    } else {
        &two
    };
    assert_eq!(error(refused)["_tag"], "Conflict");
    let raced = dump(&a).await;
    for table in ["rooms", "memberships", "audit_logs", "background_jobs"] {
        let count = |rows: &Value| rows[table].as_array().unwrap().len();
        assert_eq!(
            count(&raced) - count(&after),
            count(&after) - count(&before),
            "{table}"
        );
    }
    assert_eq!(
        super::admin_tests::settle(&capture).await.len(),
        frames.len()
    );
}

#[tokio::test]
async fn spa_api_rooms_creation_keys_are_viewer_scoped() {
    let a = app().await.expect("frozen seeds required");
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let body = json!({"type":"closed","clientRoomId":"shared-room-key","name":"Once","userIds":[DAVID,KEVIN]});
    let first = write(&mut david, Method::POST, "/api/v1/rooms", body.clone()).await;
    let second = write(&mut kevin, Method::POST, "/api/v1/rooms", body.clone()).await;
    assert_eq!(first.status, StatusCode::CREATED, "{}", first.text());
    assert_eq!(second.status, StatusCode::CREATED, "{}", second.text());
    let first: api::RoomMutation = parse(&first);
    let second: api::RoomMutation = parse(&second);
    assert_ne!(first.room.id, second.room.id);
    assert_eq!(first.room.creator_id, DAVID);
    assert_eq!(second.room.creator_id, KEVIN);
    for (viewer, expected) in [(&mut david, first), (&mut kevin, second)] {
        let replay = write(viewer, Method::POST, "/api/v1/rooms", body.clone()).await;
        assert_eq!(replay.status, StatusCode::OK);
        assert_eq!(parse::<api::RoomMutation>(&replay), expected);
    }
}

#[tokio::test]
async fn spa_api_rooms_concurrent_creation_retries_create_once() {
    let a = app().await.expect("frozen seeds required");
    let mut first = a.sign_in(DAVID).await;
    let mut second = a.sign_in(DAVID).await;
    first.authenticity_token().await;
    second.authenticity_token().await;
    let capture = a.booted.app.cable.capture_every_publication();
    for name in ["open", "closed", "board", "voice", "stage"] {
        let mut body =
            json!({"type":name,"clientRoomId":format!("room-control-{name}"),"name":"Once"});
        if name != "open" {
            body["userIds"] = json!([DAVID, KEVIN]);
        }
        let before = dump(&a).await;
        let control = write(&mut first, Method::POST, "/api/v1/rooms", body.clone()).await;
        assert_eq!(control.status, StatusCode::CREATED, "{}", control.text());
        let after = dump(&a).await;
        let frames = super::admin_tests::settle(&capture).await;
        let key = format!("room-race-{name}");
        body["clientRoomId"] = json!(key);
        campfire_api::test_hooks::hold_after_duplicate_check(&key, 2);
        let (one, two) = tokio::join!(
            write(&mut first, Method::POST, "/api/v1/rooms", body.clone()),
            write(&mut second, Method::POST, "/api/v1/rooms", body),
        );
        let mut statuses = [one.status, two.status];
        statuses.sort();
        assert_eq!(
            statuses,
            [StatusCode::OK, StatusCode::CREATED],
            "{} / {}",
            one.text(),
            two.text()
        );
        assert_eq!(
            parse::<api::RoomMutation>(&one).room.id,
            parse::<api::RoomMutation>(&two).room.id
        );
        let raced = dump(&a).await;
        for table in ["rooms", "memberships", "audit_logs", "background_jobs"] {
            let count = |rows: &Value| rows[table].as_array().unwrap().len();
            assert_eq!(
                count(&raced) - count(&after),
                count(&after) - count(&before),
                "{name}: {table}"
            );
        }
        assert_eq!(
            super::admin_tests::settle(&capture).await.len(),
            frames.len(),
            "{name}: broadcasts"
        );
    }
}

#[tokio::test]
async fn spa_api_rooms_creation_requires_a_nonblank_key() {
    let a = app().await.expect("frozen seeds required");
    let mut david = a.sign_in(DAVID).await;
    david.authenticity_token().await;
    let before = dump(&a).await;
    let capture = a.booted.app.cable.capture_every_publication();
    for key in [None, Some(json!("")), Some(json!(" \t\n "))] {
        let mut body = json!({"type":"open","name":"Never"});
        if let Some(key) = key {
            body["clientRoomId"] = key;
        }
        let reply = write(&mut david, Method::POST, "/api/v1/rooms", body).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error(&reply)["_tag"], "Validation");
        assert_eq!(
            error(&reply)["fields"]["clientRoomId"],
            json!(["can't be blank"])
        );
    }
    assert_eq!(dump(&a).await, before);
    assert!(super::admin_tests::settle(&capture).await.is_empty());
}

fn kind(name: &str) -> RoomType {
    match name {
        "open" => RoomType::Open,
        "closed" => RoomType::Closed,
        "direct" => RoomType::Direct,
        "voice" => RoomType::Voice,
        "stage" => RoomType::Stage,
        "board" => RoomType::Board,
        _ => unreachable!(),
    }
}

fn namespace(name: &str) -> &str {
    match name {
        "open" => "opens",
        "closed" => "closeds",
        "direct" => "directs",
        "voice" => "voices",
        "stage" => "stages",
        "board" => "boards",
        _ => unreachable!(),
    }
}

async fn room(a: &TestApp, name: &str) -> Room {
    let kind = kind(name);
    a.db()
        .write(move |tx| Room::create_for(tx, kind, Some("Before"), DAVID, &[DAVID, JASON, KEVIN]))
        .await
        .unwrap()
}

async fn created(b: &mut Browser<'_>, name: &str, ids: &[i64]) -> api::RoomMutation {
    let mut body = json!({"type":name,"name":"API room","iconName":"smile"});
    body["clientRoomId"] = json!(uuid::Uuid::new_v4().to_string());
    if name != "open" {
        body["userIds"] = json!(ids);
    }
    let reply = write(b, Method::POST, "/api/v1/rooms", body).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    parse(&reply)
}

async fn create_parity(name: &str) {
    let outcome = assert_parity(
        nothing,
        async |b, _| {
            let path = format!("/rooms/{}", namespace(name));
            let mut fields = vec![("room[name]", "API room"), ("room[icon_name]", "smile")];
            let david = DAVID.to_string();
            let kevin = KEVIN.to_string();
            let rita = RITA.to_string();
            if name != "open" {
                fields.extend([
                    ("user_ids[]", david.as_str()),
                    ("user_ids[]", &kevin),
                    ("user_ids[]", &rita),
                ]);
            }
            classic(b, Method::POST, &path, &fields).await;
        },
        async |b, _| {
            let result = created(b, name, &[DAVID, KEVIN, RITA]).await;
            assert_eq!(serde_json::to_value(result.room.kind).unwrap(), name);
            assert_eq!(result.room.name.as_deref(), Some("API room"));
            assert_eq!(result.room.icon_name.as_deref(), Some("smile"));
            assert!(result.detail.is_some());
            assert!(result.row.is_some());
        },
    )
    .await
    .expect("frozen seeds required");
    assert!(audits(&outcome).contains("room.create"));
    assert!(
        !outcome.frames.is_empty(),
        "classic row/header publications"
    );
}

async fn update_parity(name: &str) {
    let outcome = assert_parity(
        async |a, _| json!(room(a, name).await.id),
        async |b, id| {
            let path = format!("/rooms/{}/{}", namespace(name), id.as_i64().unwrap());
            let david = DAVID.to_string();
            let jason = JASON.to_string();
            let rita = RITA.to_string();
            let mut fields = vec![("room[name]", "After"), ("room[icon_name]", "github")];
            if name != "open" {
                fields.extend([
                    ("user_ids[]", david.as_str()),
                    ("user_ids[]", &jason),
                    ("user_ids[]", &rita),
                ]);
            }
            classic(b, Method::PATCH, &path, &fields).await;
        },
        async |b, id| {
            let mut body = json!({"type":name,"name":"After","iconName":"github"});
            if name != "open" {
                body["userIds"] = json!([DAVID, JASON, RITA]);
            }
            let reply = write(b, Method::PATCH, &format!("/api/v1/rooms/{id}"), body).await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
            let result: api::RoomMutation = parse(&reply);
            assert_eq!(result.room.name.as_deref(), Some("After"));
            if name != "open" {
                assert_eq!(result.detail.unwrap().member_count, 3);
            }
        },
    )
    .await
    .expect("frozen seeds required");
    assert!(
        !outcome.frames.is_empty(),
        "classic row/header publications"
    );
    if name != "open" {
        assert!(audits(&outcome).contains("room.membership"));
    }
}

async fn delete_parity(name: &str) {
    let outcome = assert_parity(
        async |a, _| json!(room(a, name).await.id),
        async |b, id| {
            let path = if name == "direct" {
                format!("/rooms/directs/{id}")
            } else {
                format!("/rooms/{id}")
            };
            classic(b, Method::DELETE, &path, &[]).await;
        },
        async |b, id| {
            let reply = write(b, Method::DELETE, &format!("/api/v1/rooms/{id}"), json!({})).await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
            let result: api::RoomRemoved = parse(&reply);
            assert_eq!(result.room_id, id.as_i64().unwrap());
            assert!(result.deleted);
        },
    )
    .await
    .expect("frozen seeds required");
    assert!(audits(&outcome).contains("room.destroy"));
}

macro_rules! parity_tests {
    ($create:ident, $update:ident, $delete:ident, $name:literal) => {
        #[tokio::test]
        async fn $create() {
            create_parity($name).await;
        }
        #[tokio::test]
        async fn $update() {
            update_parity($name).await;
        }
        #[tokio::test]
        async fn $delete() {
            delete_parity($name).await;
        }
    };
}
parity_tests!(
    spa_api_rooms_open_create_matches_classic,
    spa_api_rooms_open_update_matches_classic,
    spa_api_rooms_open_delete_matches_classic,
    "open"
);
parity_tests!(
    spa_api_rooms_closed_create_matches_classic,
    spa_api_rooms_closed_update_matches_classic,
    spa_api_rooms_closed_delete_matches_classic,
    "closed"
);
parity_tests!(
    spa_api_rooms_board_create_matches_classic,
    spa_api_rooms_board_update_matches_classic,
    spa_api_rooms_board_delete_matches_classic,
    "board"
);
parity_tests!(
    spa_api_rooms_voice_create_matches_classic,
    spa_api_rooms_voice_update_matches_classic,
    spa_api_rooms_voice_delete_matches_classic,
    "voice"
);
parity_tests!(
    spa_api_rooms_stage_create_matches_classic,
    spa_api_rooms_stage_update_matches_classic,
    spa_api_rooms_stage_delete_matches_classic,
    "stage"
);

#[tokio::test]
async fn spa_api_rooms_direct_create_matches_classic() {
    let outcome = assert_parity(
        nothing,
        async |b, _| {
            classic(
                b,
                Method::POST,
                "/rooms/directs",
                &[
                    ("user_ids[]", &KEVIN.to_string()),
                    ("user_ids[]", &LOU.to_string()),
                    ("user_ids[]", &RITA.to_string()),
                ],
            )
            .await;
        },
        async |b, _| {
            let reply = write(
                b,
                Method::POST,
                "/api/v1/directs",
                json!({"userIds":[KEVIN,LOU,RITA]}),
            )
            .await;
            assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
            let row: api::SidebarRow = parse(&reply);
            assert_eq!(row.direct_member_ids.len(), 2);
            assert!(!row.direct_member_ids.contains(&RITA));
        },
    )
    .await
    .expect("frozen seeds required");
    assert!(audits(&outcome).contains("room.create"));
}

#[tokio::test]
async fn spa_api_rooms_direct_rename_and_add_matches_classic() {
    assert_parity(
        async |a, _| json!(room(a, "direct").await.id),
        async |b, id| {
            classic(
                b,
                Method::PATCH,
                &format!("/rooms/directs/{id}"),
                &[("room[name]", "  Group  ")],
            )
            .await;
            classic(
                b,
                Method::POST,
                &format!("/rooms/directs/{id}/add_members"),
                &[
                    ("user_ids[]", &LOU.to_string()),
                    ("user_ids[]", &RITA.to_string()),
                ],
            )
            .await;
        },
        async |b, id| {
            let reply = write(
                b,
                Method::PATCH,
                &format!("/api/v1/directs/{id}"),
                json!({"name":"  Group  "}),
            )
            .await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
            assert_eq!(
                Some(parse::<api::RoomDetail>(&reply).display_name.as_str()),
                Some("Group")
            );
            let reply = write(
                b,
                Method::POST,
                &format!("/api/v1/directs/{id}/members"),
                json!({"userIds":[LOU,RITA]}),
            )
            .await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
            assert_eq!(parse::<api::RoomDetail>(&reply).member_count, 4);
        },
    )
    .await
    .expect("frozen seeds required");
}

#[tokio::test]
async fn spa_api_rooms_direct_delete_matches_classic() {
    delete_parity("direct").await;
}

#[tokio::test]
async fn spa_api_rooms_direct_leave_matches_classic() {
    let outcome = assert_parity(
        async |a, _| json!(room(a, "direct").await.id),
        async |b, id| {
            classic(
                b,
                Method::DELETE,
                &format!("/rooms/directs/{id}/leave"),
                &[],
            )
            .await
        },
        async |b, id| {
            let reply = write(
                b,
                Method::DELETE,
                &format!("/api/v1/rooms/{id}/membership"),
                json!({}),
            )
            .await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
            let result: api::RoomLeft = parse(&reply);
            assert!(!result.deleted);
        },
    )
    .await
    .expect("frozen seeds required");
    assert!(audits(&outcome).contains("room.membership"));
}

#[tokio::test]
async fn spa_api_rooms_form_defaults_and_member_permissions_match_classic() {
    let a = app().await.expect("frozen seeds required");
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    for (name, default) in [
        ("open", "New room"),
        ("closed", "New room"),
        ("board", "New board"),
        ("voice", "New voice channel"),
        ("stage", "New stage channel"),
        ("direct", ""),
    ] {
        let reply = david
            .send(get(&format!("/api/v1/rooms/new?type={name}")))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let form: api::RoomForm = parse(&reply);
        assert_eq!(form.display_name, default);
        assert!(form.can_submit && !form.can_delete && !form.can_leave);
        assert_eq!(form.allowed_types.len(), 6);
        assert!(!form.candidate_ids.contains(&RITA));
        if name == "direct" {
            assert!(!form.candidate_ids.contains(&DAVID));
        } else {
            assert!(form.user_ids.contains(&DAVID));
        }
        if name == "stage" {
            assert_eq!(form.stage_roles[0].role, api::StageRole::Host);
        }
        let room = room(&a, name).await;
        let path = format!("/api/v1/rooms/{}/edit", room.id);
        let form: api::RoomForm = parse(&kevin.send(get(&path)).await);
        assert_eq!(form.can_submit, name == "direct");
        assert!(!form.can_delete);
        assert_eq!(form.can_leave, name == "direct");
        assert!(form.candidate_ids.contains(&LOU));
        if name != "direct" {
            let body = if name == "open" {
                json!({"type":name,"name":"Denied"})
            } else {
                json!({"type":name,"name":"Denied","userIds":[DAVID,KEVIN]})
            };
            let denied = write(
                &mut kevin,
                Method::PATCH,
                &format!("/api/v1/rooms/{}", room.id),
                body,
            )
            .await;
            assert_eq!(denied.status, StatusCode::FORBIDDEN);
            assert_eq!(error(&denied)["_tag"], "Forbidden");
            let denied = write(
                &mut kevin,
                Method::DELETE,
                &format!("/api/v1/rooms/{}", room.id),
                json!({}),
            )
            .await;
            assert_eq!(denied.status, StatusCode::FORBIDDEN);
        }
    }
}

#[tokio::test]
async fn spa_api_rooms_creation_restriction_and_access_match_classic() {
    let a = app().await.expect("frozen seeds required");
    a.db()
        .write(|tx| {
            tx.conn().execute_cached(
                "UPDATE accounts SET settings=?",
                [json!({"restrict_room_creation_to_administrators":true}).to_string()],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut kevin = a.sign_in(KEVIN).await;
    for name in ["open", "closed", "board", "voice", "stage"] {
        let reply = kevin
            .send(get(&format!("/api/v1/rooms/new?type={name}")))
            .await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN);
        let denied = write(
            &mut kevin,
            Method::POST,
            "/api/v1/rooms",
            json!({"type":name,"name":"Denied","userIds":[KEVIN]}),
        )
        .await;
        assert_eq!(denied.status, StatusCode::FORBIDDEN);
    }
    let form: api::RoomForm = parse(&kevin.send(get("/api/v1/rooms/new?type=direct")).await);
    assert_eq!(form.allowed_types, [api::RoomKind::Direct]);
    let reply = write(
        &mut kevin,
        Method::POST,
        "/api/v1/directs",
        json!({"userIds":[LOU]}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED);
    let mut david = a.sign_in(DAVID).await;
    for name in ["open", "closed", "board", "voice", "stage"] {
        created(&mut david, name, &[DAVID, KEVIN]).await;
    }
    let other = a
        .db()
        .write(|tx| Room::create_for(tx, RoomType::Closed, Some("Other"), KEVIN, &[KEVIN]))
        .await
        .unwrap();
    let before = dump(&a).await;
    for path in [
        format!("/api/v1/rooms/{}/edit", other.id),
        format!("/api/v1/rooms/{}", other.id),
    ] {
        let reply = david.send(get(&path)).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
    }
    let denied = write(
        &mut david,
        Method::DELETE,
        &format!("/api/v1/rooms/{}", other.id),
        json!({}),
    )
    .await;
    assert_eq!(denied.status, StatusCode::NOT_FOUND);
    assert_eq!(dump(&a).await, before);
}

#[tokio::test]
async fn spa_api_rooms_icon_errors_match_classic_without_writes() {
    for name in ["open", "closed", "board", "voice", "stage"] {
        let outcome = assert_parity(
            async |a, _| json!(room(a, name).await.id),
            async |b, id| {
                for (method, path) in [
                    (Method::POST, format!("/rooms/{}", namespace(name))),
                    (Method::PATCH, format!("/rooms/{}/{id}", namespace(name))),
                ] {
                    let reply = b
                        .write(Req::new(method, &path).form(&[
                            ("room[name]", "Attempted"),
                            ("room[icon_name]", "not_an_icon_s8"),
                            ("user_ids[]", &DAVID.to_string()),
                        ]))
                        .await;
                    assert_eq!(
                        reply.status,
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "{}",
                        reply.text()
                    );
                    assert!(reply.text().contains("Icon name is not a known icon"));
                }
            },
            async |b, id| {
                let mut body = json!({"type":name,"name":"Attempted","iconName":"not_an_icon_s8"});
                if name != "open" {
                    body["userIds"] = json!([DAVID]);
                }
                for (method, path) in [
                    (Method::POST, "/api/v1/rooms".to_string()),
                    (Method::PATCH, format!("/api/v1/rooms/{id}")),
                ] {
                    let mut request = body.clone();
                    if method == Method::POST {
                        request["clientRoomId"] = json!(uuid::Uuid::new_v4().to_string());
                    }
                    let reply = write(b, method, &path, request).await;
                    assert_eq!(
                        reply.status,
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "{}",
                        reply.text()
                    );
                    let failure = error(&reply);
                    assert_eq!(failure["_tag"], "Validation");
                    assert_eq!(failure["message"], "Icon name is not a known icon");
                    assert!(failure["fields"]["iconName"].is_array());
                }
            },
        )
        .await
        .expect("frozen seeds required");
        assert_eq!(outcome.before, outcome.rows);
        assert!(outcome.frames.is_empty());
    }
}

#[tokio::test]
async fn spa_api_rooms_stage_host_guard_matches_classic_atomically() {
    let outcome = assert_parity(
        async |a, _| json!(room(a, "stage").await.id),
        async |b, id| {
            let reply = b
                .write(
                    Req::new(Method::PATCH, &format!("/rooms/stages/{id}")).form(&[
                        ("room[name]", "Stranded"),
                        ("user_ids[]", &JASON.to_string()),
                    ]),
                )
                .await;
            assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert!(
                reply
                    .text()
                    .contains("Promote another host before removing David")
            );
        },
        async |b, id| {
            let reply = write(
                b,
                Method::PATCH,
                &format!("/api/v1/rooms/{id}"),
                json!({"type":"stage","name":"Stranded","userIds":[JASON]}),
            )
            .await;
            assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
            let failure = error(&reply);
            assert_eq!(
                failure["message"],
                "Promote another host before removing David"
            );
            assert!(failure["fields"]["userIds"].is_array());
        },
    )
    .await
    .expect("frozen seeds required");
    assert_eq!(outcome.before, outcome.rows);
    assert!(outcome.frames.is_empty());
}

#[tokio::test]
async fn spa_api_rooms_self_removal_and_creator_omission_still_succeed() {
    let a = app().await.expect("frozen seeds required");
    let mut david = a.sign_in(DAVID).await;
    for name in ["closed", "board", "voice"] {
        let result = created(&mut david, name, &[KEVIN]).await;
        assert!(result.detail.is_none() && result.row.is_none());
        assert_eq!(
            david
                .send(get(&format!("/api/v1/rooms/{}/edit", result.room.id)))
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
    let stage = created(&mut david, "stage", &[KEVIN]).await;
    assert!(stage.detail.is_some(), "stage always grants creator host");
    for name in ["closed", "board", "voice", "stage"] {
        let room = room(&a, name).await;
        let ids = if name == "stage" { vec![] } else { vec![KEVIN] };
        let reply = write(
            &mut david,
            Method::PATCH,
            &format!("/api/v1/rooms/{}", room.id),
            json!({"type":name,"name":"Removed","userIds":ids}),
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let result: api::RoomMutation = parse(&reply);
        assert!(result.detail.is_none() && result.row.is_none());
        assert_eq!(
            david
                .send(get(&format!("/api/v1/rooms/{}/edit", room.id)))
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
}

#[tokio::test]
async fn spa_api_rooms_nullable_patches_and_only_open_closed_conversion() {
    let a = app().await.expect("frozen seeds required");
    let mut david = a.sign_in(DAVID).await;
    for name in ["open", "closed", "board", "voice", "stage"] {
        let room = room(&a, name).await;
        let path = format!("/api/v1/rooms/{}", room.id);
        let mut body = json!({"type":name});
        if name != "open" {
            body["userIds"] = json!([DAVID, JASON, KEVIN]);
        }
        let reply = write(&mut david, Method::PATCH, &path, body.clone()).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        assert_eq!(
            parse::<api::RoomMutation>(&reply).room.name.as_deref(),
            Some("Before")
        );
        body["name"] = Value::Null;
        body["iconName"] = Value::Null;
        let reply = write(&mut david, Method::PATCH, &path, body).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(parse::<api::RoomMutation>(&reply).room.name, None);
        if !matches!(name, "open" | "closed") {
            let before = dump(&a).await;
            let reply = write(
                &mut david,
                Method::PATCH,
                &path,
                json!({"type":"open","name":"Wrong"}),
            )
            .await;
            assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(
                error(&reply)["fields"]["type"],
                json!(["cannot convert this room to that type"])
            );
            assert_eq!(dump(&a).await, before);
        }
    }
    let room = room(&a, "closed").await;
    let path = format!("/api/v1/rooms/{}", room.id);
    let reply = write(&mut david, Method::PATCH, &path, json!({"type":"open"})).await;
    assert_eq!(
        parse::<api::RoomMutation>(&reply).room.kind,
        api::RoomKind::Open
    );
    let reply = write(
        &mut david,
        Method::PATCH,
        &path,
        json!({"type":"closed","userIds":[DAVID,KEVIN]}),
    )
    .await;
    let result: api::RoomMutation = parse(&reply);
    assert_eq!(result.room.kind, api::RoomKind::Closed);
    assert_eq!(result.detail.unwrap().member_count, 2);
}

#[tokio::test]
async fn spa_api_rooms_direct_delete_matches_visible_form_permissions() {
    let a = app().await.expect("frozen seeds required");
    let pair = a
        .db()
        .write(|tx| Room::create_for(tx, RoomType::Direct, None, DAVID, &[DAVID, KEVIN]))
        .await
        .unwrap();
    let mut kevin = a.sign_in(KEVIN).await;
    let form: api::RoomForm = parse(
        &kevin
            .send(get(&format!("/api/v1/rooms/{}/edit", pair.id)))
            .await,
    );
    assert!(form.can_delete && form.can_leave && !form.can_submit);
    let reply = write(
        &mut kevin,
        Method::DELETE,
        &format!("/api/v1/rooms/{}", pair.id),
        json!({}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK);
    let group = room(&a, "direct").await;
    let before = dump(&a).await;
    let reply = write(
        &mut kevin,
        Method::DELETE,
        &format!("/api/v1/rooms/{}", group.id),
        json!({}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(dump(&a).await, before);
}

#[tokio::test]
async fn spa_api_rooms_reject_non_session_auth_and_invalid_contract_inputs() {
    let a = app().await.expect("frozen seeds required");
    let mut anonymous = a.anonymous();
    assert_eq!(
        anonymous
            .send(get("/api/v1/rooms/new?type=open"))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let bot = anonymous
        .send(get(&format!(
            "/api/v1/rooms/new?type=open&bot_key={BENDER_KEY}"
        )))
        .await;
    assert_eq!(bot.status, StatusCode::FORBIDDEN);
    let mut david = a.sign_in(DAVID).await;
    let before = dump(&a).await;
    for body in [
        json!({"type":"direct","clientRoomId":"invalid-room","userIds":[KEVIN]}),
        json!({"type":"closed","clientRoomId":"invalid-room","userIds":["773523952"]}),
        json!({"type":"open","clientRoomId":"invalid-room","userIds":[DAVID]}),
        json!({"type":"stage","clientRoomId":"invalid-room","userIds":[],"stageRoles":[]}),
    ] {
        let reply = write(&mut david, Method::POST, "/api/v1/rooms", body).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert_eq!(error(&reply)["_tag"], "Validation");
    }
    assert_eq!(dump(&a).await, before);
}

#[tokio::test]
async fn spa_api_rooms_non_admin_creators_can_create_edit_and_delete_each_type() {
    let a = app().await.expect("frozen seeds required");
    let mut kevin = a.sign_in(KEVIN).await;
    for name in ["open", "closed", "board", "voice", "stage"] {
        let result = created(&mut kevin, name, &[KEVIN, JASON]).await;
        let id = result.room.id;
        let form: api::RoomForm =
            parse(&kevin.send(get(&format!("/api/v1/rooms/{id}/edit"))).await);
        assert!(form.can_submit && form.can_delete);
        let mut body = json!({"type": name, "name": "Creator edit"});
        if name != "open" {
            body["userIds"] = json!([KEVIN, JASON]);
        }
        let reply = write(
            &mut kevin,
            Method::PATCH,
            &format!("/api/v1/rooms/{id}"),
            body,
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        assert_eq!(
            parse::<api::RoomMutation>(&reply).room.name.as_deref(),
            Some("Creator edit")
        );
        let reply = write(
            &mut kevin,
            Method::DELETE,
            &format!("/api/v1/rooms/{id}"),
            json!({}),
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    }
}

#[tokio::test]
async fn spa_api_rooms_direct_name_validation_matches_classic_unicode_length() {
    let outcome = assert_parity(
        async |a, _| json!(room(a, "direct").await.id),
        async |b, id| {
            let name = "界".repeat(101);
            let reply = b
                .write(
                    Req::new(Method::PATCH, &format!("/rooms/directs/{id}"))
                        .form(&[("room[name]", &name)]),
                )
                .await;
            assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        },
        async |b, id| {
            let reply = write(
                b,
                Method::PATCH,
                &format!("/api/v1/directs/{id}"),
                json!({"name":"界".repeat(101)}),
            )
            .await;
            assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(
                error(&reply)["message"],
                "Name is too long (maximum is 100 characters)"
            );
            assert!(error(&reply)["fields"]["name"].is_array());
        },
    )
    .await
    .expect("frozen seeds required");
    assert_eq!(outcome.before, outcome.rows);
    assert!(outcome.frames.is_empty());
}

#[tokio::test]
async fn spa_api_rooms_management_changes_publish_refreshes_to_the_real_sync_socket() {
    use super::api_tests::{Sync, serve};
    let a = app().await.expect("frozen seeds required");
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    sync.welcome().await;
    for name in ["open", "closed", "board", "voice", "stage"] {
        let result = created(&mut david, name, &[DAVID, KEVIN]).await;
        let id = result.room.id;
        let event = sync.until(|event| matches!(&event.payload, api::SyncPayload::SidebarRowUpserted(row) if row.room.id == id), |_| false).await;
        let api::SyncPayload::SidebarRowUpserted(row) = event.payload else {
            unreachable!()
        };
        assert_eq!(row.refresh_room, Some(true));
        assert_eq!(row.room.name.as_deref(), Some("API room"));
        let mut body = json!({"type":name,"name":"Live edit"});
        if name != "open" {
            body["userIds"] = json!([DAVID, JASON]);
        }
        let reply = write(
            &mut david,
            Method::PATCH,
            &format!("/api/v1/rooms/{id}"),
            body,
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let event=sync.until(|event| matches!(&event.payload, api::SyncPayload::SidebarRowUpserted(row) if row.room.id == id && row.room.name.as_deref() == Some("Live edit")), |_| false).await;
        let api::SyncPayload::SidebarRowUpserted(row) = event.payload else {
            unreachable!()
        };
        assert_eq!(row.refresh_room, Some(true));
        let reply = write(
            &mut david,
            Method::DELETE,
            &format!("/api/v1/rooms/{id}"),
            json!({}),
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK);
        let event=sync.until(|event| matches!(&event.payload, api::SyncPayload::SidebarRowRemoved(row) if row.room_id == id), |_| false).await;
        let api::SyncPayload::SidebarRowRemoved(row) = event.payload else {
            unreachable!()
        };
        assert_eq!(row.refresh_room, Some(true));
    }
    server.abort();
}
