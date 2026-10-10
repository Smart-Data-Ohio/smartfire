use super::*;

/// A saved classic return path maps to its SPA URL after the final factor, for everyone: no choice,
/// a stored choice of the classic UI, and with `?classic=1` (which no longer escapes the mapping).
#[tokio::test]
async fn spa_only_password_and_challenge_success_map_returns_to_the_spa_for_every_ui() {
    use campfire_db::{TwoFactorCredential, models::user::ui_preference};
    use rails_compat::{ar_encryption::ArEncryption, totp};
    for preference in [None, Some(UiPreference::Classic)] {
        for challenge in [false, true] {
            for (saved, next) in [
                ("/", "/app/"),
                ("/users/me/profile", "/app/settings"),
                ("/app/settings/security", "/app/settings/security"),
                (
                    "/rooms/486777696?message_id=217777555",
                    "/app/r/486777696/m/217777555",
                ),
                // `classic` is dropped, not obeyed.
                ("/rooms/486777696?classic=1", "/app/r/486777696"),
                ("/users/me/profile?classic=1", "/app/settings"),
                (
                    "/rooms/486777696?message_id=217777555&classic=1",
                    "/app/r/486777696/m/217777555",
                ),
            ] {
                let a = enabled().await.expect("frozen seed required");
                let enc = ArEncryption::new(&a.booted.app.secrets);
                let secret = a.db().write(move |tx| {
                    if let Some(preference) = preference {
                        ui_preference::store(tx, DAVID, preference)?;
                    }
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
                assert_eq!(
                    reply.location(),
                    Some(to(next).as_str()),
                    "{saved}, {preference:?}, challenge={challenge}"
                );
            }
        }
    }
}
