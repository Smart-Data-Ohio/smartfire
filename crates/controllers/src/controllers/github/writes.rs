//! Shared GithubWriteAction scope and legacy HTTP transport over human write policy.
use crate::{
    app::AppCtx,
    concerns::{self, Before, cast_integer},
    controllers::presenters::page::db_error,
    integrations::github::{
        client::PullRequestKey,
        pull_requests::PullRequest,
        threads::PullRequestThread,
        writes::Action,
    },
};
use campfire_db::ChannelThread;
use campfire_kit::{Ctx, Error, Param, Result, StatusCode, format};
async fn before(c: &mut Ctx) -> Result<(PullRequest, ChannelThread, i64)> {
    concerns::before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    let id = c
        .params
        .get("pull_request_id")
        .filter(|p| !p.is_null() && p.to_json() != serde_json::Value::Bool(false))
        .or_else(|| c.params.get("id"))
        .and_then(Param::to_s)
        .as_deref()
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    let user_id = concerns::require_current_user(c)?.id;
    let (pr, thread) = c
        .app()
        .db
        .read(move |conn| {
            let pr = PullRequest::find(conn, id)?;
            let mapping = PullRequestThread::for_room_pr(conn, room.id, id)?.ok_or(
                campfire_db::Error::RecordNotFound("Github::PullRequestThread"),
            )?;
            let thread = ChannelThread::find(conn, mapping.channel_thread_id)?;
            Ok((pr, thread))
        })
        .await
        .map_err(db_error)?;
    Ok((pr, thread, user_id))
}
fn param(c: &Ctx, key: &str) -> String {
    c.params.get(key).and_then(Param::to_s).unwrap_or_default()
}
async fn write(c: &mut Ctx, action: Action) -> Result {
    let (pr, thread, user_id) = before(c).await?;
    let key = PullRequestKey {
        owner: pr.owner.clone(),
        repo: pr.repo.clone(),
        number: pr.number,
    };
    let result = crate::integrations::github::writes::perform(
        &c.app().db,
        &c.app().github_accounts,
        user_id,
        &key,
        crate::integrations::github::writes::Input {
            action,
            body: &param(c, "body"),
            event: &param(c, "event"),
            reviewers: &param(c, "reviewers"),
        },
    )
    .await
    .map_err(db_error)?;
    let _ = thread;
    c.respond_to(&[&format::HTML])?;
    Ok(c.head(if result.accepted { StatusCode::OK } else { StatusCode::UNPROCESSABLE_ENTITY }))
}
pub async fn comment(c: &mut Ctx) -> Result {
    write(c, Action::Comment).await
}
pub async fn review(c: &mut Ctx) -> Result {
    write(c, Action::Review).await
}
pub async fn review_request(c: &mut Ctx) -> Result {
    write(c, Action::ReviewRequest).await
}
