//! The original named icon assertions, through the actual upload/list/delete endpoints.
use super::*;
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_richtext::dom::Dom;
const JZ: i64 = 773523953;
fn file(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vectors/workspace_icons")
            .join(name),
    )
    .unwrap()
}
async fn upload(app: &TestApp, name: &str, title: &str, image: &str) -> Reply {
    app.david()
        .write(Req::new(Method::POST, "/account/icons").multipart(
            &[
                ("workspace_icon[name]", name),
                ("workspace_icon[title]", title),
            ],
            (
                "workspace_icon[image]",
                image,
                if image.ends_with("svg") {
                    "image/svg+xml"
                } else {
                    "image/png"
                },
                &file(image),
            ),
        ))
        .await
}
#[tokio::test]
async fn original_acme_upload_list_and_audit_shortcodes() {
    let app = TestApp::boot_frozen().await.expect("CI seed required");
    let before = app
        .db()
        .read(|c| Ok(WorkspaceIcon::ordered(c)?.len()))
        .await
        .unwrap();
    let created = upload(&app, "acme", "Acme Corp", "clean.svg").await;
    assert_eq!(created.status, StatusCode::FOUND);
    assert_eq!(
        created.location(),
        Some("http://campfire.test/account/icons")
    );
    let id=app.db().read(move |c| {
        assert_eq!(WorkspaceIcon::ordered(c)?.len(),before+1);
        let icon=WorkspaceIcon::find_by_name(c,"acme")?.unwrap();
        assert_eq!(icon.title,"Acme Corp");assert_eq!(icon.creator_id,DAVID);
        assert!(attachments::attached_blob(c,"WorkspaceIcon",icon.id,"image")?.is_some());
        let (count,target_type,target_id,label):(i64,String,i64,String)=c.query_row("SELECT COUNT(*),target_type,target_id,target_label FROM audit_logs WHERE action='workspace_icon.create'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
        assert_eq!((count,target_type.as_str(),target_id,label.as_str()),(1,"WorkspaceIcon",icon.id,":acme:"));
        Ok(icon.id)
    }).await.unwrap();
    let list = app.david().get("/account/icons").await;
    assert_eq!(list.status, StatusCode::OK);
    let body = list.text();
    let mut dom = Dom::new();
    let root = dom.parse_fragment(&body).unwrap();
    let nodes = dom.descendants(root);
    assert_eq!(
        nodes
            .iter()
            .filter(|n| dom.name(**n) == "img" && dom.attr(**n, "src") == Some("/icons/acme"))
            .count(),
        1
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|n| dom.name(**n) == "code" && dom.text_content(**n) == ":acme:")
            .count(),
        1
    );
    assert!(body.contains("Acme Corp"));
    assert!(body.contains("Uploaded by David"));
    let removed = app
        .david()
        .write(Req::new(Method::DELETE, &format!("/account/icons/{id}")))
        .await;
    assert_eq!(removed.status, StatusCode::FOUND);
    assert_eq!(
        removed.location(),
        Some("http://campfire.test/account/icons")
    );
    app.db().read(move |c| {
        assert_eq!(WorkspaceIcon::ordered(c)?.len(),before);
        let (count,target_id,label):(i64,i64,String)=c.query_row("SELECT COUNT(*),target_id,target_label FROM audit_logs WHERE action='workspace_icon.destroy'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        assert_eq!((count,target_id,label.as_str()),(1,id,":acme:"));
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn original_jz_cannot_list_create_or_remove_the_existing_icon() {
    let app = TestApp::boot_frozen().await.expect("CI seed required");
    upload(&app, "acme", "Acme Corp", "clean.svg").await;
    let id = app
        .db()
        .read(|c| Ok(WorkspaceIcon::find_by_name(c, "acme")?.unwrap().id))
        .await
        .unwrap();
    let mut member = app.sign_in(JZ).await;
    assert_eq!(
        member.get("/account/icons").await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        member
            .write(Req::new(Method::POST, "/account/icons").multipart(
                &[
                    ("workspace_icon[name]", "nope"),
                    ("workspace_icon[title]", "Nope")
                ],
                (
                    "workspace_icon[image]",
                    "clean.svg",
                    "image/svg+xml",
                    &file("clean.svg")
                )
            ))
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
    assert!(
        app.db()
            .read(move |c| Ok(WorkspaceIcon::ordered(c)?
                .into_iter()
                .find(|icon| icon.id == id)))
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn original_password_session_enrollment_and_stale_icon_access() {
    use campfire_db::{Session, TwoFactorCredential, User};
    let app = TestApp::boot_frozen()
        .await
        .expect("CI seed required")
        .without_job_runner()
        .await;
    upload(&app, "acme", "Acme Corp", "clean.svg").await;
    let email = app
        .db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM sessions WHERE user_id=?", [JZ])?;
            tx.conn()
                .execute("DELETE FROM two_factor_credentials WHERE user_id=?", [JZ])?;
            Ok(User::find(tx.conn(), JZ)?.email_address.unwrap())
        })
        .await
        .unwrap();
    let mut browser = app.anonymous();
    let signed = browser
        .write(
            Req::new(Method::POST, "/session")
                .form(&[("email_address", &email), ("password", "secret123456")]),
        )
        .await;
    assert_eq!(signed.status, StatusCode::FOUND);
    let token = app
        .db()
        .read(|c| {
            let sessions = Session::for_user(c, JZ)?;
            assert_eq!(sessions.len(), 1);
            Ok(sessions[0].token.clone())
        })
        .await
        .unwrap();
    let first = browser.get("/icons/acme").await;
    assert_eq!(first.status, StatusCode::FOUND);
    assert_eq!(
        first.location(),
        Some("http://campfire.test/two_factor_setup")
    );
    let encryption = rails_compat::ar_encryption::ArEncryption::new(&app.booted.app.secrets);
    app.db()
        .write(move |tx| {
            let credential = TwoFactorCredential::create(tx, &encryption, JZ, "JBSWY3DPEHPK3PXP")?;
            tx.conn().execute(
                "UPDATE two_factor_credentials SET confirmed_at=? WHERE id=?",
                rusqlite::params![tx.now(), credential.id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let stale = browser.get("/icons/acme").await;
    assert_eq!(stale.status, StatusCode::FOUND);
    assert_eq!(stale.location(), Some("http://campfire.test/session/new"));
    assert!(
        app.db()
            .read(move |c| Session::find_by_token(c, &token))
            .await
            .unwrap()
            .is_none()
    );
}
