use super::*;
use crate::controllers::presenters::test_support::*;
use askama::Template;
use axum::http::Method;
use serde_json::Value;
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../../../vectors/users_icons.json")).unwrap()
}
fn bytes(file: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vectors/workspace_icons")
            .join(file),
    )
    .unwrap()
}
async fn upload(app: &TestApp, name: &str, title: &str, file: &str) -> Reply {
    app.david()
        .write(Req::new(Method::POST, "/account/icons").multipart(
            &[
                ("workspace_icon[name]", name),
                ("workspace_icon[title]", title),
            ],
            (
                "workspace_icon[image]",
                file,
                if file.ends_with("svg") {
                    "image/svg+xml"
                } else {
                    "image/png"
                },
                &bytes(file),
            ),
        ))
        .await
}
#[tokio::test]
async fn all_committed_media_and_field_validation_cases_match_rails() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    for case in vectors()["cases"].as_array().unwrap() {
        if case["input"]["duplicate"] == true {
            upload(&app, "acme", "Acme Corp", "clean.svg").await;
        }
        let file = case["input"]
            .get("file")
            .map_or(Some("clean.svg"), |f| f.as_str());
        let facts = if let Some(file) = file {
            let staged = app
                .booted
                .app
                .storage
                .stage_bytes(&bytes(file), campfire_storage::Filename::new(file), None)
                .unwrap();
            let assignment = Assignment::Create(staged);
            // Same storage fact calculation as the HTTP path, independent of untracked files.
            let Assignment::Create(staged) = &assignment else {
                unreachable!()
            };
            let blob = staged.blob();
            let content_type = blob.content_type.clone().unwrap_or_default();
            let error = campfire_storage::workspace_icon::content_error(
                &app.booted.app.storage.service.path_for(&blob.key),
                &content_type,
            );
            Some(ImageFacts {
                content_type,
                byte_size: blob.byte_size,
                content_error: error,
            })
        } else {
            None
        };
        let icon = NewIcon {
            name: Some(case["input"]["name"].as_str().unwrap_or("acme").into()),
            title: Some(
                case["input"]["title"]
                    .as_str()
                    .unwrap_or("Acme Corp")
                    .into(),
            ),
            creator_id: DAVID,
        }
        .normalized();
        let brand = crate::rich_text::builtin_icon(icon.name.as_deref().unwrap());
        let title = icon.title.clone();
        let name = icon.name.clone();
        let errors = app
            .db()
            .read(move |c| icon.errors(c, brand, facts.as_ref()))
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(errors.full_messages()).unwrap(),
            case["errors"],
            "{}",
            case["input"]
        );
        assert_eq!(name.as_deref(), case["name"].as_str());
        assert_eq!(title.as_deref(), case["title"].as_str());
        assert_eq!(errors.is_empty(), case["valid"].as_bool().unwrap());
    }
}
#[tokio::test]
async fn index_body_and_navigation_match_rails_empty_populated_and_error_pages() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    for case in vectors()["pages"].as_array().unwrap() {
        let icons = if case["name"] == "empty" {
            vec![]
        } else {
            vec![
                views::Icon {
                    id: 11001,
                    name: "acme".into(),
                    title: "Acme Corp".into(),
                    creator_name: "David".into(),
                },
                views::Icon {
                    id: 11002,
                    name: "zeta".into(),
                    title: "Zeta <&\"".into(),
                    creator_name: "David".into(),
                },
            ]
        };
        let form = if case["name"] == "invalid" {
            views::Form {
                name: Some("openai".into()),
                title: Some("".into()),
                invalid_fields: vec!["name".into(), "title".into(), "image".into()],
                errors: vec![
                    "Title can't be blank".into(),
                    "Title is too short (minimum is 1 character)".into(),
                    "Name is already taken by a built-in icon".into(),
                    "Image must not contain script elements".into(),
                ],
            }
        } else {
            views::Form::default()
        };
        for block in ["html", "nav"] {
            let actual = crate::controllers::users::people_tests::render_with(
                &app,
                |_| {},
                |ctx| {
                    let page = views::Index {
                        ctx,
                        icons: icons.clone(),
                        icon: form.clone(),
                    };
                    if block == "html" {
                        page.as_content().render().unwrap()
                    } else {
                        page.as_nav().render().unwrap()
                    }
                },
            );
            if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(
                    format!(
                        "{dir}/icons-{}-{block}.actual",
                        case["name"].as_str().unwrap()
                    ),
                    &actual,
                )
                .unwrap();
                std::fs::write(
                    format!(
                        "{dir}/icons-{}-{block}.expected",
                        case["name"].as_str().unwrap()
                    ),
                    case[block].as_str().unwrap(),
                )
                .unwrap();
            }
            assert_eq!(
                actual,
                case[block].as_str().unwrap(),
                "{} {block}: complete Rails bytes",
                case["name"]
            );
        }
    }
}
#[tokio::test]
async fn upload_normalizes_attaches_and_records_one_creation_audit() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    assert_eq!(
        upload(&app, " Acme_Corp ", "Acme Corp", "clean.svg")
            .await
            .location(),
        Some("http://campfire.test/account/icons")
    );
    app.db().read(|c| {
        let icon=WorkspaceIcon::find_by_name(c,"acme_corp")?.unwrap();assert_eq!(icon.creator_id,DAVID);assert_eq!(icon.title,"Acme Corp");
        assert!(attachments::attached_blob(c,"WorkspaceIcon",icon.id,"image")?.is_some());
        let (count,actor,target,label,details):(i64,i64,String,String,String)=c.query_row("SELECT COUNT(*),actor_id,target_type,target_label,details FROM audit_logs WHERE action='workspace_icon.create' AND target_id=?",[icon.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
        assert_eq!((count,actor,target.as_str(),label.as_str()),(1,DAVID,"WorkspaceIcon","acme_corp"));assert_eq!(serde_json::from_str::<Value>(&details).unwrap(),serde_json::json!({"name":"acme_corp","title":"Acme Corp"}));Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn invalid_and_duplicate_uploads_return_inline_errors_without_rows_or_audits() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let bad = upload(&app, "openai", "", "script.svg").await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(bad.text().contains("already taken by a built-in icon"));
    upload(&app, "acme", "Acme Corp", "clean.svg").await;
    let duplicate = upload(&app, "ACME", "Acme Corp", "clean.svg").await;
    assert_eq!(duplicate.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(duplicate.text().contains("has already been taken"));
    app.db()
        .read(|c| {
            assert_eq!(WorkspaceIcon::ordered(c)?.len(), 1);
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action='workspace_icon.create'",
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
async fn members_cannot_list_upload_or_delete_icons() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    upload(&app, "acme", "Acme Corp", "clean.svg").await;
    let id = app
        .db()
        .read(|c| Ok(WorkspaceIcon::ordered(c)?[0].id))
        .await
        .unwrap();
    let mut member = app.sign_in(KEVIN).await;
    assert_eq!(
        member.get("/account/icons").await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        member
            .write(Req::new(Method::POST, "/account/icons").form(&[("workspace_icon[name]", "x")]))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        member
            .write(Req::new(Method::DELETE, &format!("/account/icons/{id}")))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.db()
            .read(|c| Ok(WorkspaceIcon::ordered(c)?.len()))
            .await
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn destroy_removes_attachment_and_blob_and_records_one_snapshot_audit() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    upload(&app, "acme", "Acme Corp", "clean.svg").await;
    let (id, blob) = app
        .db()
        .read(|c| {
            let icon = WorkspaceIcon::ordered(c)?.remove(0);
            let blob = attachments::attached_blob(c, "WorkspaceIcon", icon.id, "image")?.unwrap();
            Ok((icon.id, blob.id))
        })
        .await
        .unwrap();
    assert_eq!(
        app.david()
            .write(Req::new(Method::DELETE, &format!("/account/icons/{id}")))
            .await
            .location(),
        Some("http://campfire.test/account/icons")
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let exists = app
            .db()
            .read(move |c| {
                Ok(c.query_row(
                    "SELECT EXISTS(SELECT 1 FROM active_storage_blobs WHERE id=?)",
                    [blob],
                    |r| r.get::<_, bool>(0),
                )?)
            })
            .await
            .unwrap();
        if !exists {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "PurgeJob did not remove icon blob"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    app.db().read(move |c|{assert!(WorkspaceIcon::ordered(c)?.is_empty());assert_eq!(c.query_row("SELECT COUNT(*) FROM active_storage_blobs WHERE id=?",[blob],|r|r.get::<_,i64>(0))?,0);assert_eq!(c.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='workspace_icon.destroy' AND target_id=? AND actor_id=? AND target_label='acme'",rusqlite::params![id,DAVID],|r|r.get::<_,i64>(0))?,1);Ok(())}).await.unwrap();
}
#[tokio::test]
async fn serving_svg_and_png_uses_private_bytes_and_checksum_conditionals() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    for (name, file, mime) in [
        ("acme", "clean.svg", "image/svg+xml"),
        ("pixel", "square_64.png", "image/png"),
    ] {
        upload(&app, name, "Icon", file).await;
        let path = format!("/icons/{name}");
        let response = app.david().get(&path).await;
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body, bytes(file));
        assert!(response.content_type().unwrap().starts_with(mime));
        assert_eq!(
            response.header("cache-control"),
            Some("max-age=3600, private")
        );
        assert_eq!(response.header("x-content-type-options"), Some("nosniff"));
        if name == "acme" {
            assert_eq!(
                response.header("content-security-policy"),
                Some("default-src 'none'; style-src 'unsafe-inline'")
            );
        }
        let cached = app
            .david()
            .send(
                Req::new(Method::GET, &path)
                    .header("if-none-match", response.header("etag").unwrap()),
            )
            .await;
        assert_eq!(cached.status, StatusCode::NOT_MODIFIED);
        assert!(cached.body.is_empty());
        assert_eq!(
            app.anonymous().get(&path).await.status,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        app.david().get("/icons/nope").await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn icon_serving_enforces_ws9_enrollment_and_stale_session_rules() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    upload(&app, "acme", "Acme Corp", "clean.svg").await;
    let mut member = app.sign_in(KEVIN).await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM two_factor_credentials WHERE user_id=?",
                [KEVIN],
            )?;
            tx.conn().execute(
                "UPDATE sessions SET two_factor_verified_at=NULL WHERE user_id=?",
                [KEVIN],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        member.get("/icons/acme").await.location(),
        Some("http://campfire.test/two_factor_setup")
    );
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE sessions SET two_factor_verified_at=NULL WHERE user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        app.david().get("/icons/acme").await.location(),
        Some("http://campfire.test/session/new")
    );
}

#[tokio::test]
async fn uniqueness_index_races_render_taken_and_roll_back_upload_and_audit() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    app.db().write(|tx|{tx.conn().execute_batch("CREATE TRIGGER race_icon BEFORE INSERT ON workspace_icons WHEN NEW.name='race' BEGIN INSERT INTO workspace_icons(name,title,creator_id,created_at,updated_at) VALUES(NEW.name,NEW.title,NEW.creator_id,NEW.created_at,NEW.updated_at); END")?;Ok(())}).await.unwrap();
    let response = upload(&app, "race", "Race", "clean.svg").await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.text().contains("has already been taken"));
    app.db()
        .read(|c| {
            assert!(WorkspaceIcon::ordered(c)?.is_empty());
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action='workspace_icon.create'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}
