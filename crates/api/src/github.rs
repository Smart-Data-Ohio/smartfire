//! Pull-request writes on `/api/v1`: the actions the viewer can take, and commenting, reviewing
//! and requesting reviewers. The GitHub call, its errors and the "no local message" record are
//! [`campfire_app::integrations::github::writes::perform`], the same service the classic
//! controllers use. A success does not refresh the card; GitHub's webhook does, through the
//! `github_cards` channel and its `message.cards` twin.

use std::collections::BTreeMap;

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_app::integrations::github::{
    accounts::Account,
    client::PullRequestKey,
    pull_requests::PullRequest,
    threads::{PullRequestThread, discuss},
    writes::{self, Action},
};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_runtime::concerns::{self, cast_integer};
use campfire_runtime::context::db_error;

use crate::endpoints::{before_actions, body, set_room};
use crate::error::fail;

endpoint!(
    /// `GET /api/v1/rooms/:room_id/github/pull_requests/:id/actions`
    github_actions => show_actions
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/github/pull_requests/:id/comments`
    github_comment => post_comment
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/github/pull_requests/:id/reviews`
    github_review => post_review
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/github/pull_requests/:id/review_requests`
    github_review_request => post_review_request
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/github/pull_requests/:id/discussion`
    github_discussion => post_discussion
);

struct Scope {
    key: PullRequestKey,
    user_id: i64,
    status: api::GithubPullRequestStatus,
}

async fn scope(c: &mut Ctx) -> Result<Scope> {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let id = path_id(c, "id")?;
    let room_id = room.id;
    let pull_request = c
        .app()
        .db
        .read(move |conn| {
            let pull_request = PullRequest::find(conn, id)?;
            if PullRequestThread::for_room_pr(conn, room_id, id)?.is_none() {
                return Err(campfire_db::Error::RecordNotFound(
                    "Github::PullRequestThread",
                ));
            }
            Ok(pull_request)
        })
        .await
        .map_err(db_error)?;
    let user_id = concerns::require_current_user(c)?.id;
    Ok(Scope {
        key: PullRequestKey {
            owner: pull_request.owner,
            repo: pull_request.repo,
            number: pull_request.number,
        },
        user_id,
        status: status_of(pull_request.state.as_deref()),
    })
}

fn path_id(c: &Ctx, name: &str) -> Result<i64> {
    c.param_str(name)
        .and_then(cast_integer)
        .ok_or(Error::NotFound)
}

fn status_of(state: Option<&str>) -> api::GithubPullRequestStatus {
    match state {
        Some("merged") => api::GithubPullRequestStatus::Merged,
        Some("closed") => api::GithubPullRequestStatus::Closed,
        Some("draft") => api::GithubPullRequestStatus::Draft,
        _ => api::GithubPullRequestStatus::Open,
    }
}

struct ViewerGithub {
    login: Option<String>,
    link: api::GithubAccountLink,
    usable: bool,
}

async fn viewer_github(c: &Ctx, user_id: i64) -> Result<ViewerGithub> {
    let account = c
        .app()
        .db
        .read(move |conn| Account::for_user(conn, user_id))
        .await
        .map_err(db_error)?;
    let Some(account) = account else {
        return Ok(ViewerGithub {
            login: None,
            link: api::GithubAccountLink::None,
            usable: false,
        });
    };
    let usable = c
        .app()
        .github_accounts
        .usable(account.id)
        .await
        .map_err(db_error)?;
    Ok(ViewerGithub {
        login: Some(account.github_login),
        link: if usable {
            api::GithubAccountLink::Connected
        } else {
            api::GithubAccountLink::Rejected
        },
        usable,
    })
}

async fn show_actions(c: &mut Ctx) -> Result {
    let scope = scope(c).await?;
    let viewer = viewer_github(c, scope.user_id).await?;
    c.json(
        StatusCode::OK,
        &api::GithubPullRequestActions {
            login: viewer.login,
            account: viewer.link,
            can_comment: viewer.usable,
            can_review: viewer.usable,
            can_request_reviewers: viewer.usable,
            status: scope.status,
        },
    )
}

async fn post_comment(c: &mut Ctx) -> Result {
    let scoped = scope(c).await?;
    let input: api::CreateGithubComment = body(c).await?;
    write(c, scoped, Action::Comment, &input.body, "", "", "body").await
}

async fn post_review(c: &mut Ctx) -> Result {
    let scoped = scope(c).await?;
    let input: api::CreateGithubReview = body(c).await?;
    let (action, event) = match input.event {
        api::GithubReviewKind::Approve => (Action::Review, "APPROVE"),
        api::GithubReviewKind::RequestChanges => (Action::Review, "REQUEST_CHANGES"),
        api::GithubReviewKind::Comment => (Action::ReviewComment, ""),
    };
    write(c, scoped, action, &input.body, event, "", "body").await
}

async fn post_review_request(c: &mut Ctx) -> Result {
    let scoped = scope(c).await?;
    let input: api::CreateGithubReviewRequest = body(c).await?;
    write(
        c,
        scoped,
        Action::ReviewRequest,
        "",
        "",
        &input.reviewers,
        "reviewers",
    )
    .await
}

async fn write(
    c: &mut Ctx,
    scoped: Scope,
    action: Action,
    comment: &str,
    event: &str,
    reviewers: &str,
    field: &str,
) -> Result {
    let result = writes::perform(
        &c.app().db,
        &c.app().github_accounts,
        scoped.user_id,
        &scoped.key,
        writes::Input {
            action,
            body: comment,
            event,
            reviewers,
        },
    )
    .await
    .map_err(db_error)?;
    if result.accepted {
        return c.json(
            StatusCode::OK,
            &api::GithubWriteResult {
                notice: result.notice.unwrap_or_default(),
            },
        );
    }
    if let Some(alert) = result.alert {
        return Err(fail(c, refusal(field, &alert)));
    }
    // No account, or a linked one that isn't usable: `perform` makes no GitHub call and leaves
    // the alert empty. The classic page says which, from the account row.
    let viewer = viewer_github(c, scoped.user_id).await?;
    let message = if viewer.link == api::GithubAccountLink::Rejected {
        "GitHub rejected your token. Reconnect GitHub to comment and review from here."
    } else {
        "Connect GitHub to comment and review from here as yourself."
    };
    Err(fail(c, refusal("account", message)))
}

/// `github/pull_request_threads#create`: [`discuss`] creates the thread, the membership and the
/// `PullRequestThread` row. A thread that already maps this pull request is returned as it is.
async fn post_discussion(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let user_id = concerns::require_current_user(c)?.id;
    let id = path_id(c, "id")?;
    let input: api::CreateGithubDiscussion = body(c).await?;
    let room_id = room.id;
    let parent = input.message_id;
    let (mapping, created) = c
        .app()
        .db
        .write(move |tx| {
            let existed = PullRequestThread::for_room_pr(tx.conn(), room_id, id)?.is_some();
            discuss(tx, room_id, user_id, id, parent).map(|mapping| (mapping, !existed))
        })
        .await
        .map_err(db_error)?;
    if created {
        c.app().broadcasts.thread_created(mapping.channel_thread_id);
    }
    c.json(
        if created {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        &api::GithubDiscussion {
            thread_id: mapping.channel_thread_id,
        },
    )
}

fn refusal(field: &str, message: &str) -> api::ApiError {
    api::ApiError::Validation {
        message: message.into(),
        fields: BTreeMap::from([(field.to_string(), vec![message.to_string()])]),
    }
}
