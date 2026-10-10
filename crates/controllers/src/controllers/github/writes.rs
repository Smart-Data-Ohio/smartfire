//! Shared GithubWriteAction scope and thin HTML/Turbo transport over human write policy.
use crate::{
    app::AppCtx,
    concerns::{self, Before, cast_integer},
    controllers::presenters::{page::db_error, view_context::Layout},
    integrations::github::{
        accounts::Account,
        client::PullRequestKey,
        pull_requests::PullRequest,
        threads::PullRequestThread,
        writes::{Action, Outcome},
    },
};
use campfire_db::ChannelThread;
use campfire_kit::{Ctx, Error, Param, Result, StatusCode, format};
use campfire_views::github::write_actions::WriteActions;
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
async fn render(
    c: &mut Ctx,
    pr: PullRequest,
    thread: ChannelThread,
    user_id: i64,
    result: Outcome,
    show: bool,
) -> Result {
    let account = c
        .app()
        .db
        .read(move |conn| Account::for_user(conn, user_id))
        .await
        .map_err(db_error)?;
    let usable = if let Some(a) = &account {
        c.app()
            .github_accounts
            .usable(a.id)
            .await
            .map_err(db_error)?
    } else {
        false
    };
    let data = WriteActions {
        thread_id: thread.id,
        room_id: thread.room_id,
        pull_request_id: pr.id,
        linked: account.is_some(),
        usable,
        login: account.map(|a| a.github_login).unwrap_or_default(),
        notice: result.notice,
        alert: result.alert,
        comment_body: result.comment_body,
        review_body: result.review_body,
        reviewers_body: result.reviewers_body,
    };
    let layout = Layout::load(c).await?;
    let html = layout.render(c, |_| Ok(data.render()))?;
    if show {
        return Ok(c.html(html));
    }
    let status = if result.accepted {
        StatusCode::OK
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    };
    match c.respond_to(&[&format::TURBO_STREAM,&format::HTML])? {
  f if f==&format::TURBO_STREAM=>Ok(c.render(status,&format::TURBO_STREAM,format!("<turbo-stream action=\"replace\" target=\"github_write_actions_channel_thread_{}\"><template>{html}</template></turbo-stream>",thread.id))),
  _=>Ok(c.render(status,&format::HTML,html))
 }
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
    render(c, pr, thread, user_id, result, false).await
}
pub async fn show(c: &mut Ctx) -> Result {
    let (pr, thread, user_id) = before(c).await?;
    render(c, pr, thread, user_id, Outcome::default(), true).await
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

use campfire_views::rendering::*;
use campfire_web::controllers::presenters::view_context::LayoutRendering;
