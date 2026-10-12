//! File templates through the real SPA API and seeded transaction/event machinery.
use super::admin_tests::{app, dump, get, parse, settle, write};
use crate::controllers::presenters::test_support::{DAVID, KEVIN, Req};
use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::{Membership, Room};
use serde_json::{Value, json};

fn room(kind: &str, name: &str) -> Value {
    json!({"kind":kind,"name":name,"iconName":null,"topic":"A topic","announcement":true,"voiceBitrate":96000,"voiceUserLimit":5,
        "board":if kind == "board" { json!({"tags":[{"name":"Idea","emoji":"💡"},{"name":"Bug","emoji":null}],"tagsRequired":true,"defaultTag":1}) } else { Value::Null }})
}
fn document() -> Value {
    json!({"format":"smartfire-template","version":1,
        "categories":[{"name":"Template archive","rooms":[]},{"name":"Template team","rooms":[room("stage","Template stage"),room("board","Template board")]}],
        "rooms":[room("voice","Template voice"),room("closed","Template private"),room("open","Template text")]})
}
async fn preview(
    b: &mut crate::controllers::presenters::test_support::Browser<'_>,
    file: &Value,
) -> api::TemplatePreview {
    let reply = write(
        b,
        Method::POST,
        "/api/v1/admin/templates/preview",
        json!({"contents":file.to_string()}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    parse(&reply)
}

#[tokio::test]
async fn spa_template_export_round_trip_order_settings_defaults_and_no_secrets() {
    let a = app().await.expect("frozen seeds required");
    let mut b = a.sign_in(DAVID).await;
    let mut file = document();
    file["rooms"][0]["iconName"] = json!(":SMILE:");
    let before = dump(&a).await;
    let planned = preview(&mut b, &file).await;
    assert_eq!(dump(&a).await, before, "preview must write nothing");
    assert!(planned.errors.is_empty(), "{:?}", planned.errors);
    assert_eq!(planned.category_count, 2);
    assert_eq!(planned.room_count, 5);
    let imported = write(
        &mut b,
        Method::POST,
        "/api/v1/admin/templates/import",
        json!({"contents":file.to_string(),"expected":planned.template}),
    )
    .await;
    assert_eq!(imported.status, StatusCode::CREATED, "{}", imported.text());
    let exported = b.send(get("/api/v1/admin/templates/export")).await;
    assert_eq!(exported.status, StatusCode::OK);
    assert_eq!(
        exported.header("content-disposition"),
        Some("attachment; filename=\"smartfire-template-v1.json\"")
    );
    let exported_again = b.send(get("/api/v1/admin/templates/export")).await;
    assert_eq!(exported.body, exported_again.body);
    let structure: api::WorkspaceTemplate = parse(&exported);
    file["rooms"][0]["iconName"] = json!("smile");
    let wanted: api::WorkspaceTemplate = serde_json::from_value(file).unwrap();
    assert_eq!(
        structure
            .categories
            .iter()
            .find(|category| category.name == "Template team"),
        wanted.categories.get(1)
    );
    assert_eq!(
        structure
            .categories
            .iter()
            .filter(|category| category.name.starts_with("Template "))
            .cloned()
            .collect::<Vec<_>>(),
        wanted.categories,
    );
    assert_eq!(
        structure
            .rooms
            .iter()
            .filter(|room| room
                .name
                .as_deref()
                .is_some_and(|n| n.starts_with("Template ")))
            .cloned()
            .collect::<Vec<_>>(),
        wanted.rooms
    );
    fn safe(value: &Value) {
        match value {
            Value::Object(fields) => {
                for (key, value) in fields {
                    assert!(
                        ![
                            "id",
                            "creatorId",
                            "userIds",
                            "memberships",
                            "messages",
                            "threads",
                            "files",
                            "invites",
                            "token",
                            "inboundEmailToken",
                            "webhookUrl",
                            "roles",
                            "integrations",
                            "clientRoomId"
                        ]
                        .contains(&key.as_str()),
                        "export includes {key}"
                    );
                    safe(value);
                }
            }
            Value::Array(values) => {
                for value in values {
                    safe(value);
                }
            }
            _ => (),
        }
    }
    safe(&serde_json::to_value(&structure).unwrap());
    assert!(!exported.text().contains("secret123456"));
    a.booted
        .app
        .db
        .read(|conn| {
            let mut stmt = conn.prepare("SELECT id FROM rooms WHERE name LIKE 'Template %'")?;
            for id in stmt.query_map([], |row| row.get::<_, i64>(0))? {
                let room = Room::find(conn, id?)?;
                assert_eq!(room.creator_id, DAVID);
                let members = Membership::for_room(conn, room.id)?;
                if room.open() {
                    assert!(members.iter().any(|m| m.user_id == KEVIN));
                } else {
                    assert_eq!(members.len(), 1);
                    assert_eq!(members[0].user_id, DAVID);
                }
                if room.stage() {
                    assert_eq!(members[0].stage_role, Some(campfire_db::StageRole::Host));
                }
            }
            Ok(())
        })
        .await
        .unwrap();
    // Export the entire workspace and import the file again, including every room kind.
    let file: Value = parse(&exported);
    let planned = preview(&mut b, &file).await;
    assert!(planned.errors.is_empty(), "{:?}", planned.errors);
    let expected = planned.template.clone().unwrap();
    let reply = write(
        &mut b,
        Method::POST,
        "/api/v1/admin/templates/import",
        json!({"contents":exported.text(),"expected":expected}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let round_trip: api::WorkspaceTemplate =
        parse(&b.send(get("/api/v1/admin/templates/export")).await);
    assert_eq!(
        round_trip
            .categories
            .iter()
            .filter(|c| expected
                .categories
                .iter()
                .any(|wanted| wanted.name == c.name))
            .cloned()
            .collect::<Vec<_>>(),
        expected.categories
    );
    assert_eq!(
        round_trip
            .rooms
            .iter()
            .filter(|r| expected.rooms.iter().any(|wanted| wanted.name == r.name))
            .cloned()
            .collect::<Vec<_>>(),
        expected.rooms
    );
}

#[tokio::test]
async fn spa_template_mid_import_failure_rolls_back_rows_callbacks_and_broadcasts() {
    let a = app().await.expect("frozen seeds required");
    let mut b = a.sign_in(DAVID).await;
    let mut file = document();
    file["rooms"][1]["name"] = json!("Template failure");
    a.booted.app.db.write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER template_failure BEFORE INSERT ON rooms WHEN NEW.name='Template failure' BEGIN SELECT RAISE(ABORT,'test mid-import failure'); END;")?;
        Ok(())
    }).await.unwrap();
    let planned = preview(&mut b, &file).await;
    assert!(planned.errors.is_empty());
    let capture = a.booted.app.cable.capture_every_publication();
    settle(&capture).await;
    let before = dump(&a).await;
    let reply = write(
        &mut b,
        Method::POST,
        "/api/v1/admin/templates/import",
        json!({"contents":file.to_string(),"expected":planned.template}),
    )
    .await;
    assert!(!reply.status.is_success(), "{}", reply.text());
    assert_eq!(dump(&a).await, before);
    assert!(settle(&capture).await.is_empty());
}

#[tokio::test]
async fn spa_template_admin_only_and_validation_write_nothing() {
    let a = app().await.expect("frozen seeds required");
    let mut member = a.sign_in(KEVIN).await;
    assert_eq!(
        member
            .send(get("/api/v1/admin/templates/export"))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    for path in ["preview", "import"] {
        assert_eq!(
            write(
                &mut member,
                Method::POST,
                &format!("/api/v1/admin/templates/{path}"),
                json!({})
            )
            .await
            .status,
            StatusCode::FORBIDDEN
        );
    }
    let mut admin = a.sign_in(DAVID).await;
    let before = dump(&a).await;
    let mut version = document();
    version["version"] = json!(2);
    let mut kind = document();
    kind["rooms"][0]["kind"] = json!("alien");
    let mut ids = document();
    ids["rooms"][0]["creatorId"] = json!(KEVIN);
    let mut limit = document();
    limit["rooms"][0]["voiceUserLimit"] = json!(100);
    for file in [version, kind, ids, limit] {
        let result = preview(&mut admin, &file).await;
        assert!(!result.errors.is_empty());
        let reply = write(
            &mut admin,
            Method::POST,
            "/api/v1/admin/templates/import",
            json!({"contents":file.to_string(),"expected":document()}),
        )
        .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
    }
    for contents in [
        "{".to_string(),
        " ".repeat(campfire_api::templates::FILE_LIMIT + 1),
    ] {
        let reply = write(
            &mut admin,
            Method::POST,
            "/api/v1/admin/templates/preview",
            json!({"contents":contents}),
        )
        .await;
        let preview: api::TemplatePreview = parse(&reply);
        assert!(!preview.errors.is_empty());
        assert!(preview.template.is_none());
    }
    let reply = admin
        .write(
            Req::new(Method::POST, "/api/v1/admin/templates/preview")
                .header("content-type", "application/json")
                .body("{"),
        )
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(dump(&a).await, before);
}

#[tokio::test]
async fn spa_template_conflict_preview_matches_import_and_stale_preview_is_refused() {
    let a = app().await.expect("frozen seeds required");
    let mut b = a.sign_in(DAVID).await;
    let mut file = document();
    file["rooms"][0]["name"] = json!("Repeated");
    file["rooms"][1]["name"] = json!("Repeated");
    let first = preview(&mut b, &file).await;
    assert!(
        first
            .changes
            .iter()
            .any(|c| c.name.as_deref() == Some("Repeated")
                && c.created_name.as_deref() == Some("Repeated (2)"))
    );
    let body = json!({"contents":file.to_string(),"expected":first.template});
    assert_eq!(
        write(
            &mut b,
            Method::POST,
            "/api/v1/admin/templates/import",
            body.clone()
        )
        .await
        .status,
        StatusCode::CREATED
    );
    let before = dump(&a).await;
    let stale = write(&mut b, Method::POST, "/api/v1/admin/templates/import", body).await;
    assert_eq!(stale.status, StatusCode::CONFLICT);
    assert_eq!(dump(&a).await, before);
    let next = preview(&mut b, &file).await;
    assert!(
        next.changes
            .iter()
            .any(|c| c.created_name.as_deref() == Some("Repeated (3)"))
    );
    assert!(
        next.changes
            .iter()
            .any(|c| c.created_name.as_deref() == Some("Repeated (4)"))
    );
}
