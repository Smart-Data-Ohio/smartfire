//! Bearer-only GitHub approval transport; policy and inbox writes live in the domain.
use crate::{
    app::AppCtx,
    concerns::{self, AuthenticatedBy, Before, cast_integer},
    controllers::presenters::page::db_error,
    integrations::github::approval_requests::{self, Reply},
};
use campfire_kit::{Ctx, Param, Result, StatusCode, format};
use serde_json::json;
fn render(c: &mut Ctx, reply: Reply) -> campfire_kit::Response {
    let status = StatusCode::from_u16(reply.status).expect("domain status");
    match reply.body {
        Some(body) => c.render(status, &format::JSON, rails_compat::json_encode(&body)),
        None => c.head(status).content_type("text/html"),
    }
}
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_agent_access()).await?;
    if concerns::authenticated_by(c) != AuthenticatedBy::AgentToken {
        return Ok(c.render(
            StatusCode::FORBIDDEN,
            &format::JSON,
            json!({"error":"Forbidden: Bearer agent token required"}).to_string(),
        ));
    }
    let user_id = concerns::require_current_user(c)?.id;
    let room_id = c.param_str("room_id").and_then(cast_integer).unwrap_or(0);
    let pr_id = c
        .params
        .get("pull_request_id")
        .and_then(Param::to_s)
        .as_deref()
        .and_then(cast_integer);
    let scope = c
        .app()
        .db
        .read(move |conn| approval_requests::scope(conn, user_id, room_id, pr_id))
        .await
        .map_err(db_error)?;
    let scope = match scope {
        Ok(scope) => scope,
        Err(reply) => return Ok(render(c, reply)),
    };
    let Some(account) = c
        .app()
        .github_accounts
        .agent_identity(scope.owner_id, scope.user_id)
        .await
        .map_err(db_error)?
    else {
        return Ok(c.render(
            StatusCode::UNPROCESSABLE_ENTITY,
            &format::JSON,
            json!({"error":"Agent has no usable GitHub account"}).to_string(),
        ));
    };
    if !c
        .app()
        .github_accounts
        .usable(account.id)
        .await
        .map_err(db_error)?
    {
        return Ok(c.render(
            StatusCode::UNPROCESSABLE_ENTITY,
            &format::JSON,
            json!({"error":"Agent has no usable GitHub account"}).to_string(),
        ));
    }
    // Rails' minute-bucket key includes controller/action and the calling credential digest.
    let authorization = c
        .request
        .header("authorization")
        .unwrap_or_default()
        .to_owned();
    concerns::agent_api::throttle(c, 60, "agents/github/pull_request_actions", "create")?;
    concerns::agent_api::no_store(c);
    let secret = concerns::agent_bearer_secret(Some(&authorization))
        .unwrap_or_default()
        .to_owned();
    let submitted = serde_json::Value::Object(
        c.params
            .iter()
            .map(|(key, p)| (key.to_owned(), p.to_json()))
            .collect(),
    );
    let reply = c
        .app()
        .db
        .write(move |tx| {
            approval_requests::create(
                tx,
                approval_requests::Request {
                    user_id,
                    room_id,
                    pr_id,
                    secret,
                    account,
                    submitted,
                },
            )
        })
        .await
        .map_err(db_error)?;
    Ok(render(c, reply))
}
