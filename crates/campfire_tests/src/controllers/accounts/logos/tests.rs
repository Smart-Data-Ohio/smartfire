use crate::controllers::presenters::{attachments, test_support::*};
use axum::http::{Method, StatusCode};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use campfire_db::Account;
use serde_json::Value;
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../../../vectors/users_logos.json")).unwrap()
}
fn file(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vectors/users_logos")
            .join(name),
    )
    .unwrap()
}
#[tokio::test]
async fn stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut browser = app.david();
    let mut uploaded = None;
    for case in vectors()["cases"].as_array().unwrap() {
        let name = case["file"].as_str();
        if name != uploaded {
            let name = name.unwrap();
            let response = browser
                .write(Req::new(Method::PATCH, "/account").multipart(
                    &[],
                    (
                        "account[logo]",
                        name,
                        if name.ends_with("jpg") {
                            "image/jpeg"
                        } else {
                            "image/bmp"
                        },
                        &file(name),
                    ),
                ))
                .await;
            assert_eq!(response.status, StatusCode::FOUND);
            uploaded = Some(name);
        }
        let path = format!(
            "/account/logo{}",
            case["size"]
                .as_str()
                .map_or(String::new(), |s| format!("?size={s}"))
        );
        let response = browser.get(&path).await;
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16
        );
        assert_eq!(response.content_type(), case["content_type"].as_str());
        assert_eq!(
            response.header("cache-control"),
            case["cache_control"].as_str()
        );
        assert_eq!(
            response.body,
            STANDARD.decode(case["body"].as_str().unwrap()).unwrap(),
            "{name:?} {} complete Rails PNG bytes",
            case["size"]
        );
        let size = if case["size"] == "small" {
            192_u32
        } else {
            512_u32
        };
        assert_eq!(&response.body[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(
            u32::from_be_bytes(response.body[16..20].try_into().unwrap()),
            size,
            "original logo width"
        );
        assert_eq!(
            u32::from_be_bytes(response.body[20..24].try_into().unwrap()),
            size,
            "original logo height"
        );
        assert_eq!(
            response.header("etag"),
            case["etag"].as_str(),
            "{name:?} {}",
            case["size"]
        );
    }
}
#[tokio::test]
async fn logo_upload_replacement_deletion_audits_and_cache_validation_match_rails() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut browser = app.david();
    let stock = browser.get("/account/logo").await;
    assert_eq!(
        app.anonymous()
            .send(
                Req::new(Method::GET, "/account/logo")
                    .header("if-none-match", stock.header("etag").unwrap())
            )
            .await
            .status,
        StatusCode::NOT_MODIFIED
    );
    for name in ["moon.jpg", "pixel.bmp"] {
        assert_eq!(
            browser
                .write(Req::new(Method::PATCH, "/account").multipart(
                    &[],
                    (
                        "account[logo]",
                        name,
                        if name.ends_with("jpg") {
                            "image/jpeg"
                        } else {
                            "image/bmp"
                        },
                        &file(name)
                    )
                ))
                .await
                .status,
            StatusCode::FOUND
        );
    }
    app.db().read(|c|{
        let account=Account::first(c)?.unwrap();let row= c.query_row("SELECT actor_id,target_id,target_type,details FROM audit_logs WHERE action='account.settings.change'",[],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?)))?;
        assert_eq!((row.0,row.1,row.2.as_str()),(DAVID,account.id,"Account"));assert_eq!(serde_json::from_str::<Value>(&row.3).unwrap(),serde_json::json!({"logo":{"before":false,"after":true}}));
        assert_eq!(c.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='account.settings.change'",[],|r|r.get::<_,i64>(0))?,1);assert!(attachments::attached_blob(c,"Account",account.id,"logo")?.is_some());Ok(())
    }).await.unwrap();
    let removed = browser
        .write(Req::new(Method::DELETE, "/account/logo"))
        .await;
    assert_eq!(removed.status, StatusCode::FOUND);
    assert_eq!(
        removed.location(),
        Some("http://campfire.test/account/edit")
    );
    app.db()
        .read(|c| {
            let account = Account::first(c)?.unwrap();
            assert!(attachments::attached_blob(c, "Account", account.id, "logo")?.is_none());
            let details:String=c.query_row("SELECT details FROM audit_logs WHERE action='account.settings.change' ORDER BY id DESC LIMIT 1",[],|r|r.get(0))?;
            assert_eq!(serde_json::from_str::<Value>(&details).unwrap(),serde_json::json!({"logo":{"before":true,"after":false}}),"original logo removal audit pair");
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action='account.settings.change'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                2,
                "original delete adds exactly one logo settings audit after the first upload"
            );
            Ok(())
        })
        .await
        .unwrap();
}
