//! Human PAT, GitHub App OAuth, and administrator-only bot connection transport.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    integrations::github::{
        client::{Error as GithubError, ErrorKind, ruby_strip},
        connections::ConnectionService,
        oauth,
    },
};
use campfire_db::{
    User,
    audit_log::{Actor, Context},
};
use campfire_kit::{Ctx, Error, Param, Redirect, Result, StatusCode};
fn service(c: &Ctx) -> ConnectionService<'_> {
    ConnectionService {
        db: &c.app().db,
        accounts: &c.app().github_accounts,
        app: &c.app().github_app,
        crypto: std::sync::Arc::new(rails_compat::ar_encryption::ArEncryption::new(
            &c.app().secrets,
        )),
    }
}
fn audit(c: &Ctx) -> Result<Context> {
    Ok(Context {
        actor: Some(Actor::from(concerns::require_current_user(c)?)),
        ip_address: Some(c.request.remote_ip()?.into()),
        user_agent: c.request.user_agent().map(str::to_owned),
    })
}
fn param(c: &Ctx, key: &str) -> String {
    c.params.get(key).and_then(Param::to_s).unwrap_or_default()
}
fn redirect(c: &mut Ctx, path: &str, notice: Option<String>, alert: Option<String>) -> Result {
    c.redirect_to_with(
        path,
        Redirect {
            notice,
            alert,
            ..Default::default()
        },
    )
}
fn failure(c: &mut Ctx, path: &str, e: GithubError, app: bool) -> Result {
    let text = connection_failure(e, app)?;
    redirect(c, path, None, Some(text))
}
fn connection_failure(e: GithubError, app: bool) -> Result<String> {
    let text = match e.kind {
        ErrorKind::Unauthorized => {
            if app {
                "GitHub rejected the connection. Try again."
            } else {
                "GitHub rejected that token. Check it and try again."
            }
        }
        ErrorKind::Refused | ErrorKind::Other => "Could not reach GitHub. Try again.",
        _ => return Err(Error::internal(e)),
    };
    Ok(text.into())
}
async fn before(c: &mut Ctx, bot: bool) -> Result<(User, String)> {
    concerns::before_actions(c, Before::default()).await?;
    let user = if bot {
        concerns::ensure_can_administer(c)?;
        crate::controllers::accounts::bots::find_active_bot(c, "bot_id").await?
    } else {
        concerns::require_current_user(c)?.clone()
    };
    concerns::sudo::require_sudo_mode(c)?;
    let path = if bot {
        campfire_routes::edit_account_bot(user.id)
    } else {
        campfire_routes::user_profile()
    };
    Ok((user, path))
}
async fn link_pat(c: &mut Ctx, bot: bool) -> Result {
    let (user, path) = before(c, bot).await?;
    let token = param(c, "access_token");
    match connect_token(c, user, &token, bot).await? {
        Ok(notice) => redirect(c, &path, Some(notice), None),
        Err(alert) => redirect(c, &path, None, Some(alert)),
    }
}

/// The PAT action after authentication and sudo: the classic notice or alert.
pub async fn connect_token(
    c: &Ctx,
    user: User,
    token: &str,
    bot: bool,
) -> Result<std::result::Result<String, String>> {
    let token = ruby_strip(token).to_owned();
    if crate::integrations::github::blank(&token) {
        return Ok(Err("Paste a token to connect GitHub.".into()));
    }
    let credentials = match service(c).pat(&token).await {
        Ok(v) => v,
        Err(e) => return Ok(Err(connection_failure(e, false)?)),
    };
    let (account, matched) = service(c)
        .link(user, credentials, bot, audit(c)?)
        .await
        .map_err(Error::internal)?;
    let notice = if bot || matched {
        format!("GitHub connected as {}.", account.github_login)
    } else {
        format!(
            "GitHub connected as {}. Another member's linked GitHub account already uses that username, so your profile username was left unchanged.",
            account.github_login
        )
    };
    Ok(Ok(notice))
}
async fn unlink(c: &mut Ctx, bot: bool) -> Result {
    let (user, path) = before(c, bot).await?;
    let notice = disconnect_user(c, user, bot).await?;
    redirect(c, &path, Some(notice), None)
}

/// The disconnect action after authentication and sudo, including revocation and its audit.
pub async fn disconnect_user(c: &Ctx, user: User, bot: bool) -> Result<String> {
    service(c)
        .disconnect(user, bot, audit(c)?)
        .await
        .map_err(Error::internal)?;
    Ok("GitHub disconnected.".into())
}
pub async fn create(c: &mut Ctx) -> Result {
    link_pat(c, false).await
}
pub async fn destroy(c: &mut Ctx) -> Result {
    unlink(c, false).await
}
// WS11-UI owns bot writes, including the pinned after-commit audit boundary.
// Those routes call accounts::bots::github_connections; human routes use this service.
async fn app_before(c: &mut Ctx) -> Result<()> {
    concerns::before_actions(c, Before::default()).await?;
    if !c.app().github_app.configured() {
        return Err(Error::Halt(Box::new(concerns::head(StatusCode::NOT_FOUND))));
    }
    Ok(())
}
pub async fn connect(c: &mut Ctx) -> Result {
    app_before(c).await?;
    concerns::sudo::require_sudo_mode(c)?;
    let raw = rand::random::<[u8; 16]>()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    c.session().insert("github_app_oauth_state", raw.clone());
    let location = c.app().github_app.authorize_url(
        &c.url_for(&campfire_routes::github_app_callback()),
        &oauth::sign_state(&c.app().secrets, &raw),
    );
    c.redirect_to_with(&location, Redirect { allow_other_host: true, ..Default::default() })
}
pub async fn callback(c: &mut Ctx) -> Result {
    app_before(c).await?;
    let stored = c.session().remove("github_app_oauth_state");
    let path = campfire_routes::user_profile();
    if !oauth::valid_state(
        &c.app().secrets,
        &param(c, "state"),
        stored.as_ref(),
        c.now(),
    ) {
        return redirect(
            c,
            &path,
            None,
            Some("GitHub connection expired. Try again.".into()),
        );
    }
    if c.params.get("error").is_some_and(Param::is_present) {
        return redirect(
            c,
            &path,
            None,
            Some("GitHub connection was not approved.".into()),
        );
    }
    let credentials = match service(c)
        .exchange(
            &param(c, "code"),
            &c.url_for(&campfire_routes::github_app_callback()),
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return failure(c, &path, e, true),
    };
    let user = concerns::require_current_user(c)?.clone();
    let (account, _) = service(c)
        .link(user, credentials, false, audit(c)?)
        .await
        .map_err(Error::internal)?;
    redirect(
        c,
        &path,
        Some(format!("GitHub connected as {}.", account.github_login)),
        None,
    )
}
