//! `app/controllers/slack/*` and account Slack setup. Domain work lives in integrations::slack.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    integrations::slack::{connections::Service, oauth},
};
use campfire_db::{
    User,
    audit_log::{Actor, Context},
};
use campfire_kit::{Ctx, Error, Param, Redirect, Result};
use serde_json::json;
fn service(c: &Ctx) -> Service<'_> {
    Service {
        db: &c.app().db,
        oauth: oauth::OAuth {
            network: c.app().slack_network.clone(),
        },
        encryption: std::sync::Arc::new(rails_compat::ar_encryption::ArEncryption::new(
            &c.app().secrets,
        )),
    }
}
fn param(c: &Ctx, key: &str) -> String {
    c.params
        .get(key)
        .map(|p| oauth::string(&p.to_json()))
        .unwrap_or_default()
}
fn default_path(user: &User) -> &'static str {
    if user.is_administrator() {
        "/account/slack_import"
    } else {
        "/slack/imports"
    }
}
fn return_path(user: &User, path: &str) -> String {
    if ["/account/slack_import", "/slack/imports"].contains(&path) {
        path.into()
    } else {
        default_path(user).into()
    }
}
fn audit(c: &Ctx, user: &User) -> Result<Context> {
    Ok(Context {
        actor: Some(Actor::from(user)),
        ip_address: Some(c.request.remote_ip()?.into()),
        user_agent: c.request.user_agent().map(str::to_owned),
    })
}
fn redirect(c: &mut Ctx, path: &str, notice: Option<&str>, alert: Option<&str>) -> Result {
    c.redirect_to_with(
        path,
        Redirect {
            notice: notice.map(str::to_owned),
            alert: alert.map(str::to_owned),
            ..Default::default()
        },
    )
}
pub async fn start(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::require_sudo_mode(c)?;
    let user = concerns::require_current_user(c)?.clone();
    let Some(workspace) = service(c)
        .current_workspace()
        .await
        .map_err(Error::internal)?
    else {
        return redirect(
            c,
            default_path(&user),
            None,
            Some("Set up the Slack app credentials first."),
        );
    };
    let raw = rand::random::<[u8; 16]>()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let path = return_path(&user, &param(c, "return_to"));
    c.session()
        .insert("slack_oauth_state", json!({"state":raw,"user_id":user.id}));
    c.session().insert("slack_oauth_return_to", path);
    let location = oauth::authorize_url(
        workspace.client_id.as_deref().unwrap_or(""),
        &c.url_for("/slack/oauth/callback"),
        &oauth::sign_state(&c.app().secrets, &raw),
        workspace.team_id.as_deref(),
    );
    c.redirect_to_with(
        &location,
        Redirect {
            allow_other_host: true,
            ..Default::default()
        },
    )
}
pub async fn callback(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let user = concerns::require_current_user(c)?.clone();
    let stored = c.session().remove("slack_oauth_state");
    let return_to = c.session().remove("slack_oauth_return_to");
    let path = return_path(
        &user,
        &return_to.as_ref().map(oauth::string).unwrap_or_default(),
    );
    if !oauth::valid_state(
        &c.app().secrets,
        &param(c, "state"),
        stored.as_ref(),
        user.id,
        c.now(),
    ) {
        return redirect(c, &path, None, Some("Slack connection expired. Try again."));
    }
    if c.params.get("error").is_some_and(Param::is_present) {
        return redirect(c, &path, None, Some("Slack connection was not approved."));
    }
    let Some(workspace) = service(c)
        .current_workspace()
        .await
        .map_err(Error::internal)?
    else {
        return redirect(
            c,
            &path,
            None,
            Some("The Slack app credentials were removed. Set them up again."),
        );
    };
    let secret = workspace
        .client_secret(&service(c).encryption)
        .map_err(Error::internal)?
        .unwrap_or_default();
    let exchange = service(c)
        .oauth
        .exchange_code(
            workspace.client_id.as_deref().unwrap_or(""),
            &secret,
            &param(c, "code"),
            &c.url_for("/slack/oauth/callback"),
        )
        .await;
    let exchange = match exchange {
        Ok(v) => v,
        Err(_) => return redirect(c, &path, None, Some("Could not connect Slack. Try again.")),
    };
    let context = audit(c, &user)?;
    let alert = service(c)
        .connect(user, workspace, exchange, context)
        .await
        .map_err(Error::internal)?;
    redirect(
        c,
        &path,
        alert.is_none().then_some("Slack connected."),
        alert.as_deref(),
    )
}
pub async fn disconnect(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::require_sudo_mode(c)?;
    let user = concerns::require_current_user(c)?.clone();
    let path = return_path(&user, &param(c, "return_to"));
    let context = audit(c, &user)?;
    if service(c)
        .disconnect(user, context)
        .await
        .map_err(Error::internal)?
    {
        redirect(c, &path, Some("Slack disconnected."), None)
    } else {
        redirect(
            c,
            &path,
            None,
            Some("Finish or cancel your running Slack import first."),
        )
    }
}
pub mod setup;
#[cfg(test)]
mod tests;
