use super::*;

#[tokio::test]
async fn spa_coexistence_password_and_challenge_success_map_returns_after_the_final_factor() {
    use campfire_db::{TwoFactorCredential, models::user::ui_preference};
    use rails_compat::{ar_encryption::ArEncryption, totp};
    for preference in [UiPreference::Classic, UiPreference::Next] {
        for challenge in [false, true] {
            for (saved, next) in [
                ("/", "/app/"),
                ("/users/me/profile", "/app/settings"),
                ("/app/settings/security", "/app/settings/security"),
                (
                    "/rooms/486777696?message_id=217777555",
                    "/app/r/486777696/m/217777555",
                ),
                // `?classic=1` keeps the classic page, as it does on any SPA-routed request.
                ("/rooms/486777696?classic=1", "/rooms/486777696?classic=1"),
                (
                    "/users/me/profile?classic=1",
                    "/users/me/profile?classic=1",
                ),
            ] {
                let a = enabled().await.expect("frozen seed required");
                let enc = ArEncryption::new(&a.booted.app.secrets);
                let secret = a.db().write(move |tx| {
                    ui_preference::store(tx, DAVID, preference)?;
                    if challenge {
                        let credential = TwoFactorCredential::for_user(tx.conn(), DAVID)?.unwrap();
                        tx.conn().execute("UPDATE two_factor_credentials SET last_totp_at=NULL,consecutive_failures=0,locked_until=NULL WHERE id=?", [credential.id])?;
                        Ok(Some(credential.secret(&enc)?))
                    } else {
                        campfire_db::User::find(tx.conn(), DAVID)?.reset_two_factor(tx)?;
                        Ok(None)
                    }
                }).await.unwrap();
                let mut b = a.anonymous();
                b.get(saved).await;
                b.get("/session/new").await;
                let reply = b
                    .write(Req::new(Method::POST, "/session").form(&[
                        ("email_address", "david@37signals.com"),
                        ("password", "secret123456"),
                    ]))
                    .await;
                let reply = if let Some(secret) = secret {
                    assert_eq!(reply.location(), Some(to("/two_factor_challenge").as_str()));
                    b.get("/two_factor_challenge").await;
                    let code = totp::at(&secret, a.booted.app.clock.now().as_second()).unwrap();
                    b.write(
                        Req::new(Method::POST, "/two_factor_challenge").form(&[("code", &code)]),
                    )
                    .await
                } else {
                    reply
                };
                let destination = if preference == UiPreference::Next {
                    next
                } else {
                    saved
                };
                assert_eq!(
                    reply.location(),
                    Some(to(destination).as_str()),
                    "{saved}, challenge={challenge}"
                );
            }
        }
    }
}
