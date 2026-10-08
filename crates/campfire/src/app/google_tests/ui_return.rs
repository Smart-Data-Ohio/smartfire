use super::*;

#[tokio::test]
async fn google_link_and_reauthentication_return_to_the_selected_ui_with_the_same_flash() {
    use campfire_db::models::user::ui_preference::{self, UiPreference};
    for preference in [UiPreference::Classic, UiPreference::Next] {
        let (a, r) = app_with_env(&[("SPA_ENABLED", "1")]).await;
        a.db()
            .write(move |tx| ui_preference::store(tx, DAVID, preference))
            .await
            .unwrap();
        let mut b = a.sign_in(DAVID).await;
        b.get("/app/").await;
        let integrations = if preference == UiPreference::Next {
            "/app/settings/integrations"
        } else {
            "/users/me/profile"
        };
        let security = if preference == UiPreference::Next {
            "/app/settings/security"
        } else {
            "/users/me/profile"
        };
        let q = start(&mut b, "/user/profile/google_sign_in_link").await;
        let cancelled = b
            .get(&format!(
                "/session/google/callback?state={}&error=denied",
                crate::controllers::presenters::test_support::encode(&q["state"])
            ))
            .await;
        assert_eq!(
            cancelled.location(),
            Some(format!("http://campfire.test{integrations}").as_str())
        );
        assert_flash_once(&mut b, "alert", "Google linking was cancelled.").await;
        let q = start(&mut b, "/user/profile/google_sign_in_link").await;
        answer(&r, claims(&a, &q, "david", "different@smartdata.net"));
        assert_eq!(
            callback(&mut b, &q["state"]).await.location(),
            Some(format!("http://campfire.test{integrations}").as_str())
        );
        assert_flash_once(
            &mut b,
            "notice",
            "Google sign-in linked to different@smartdata.net.",
        )
        .await;
        assert_eq!(
            b.write(Req::new(Method::POST, "/user/profile/google_sign_in_link"))
                .await
                .location(),
            Some(format!("http://campfire.test{integrations}").as_str())
        );
        assert_flash_once(&mut b, "notice", "Google sign-in is already linked.").await;
        for outcome in ["cancelled", "stale", "wrong-subject", "success"] {
            let q = start(&mut b, "/two_factor_reauthentication").await;
            let reply = if outcome == "cancelled" {
                b.get(&format!(
                    "/session/google/callback?state={}&error=denied",
                    crate::controllers::presenters::test_support::encode(&q["state"])
                ))
                .await
            } else {
                let mut v = claims(
                    &a,
                    &q,
                    if outcome == "wrong-subject" {
                        "other"
                    } else {
                        "david"
                    },
                    "different@smartdata.net",
                );
                if outcome == "stale" {
                    v["auth_time"] = json!(a.booted.app.clock.now().as_second() - 3600);
                }
                answer(&r, v);
                callback(&mut b, &q["state"]).await
            };
            assert_eq!(
                reply.location(),
                Some(format!("http://campfire.test{security}").as_str()),
                "{outcome}"
            );
            let (kind, message) = match outcome {
                "success" => (
                    "notice",
                    "Confirmed with Google. Continue with what you were doing.",
                ),
                "cancelled" => ("alert", "Google confirmation was cancelled."),
                "wrong-subject" => (
                    "alert",
                    "That Google account is not linked here. Confirm with the Google account you sign in with.",
                ),
                _ => ("alert", "Google confirmation failed. Try again."),
            };
            assert_flash_once(&mut b, kind, message).await;
        }
        assert!(
            actions(&a)
                .await
                .contains(&"two_factor.reauthenticate".into())
        );
    }
}

async fn assert_flash_once(b: &mut Browser<'_>, kind: &str, message: &str) {
    // A background boot refresh cannot sweep feedback before the shell shows it.
    let boot = b
        .send(Req::new(Method::GET, "/api/v1/boot").header("accept", "application/json"))
        .await;
    assert_eq!(boot.status, StatusCode::OK);
    assert!(boot.json().get("flash").is_none());
    let shell = b.get("/app/settings/integrations").await;
    let html = shell.text();
    let json = html
        .split("id=\"boot\"")
        .nth(1)
        .unwrap()
        .split_once('>')
        .unwrap()
        .1
        .split("</script>")
        .next()
        .unwrap();
    let boot: Value = serde_json::from_str(json).unwrap();
    assert_eq!(boot["flash"], json!({"kind":kind,"message":message}));
    let shell = b.get("/app/settings/integrations").await;
    assert!(!shell.text().contains("\"flash\""));
}

#[tokio::test]
async fn google_sign_in_returns_directly_to_the_spa_after_the_last_factor() {
    use campfire_db::models::user::ui_preference::{self, UiPreference};
    for preference in [UiPreference::Classic, UiPreference::Next] {
        for (saved, next) in [
            ("/", "/app/"),
            ("/users/me/profile", "/app/settings"),
            ("/app/settings/security", "/app/settings/security"),
            (
                "/rooms/486777696?message_id=9",
                "/app/r/486777696/m/9",
            ),
            ("/rooms/486777696/events", "/app/r/486777696/events"),
        ] {
            let (a, r) = app_with_env(&[("SPA_ENABLED", "1")]).await;
            a.db()
                .write(move |tx| {
                    ui_preference::store(tx, DAVID, preference)?;
                    campfire_db::models::google_identity::GoogleIdentity::link_to_user(
                        tx,
                        json!({"sub":"david","email":"david@smartdata.net","hd":"smartdata.net"})
                            .as_object()
                            .unwrap(),
                        DAVID,
                    )?;
                    campfire_db::User::find(tx.conn(), DAVID)?.reset_two_factor(tx)?;
                    Ok(())
                })
                .await
                .unwrap();
            let mut b = a.anonymous();
            // An authenticated page stashes its actual request URI in the cookie session.
            assert_eq!(
                b.get(saved).await.location(),
                Some("http://campfire.test/session/new"),
                "{saved}: stashes a real authenticated page"
            );
            b.get("/session/new").await;
            let q = start(&mut b, "/session/google").await;
            answer(&r, claims(&a, &q, "david", "david@smartdata.net"));
            let reply = callback(&mut b, &q["state"]).await;
            let expected = if preference == UiPreference::Next {
                next
            } else {
                saved
            };
            assert_eq!(
                reply.location(),
                Some(format!("http://campfire.test{expected}").as_str()),
                "{saved}"
            );
        }
    }
}
