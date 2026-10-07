//! `Google::ConnectionsController`: offline consent is separate from Google sign-in.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    integrations::google::api,
};
use campfire_db::{
    Timestamp,
    models::google_account::ConnectionGrant,
};
use campfire_kit::{Ctx, Error, Redirect, Result, StatusCode};
use rails_compat::ar_encryption::ArEncryption;
use rand::RngCore;
use serde_json::{Value, json};
use subtle::ConstantTimeEq;
const STATE: &str = "google_oauth_state";
pub fn configured(c: &Ctx) -> Result<()> {
    if c.app().google.api().config.configured() {
        Ok(())
    } else {
        Err(Error::Status(StatusCode::NOT_FOUND))
    }
}
fn scalar(c: &Ctx, key: &str) -> String {
    c.params
        .get(key)
        .map(|p| campfire_richtext::ruby::json_value_to_s(&p.to_json()))
        .unwrap_or_default()
}
fn redirect(c: &mut Ctx, message: &str, notice: bool) -> Result {
    c.redirect_to_with(
        &c.url_for("/users/me/profile"),
        Redirect {
            notice: notice.then(|| message.into()),
            alert: (!notice).then(|| message.into()),
            ..Default::default()
        },
    )
}
pub async fn connect(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    configured(c)?;
    concerns::sudo::require_sudo_mode(c)?;
    let mut bytes = [0; 16];
    rand::rng().fill_bytes(&mut bytes);
    let raw = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let signed =
        rails_compat::app_verifier(&c.app().secrets, STATE).generate(&json!(raw), None, None);
    c.session().insert(STATE, raw);
    let drive = c.params.get("features").is_some_and(|p| {
        p.to_json()
            .as_array()
            .is_some_and(|a| a.contains(&json!("drive")))
    });
    c.redirect_to_with(
        &c.app()
            .google
            .api()
            .authorize_url(&c.url_for("/google/callback"), &signed, drive),
        Redirect {
            allow_other_host: true,
            ..Default::default()
        },
    )
}
pub async fn callback(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    configured(c)?;
    let stored = c.session().remove(STATE).unwrap_or(Value::Null);
    let verified = rails_compat::app_verifier(&c.app().secrets, STATE)
        .verify(&scalar(c, "state"), None, c.now())
        .ok();
    let valid = match (verified.as_ref().and_then(Value::as_str), stored.as_str()) {
        (Some(a), Some(b)) => a.as_bytes().ct_eq(b.as_bytes()).into(),
        _ => false,
    };
    if !valid {
        return redirect(c, "Google connection expired. Try again.", false);
    }
    if c.params.get("error").is_some_and(|p| p.is_present()) {
        return redirect(c, "Google Calendar connection was not approved.", false);
    }
    let api = c.app().google.api();
    let result = async {
        let tokens = api
            .exchange_code(&scalar(c, "code"), &c.url_for("/google/callback"))
            .await?;
        let now = Timestamp::from_jiff(c.now());
        let email =
            api.email_from_id_token(tokens["id_token"].as_str().unwrap_or_default(), now)?;
        let user = concerns::require_current_user(c)
            .map_err(|_| api::Error::Rejected("Missing user".into()))?
            .clone();
        let context = super::two_factor::audit_context(c)
            .map_err(|_| api::Error::Rejected("Missing audit actor".into()))?;
        let enc = ArEncryption::new(&c.app().secrets);
        let calendar = c
            .app()
            .db
            .write(move |tx| {
                let grant = ConnectionGrant {
                    user_id: user.id,
                    email,
                    access_token: tokens["access_token"].as_str().map(str::to_owned),
                    refresh_token: tokens["refresh_token"].as_str().map(str::to_owned),
                    scopes: tokens["scope"].as_str().map(str::to_owned),
                    access_token_expires_at: Some(tx.now().since(jiff::SignedDuration::from_secs(
                        api::integer(&tokens["expires_in"]),
                    ))),
                };
                campfire_db::models::google_connection::finish(tx, &enc, grant, &user, &context)
            })
            .await?;
        Ok::<_, api::Error>(calendar)
    }
    .await;
    match result {
        Ok(true) => redirect(c, "Google Calendar connected.", true),
        Ok(false) => redirect(
            c,
            "Calendar permission was not granted. Reconnect to publish events.",
            false,
        ),
        Err(api::Error::Storage(e)) => Err(Error::internal(e)),
        Err(e) => {
            tracing::warn!("Google OAuth callback failed: {}", e.class());
            redirect(c, "Could not connect Google Calendar. Try again.", false)
        }
    }
}
pub async fn destroy(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    configured(c)?;
    concerns::sudo::require_sudo_mode(c)?;
    let user = concerns::require_current_user(c)?.clone();
    let notice = disconnect_user(c, user).await?;
    redirect(c, &notice, true)
}

/// The disconnect action after configuration, authentication and sudo. Local cleanup commits
/// before the best-effort remote channel stop, then the account and audit finish together.
pub async fn disconnect_user(c: &Ctx, user: campfire_db::User) -> Result<String> {
    let id = user.id;
    let secrets = c.app().secrets.clone();
    let for_prepare = user.clone();
    let plan = c.app().db.write(move |tx| {
        campfire_db::models::google_connection::prepare_disconnect(tx, &for_prepare, &secrets)
    }).await.map_err(Error::internal)?;
    if let Some(plan) = plan {
        let channel = crate::integrations::google::calendar::stop_remote(c.app(), id).await.map_err(Error::internal)?;
        let context = super::two_factor::audit_context(c)?;
        c.app().db.write(move |tx| {
            campfire_db::models::google_connection::finish_disconnect(tx, &user, plan, channel, &context)
        }).await.map_err(Error::internal)?;
    }
    Ok("Google Calendar disconnected.".into())
}
