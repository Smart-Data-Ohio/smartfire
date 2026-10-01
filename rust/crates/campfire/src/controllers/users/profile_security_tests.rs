//! Translate the seven pinned profile 2FA criteria through WS9's merged implementation.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{TwoFactorCredential, TwoFactorRememberedDevice, User};
use rails_compat::ar_encryption::ArEncryption;

async fn enrolled(app: &TestApp) -> i64 {
    let encryption = ArEncryption::new(&app.booted.app.secrets);
    app.db()
        .write(move |tx| {
            User::find(tx.conn(), DAVID)?.reset_two_factor(tx)?;
            let credential =
                TwoFactorCredential::create(tx, &encryption, DAVID, "JBSWY3DPEHPK3PXP")?;
            tx.conn().execute(
                "UPDATE two_factor_credentials SET confirmed_at=? WHERE id=?",
                rusqlite::params![tx.now(), credential.id],
            )?;
            let (device, _) = TwoFactorRememberedDevice::create_for(
                tx,
                DAVID,
                Some("TestBrowser/1.0"),
                Some("1.2.3.4"),
            )?;
            Ok(device.id)
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn profile_shows_ws9_devices_and_revocation_controls_but_excludes_expired_devices() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let id = enrolled(&app).await;
    let expired = app
        .db()
        .write(|tx| {
            let (device, _) = TwoFactorRememberedDevice::create_for(
                tx,
                DAVID,
                Some("OldBrowser/1.0"),
                Some("5.6.7.8"),
            )?;
            tx.conn().execute(
                "UPDATE two_factor_remembered_devices SET expires_at=? WHERE id=?",
                rusqlite::params![tx.now().ago(jiff::SignedDuration::from_mins(1)), device.id],
            )?;
            Ok(device.id)
        })
        .await
        .unwrap();
    let reply = app.sign_in(DAVID).await.get("/users/me/profile").await;
    assert_eq!(reply.status, StatusCode::OK);
    let body = reply.text();
    assert!(body.contains("Two-step sign-in"));
    assert!(body.contains("TestBrowser/1.0"));
    assert!(body.contains(&format!("id=\"two_factor_remembered_device_{id}\"")));
    assert!(!body.contains(&format!("id=\"two_factor_remembered_device_{expired}\"")));
    assert!(!body.contains("OldBrowser/1.0"));
    for path in [
        "/two_factor_backup_codes".to_owned(),
        "/two_factor_setup".into(),
        format!("/two_factor_remembered_devices/{id}"),
    ] {
        assert!(body.contains(&format!("action=\"{path}\"")));
    }
    assert!(
        app.db()
            .read(|c| User::find(c, DAVID)?.two_factor_enabled(c))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn every_sensitive_profile_2fa_action_contains_ws9_reauthentication_field() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let id = enrolled(&app).await;
    let reply = app.sign_in(DAVID).await.get("/users/me/profile").await;
    assert_eq!(reply.status, StatusCode::OK);
    let body = reply.text();
    for id in [
        "new_backup_codes".to_owned(),
        "disable_two_factor".into(),
        "revoke_all_devices".into(),
        format!("two_factor_remembered_device_{id}"),
    ] {
        let section = body
            .split_once(&format!("id=\"{id}\""))
            .unwrap()
            .1
            .split_once("</form>")
            .unwrap()
            .0;
        assert!(section.contains("type=\"password\""));
        assert!(section.contains("name=\"reauth\""));
    }
}
struct ConfiguredGoogle;
impl crate::concerns::two_factor::GoogleReauthentication for ConfiguredGoogle {
    fn configured(&self) -> bool {
        true
    }
    fn start(
        &self,
        _: &mut campfire_kit::Ctx,
        _: i64,
    ) -> campfire_kit::Result<campfire_kit::Response> {
        panic!("render-only owner seam must never start OAuth")
    }
}
async fn google_confirmation(linked: bool) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    enrolled(&app).await;
    // WS14 owns the adapter; this test supplies its configuration seam, as the Rails helper does.
    app.booted
        .app
        .two_factor
        .install_google(std::sync::Arc::new(ConfiguredGoogle));
    app.db().write(move |tx| {
        tx.conn().execute("DELETE FROM google_identities WHERE user_id=?",[DAVID])?;
        if linked {tx.conn().execute("INSERT INTO google_identities(user_id,subject,email,created_at,updated_at) VALUES (?,'profile-subject','david@example.test',?,?)",rusqlite::params![DAVID,tx.now(),tx.now()])?;}
        Ok(())
    }).await.unwrap();
    let reply = app.sign_in(DAVID).await.get("/users/me/profile").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply
            .text()
            .contains("action=\"/two_factor_reauthentication\""),
        linked
    );
    assert_eq!(reply.text().contains("Confirm with Google"), linked);
}
#[tokio::test]
async fn linked_profile_offers_ws9_google_confirmation() {
    google_confirmation(true).await;
}
#[tokio::test]
async fn unlinked_profile_hides_ws9_google_confirmation() {
    google_confirmation(false).await;
}
#[tokio::test]
async fn unenrolled_profile_links_to_ws9_setup() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    app.db()
        .write(|tx| User::find(tx.conn(), DAVID)?.reset_two_factor(tx))
        .await
        .unwrap();
    let reply = app.sign_in(DAVID).await.get("/users/me/profile").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.text().contains("href=\"/two_factor_setup\""));
    assert!(reply.text().contains("Set up two-step sign-in"));
}
async fn update_devices(password: bool) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    enrolled(&app).await;
    let mut browser = app.sign_in(DAVID).await;
    let form = if password {
        vec![("user[password]", "new-secret-123456")]
    } else {
        vec![("user[name]", "David H")]
    };
    let reply = browser
        .write(Req::new(Method::PATCH, "/users/me/profile").form(&form))
        .await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert!(reply.location().unwrap().ends_with("/users/me/profile"));
    app.db().read(move |c| {
        assert_eq!(TwoFactorRememberedDevice::for_user(c,DAVID)?.len(),usize::from(!password));
        let count:i64=c.query_row("SELECT COUNT(*) FROM audit_logs WHERE action='user.password.change' AND target_id=? AND actor_id=?",rusqlite::params![DAVID,DAVID],|r|r.get(0))?;
        assert_eq!(count,i64::from(password));
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn password_update_uses_ws9_device_revocation_and_audit() {
    update_devices(true).await;
}
#[tokio::test]
async fn name_update_keeps_ws9_remembered_devices() {
    update_devices(false).await;
}
