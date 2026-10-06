//! Human PR writes, always using the viewer's own linked account. HTML/format stay outside.
use super::{
    accounts::{Account, Accounts, REJECTED_TOKEN_REASON},
    actions::{INVALID_REVIEWERS, normalize_reviewers},
    client::{ErrorKind, PullRequestKey, ruby_strip},
};
use campfire_db::Database;
use serde_json::Value;
#[derive(Clone, Copy)]
pub enum Action {
    Comment,
    Review,
    ReviewRequest,
}
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    pub accepted: bool,
    pub notice: Option<String>,
    pub alert: Option<String>,
    pub comment_body: Option<String>,
    pub review_body: Option<String>,
    pub reviewers_body: Option<String>,
}
impl Outcome {
    fn invalid(alert: &str) -> Self {
        Self {
            alert: Some(alert.into()),
            ..Default::default()
        }
    }
}
pub struct Input<'a> {
    pub action: Action,
    pub body: &'a str,
    pub event: &'a str,
    pub reviewers: &'a str,
}
pub async fn perform(
    db: &Database,
    accounts: &Accounts,
    user_id: i64,
    key: &PullRequestKey,
    input: Input<'_>,
) -> campfire_db::Result<Outcome> {
    let Input {
        action,
        body,
        event,
        reviewers,
    } = input;
    let body = ruby_strip(body);
    let mut logins = None;
    match action {
        Action::Comment if super::blank(body) => {
            return Ok(Outcome::invalid("Write a comment first."));
        }
        Action::Review if !["APPROVE", "REQUEST_CHANGES"].contains(&event) => {
            return Ok(Outcome::invalid("Choose Approve or Request changes."));
        }
        Action::Review if event == "REQUEST_CHANGES" && super::blank(body) => {
            return Ok(Outcome::invalid(
                "Add a note describing the requested changes.",
            ));
        }
        Action::ReviewRequest => {
            logins = normalize_reviewers(&Value::String(reviewers.into()));
            if logins.as_ref().is_none_or(|v| v.is_empty()) {
                return Ok(Outcome {
                    reviewers_body: Some(reviewers.into()),
                    ..Outcome::invalid(INVALID_REVIEWERS)
                });
            }
        }
        _ => {}
    }
    let Some(account) = db
        .read(move |conn| Account::for_user(conn, user_id))
        .await?
    else {
        return Ok(Outcome::default());
    };
    if !accounts.usable(account.id).await? {
        return Ok(Outcome::default());
    }
    let result = if let Some(token) = accounts.access_token_for_use(account.id).await? {
        let client = accounts.write_client(token);
        match action {
            Action::Comment => client.create_issue_comment(key, body).await,
            Action::Review => {
                client
                    .create_review(key, event, (!super::blank(body)).then_some(body))
                    .await
            }
            Action::ReviewRequest => {
                client
                    .request_reviewers(key, logins.as_ref().unwrap())
                    .await
            }
        }
    } else {
        Err(super::client::Error {
            kind: ErrorKind::Other,
            message: "GitHub account is not usable".into(),
        })
    };
    match result {
        Ok(_) => {
            let notice = match action {
                Action::Comment => {
                    format!("Comment posted on GitHub as @{}.", account.github_login)
                }
                Action::Review if event == "APPROVE" => {
                    format!("Approved on GitHub as @{}.", account.github_login)
                }
                Action::Review => {
                    format!("Requested changes on GitHub as @{}.", account.github_login)
                }
                Action::ReviewRequest => format!(
                    "Requested review from {} on GitHub as @{}.",
                    logins
                        .unwrap()
                        .iter()
                        .map(|s| format!("@{s}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    account.github_login
                ),
            };
            Ok(Outcome {
                accepted: true,
                notice: Some(notice),
                ..Default::default()
            })
        }
        Err(e) if e.kind == ErrorKind::Unauthorized => {
            db.write(move |tx| Account::mark_disconnected(tx, account.id, REJECTED_TOKEN_REASON))
                .await?;
            Ok(Outcome::invalid(
                "GitHub rejected your token. Reconnect to post.",
            ))
        }
        Err(e) if matches!(e.kind, ErrorKind::Refused | ErrorKind::Other) => {
            let mut result = Outcome::invalid(&e.message);
            match action {
                Action::Comment => result.comment_body = Some(body.into()),
                Action::Review => result.review_body = Some(body.into()),
                Action::ReviewRequest => result.reviewers_body = Some(reviewers.into()),
            };
            Ok(result)
        }
        Err(e) => Err(campfire_db::Error::Other(e.to_string())),
    }
}
