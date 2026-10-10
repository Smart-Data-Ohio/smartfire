//! Work links use the shared PR identity/fetch APIs. Google owns credentialed Drive reads.
use super::*;
use crate::integrations::github::{
    jobs::FetchPullRequestJob, pull_requests::PullRequest, references,
};
use askama::Template;
use campfire_db::models::google_account::GoogleAccount;
use campfire_db::{Event, NewWorkThreadLink, WorkThreadLink};
use rails_compat::ar_encryption::ArEncryption;

/// Best-effort title lookup through the viewer's usable, Drive-enabled Google account.
pub async fn resolve_drive_title(c: &Ctx, file_id: &str) -> Option<String> {
    let api = c.app().google.api();
    if !api.config.configured() {
        return None;
    }
    let user_id = require_current_user(c).ok()?.id;
    let enc = ArEncryption::new(&c.app().secrets);
    let account = c
        .app()
        .db
        .write(move |tx| {
            let Some(mut account) = GoogleAccount::for_user(tx.conn(), user_id)? else {
                return Ok(None);
            };
            Ok((account.usable(tx, &enc)? && account.drive()).then_some(account))
        })
        .await
        .ok()
        .flatten()?;
    let file = api
        .drive_file(
            &c.app().db,
            &c.app().secrets,
            account.user_id,
            file_id,
            campfire_db::Timestamp::from_jiff(c.now()),
        )
        .await
        .ok()?;
    let value = file.get("name")?;
    let title = match value {
        Value::Null | Value::Bool(false) => return None,
        Value::Bool(true) => "t".into(),
        Value::Array(items) if items.is_empty() => return None,
        Value::Object(items) if items.is_empty() => return None,
        value => campfire_richtext::ruby::json_value_to_s(value),
    };
    (!campfire_richtext::ruby::is_blank(&title)).then_some(title)
}
pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, thread) = scope(c, false).await?;
    c.no_store();
    let id = thread.id;
    let links = messages::present(c, move |p| board_posts::links(p, &thread)).await?;
    page::content(c, StatusCode::OK, |ctx| {
        views::LinksIndex {
            ctx,
            thread_id: id,
            links: &links,
        }
        .render()
    })
    .await
}
enum Build {
    Invalid(String),
    Missing,
    Link(NewWorkThreadLink),
}
async fn build(c: &mut Ctx, thread: &ChannelThread) -> Result<Build> {
    let user_id = require_current_user(c)?.id;
    let mut attributes = NewWorkThreadLink {
        channel_thread_id: thread.id,
        created_by_id: user_id,
        kind: Some(scalar(c, "kind")),
        ..Default::default()
    };
    match attributes.kind.as_deref() {
        Some("pull_request") => {
            let Some((owner, repo, number)) = references::extract(&scalar(c, "pull_request_url"))
                .into_iter()
                .next()
            else {
                return Ok(Build::Invalid(
                    "Enter a GitHub pull request URL, like https://github.com/owner/repo/pull/123."
                        .into(),
                ));
            };
            let number = number
                .parse::<i64>()
                .map_err(campfire_kit::Error::internal)?;
            let pr = c
                .app()
                .db
                .write(move |tx| PullRequest::for_reference(tx, &owner, &repo, number))
                .await
                .map_err(db_error)?;
            attributes.github_pull_request_id = Some(pr.id);
        }
        Some("event") => {
            let value = c.params.get("event_id");
            if value.is_none_or(Param::is_blank) {
                return Ok(Build::Invalid("Choose an event to link.".into()));
            }
            let ids = finder_ids(value);
            let room = thread.room_id;
            let found = c
                .app()
                .db
                .read(move |conn| {
                    Ok(conn.query_row(
                        "SELECT id FROM events WHERE id IN (SELECT value FROM json_each(?)) AND room_id=? ORDER BY id LIMIT 1",
                        (json!(ids).to_string(), room), |row| row.get::<_, i64>(0)).optional()?)
                })
                .await
                .map_err(db_error)?;
            if found.is_none() {
                return Ok(Build::Missing);
            }
            attributes.event_id = found;
        }
        Some("drive_file") => {
            let raw = scalar(c, "drive_url");
            let url = campfire_richtext::ruby::strip(&raw);
            let Some(file_id) = campfire_db::models::google_drive_link::file_id(&json!(url)) else {
                return Ok(Build::Invalid(
                    "Enter a Google Drive, Docs, Sheets, Slides, or Forms link.".into(),
                ));
            };
            attributes.title = resolve_drive_title(c, &file_id).await;
            attributes.url = Some(url.to_string());
        }
        _ => {
            return Ok(Build::Invalid(
                "Choose a pull request, event, or Drive file to link.".into(),
            ));
        }
    }
    Ok(Build::Link(attributes))
}
pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, thread) = scope(c, false).await?;
    let attributes = match build(c, &thread).await? {
        Build::Invalid(error) => return invalid(c, &thread, error).await,
        Build::Missing => return Ok(c.head(StatusCode::NOT_FOUND)),
        Build::Link(attributes) => attributes,
    };
    let saved = c
        .app()
        .db
        .write(move |tx| {
            let link = WorkThreadLink::create(tx, attributes)?;
            // The durable fetch and its claim are atomic with the triggering link save.
            if let Some(id) = link.github_pull_request_id {
                let mut pr = PullRequest::find(tx.conn(), id)?;
                if pr.claim_fetch_request(tx)? {
                    tx.emit_after_commit(Event::job(&FetchPullRequestJob {
                        pull_request_id: id,
                    }));
                }
            }
            Ok(())
        })
        .await;
    match saved {
        Ok(()) => (),
        Err(error) if error.is_record_not_unique() => {
            return invalid(
                c,
                &thread,
                "That is already linked to this work thread.".into(),
            )
            .await;
        }
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            let message = if errors
                .0
                .iter()
                .any(|(_, message)| message == "has already been taken")
            {
                "That is already linked to this work thread.".into()
            } else {
                campfire_views::helpers::to_sentence(&errors.full_messages(), " and ")
            };
            return invalid(c, &thread, message).await;
        }
        Err(error) => return Err(db_error(error)),
    }
    success(c, &thread, "Link added.", false).await
}
pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, thread) = scope(c, false).await?;
    let ids = finder_ids(c.params.get("id"));
    let thread_id = thread.id;
    let deleted = c
        .app()
        .db
        .write(move |tx| {
            let id = tx.conn().query_row(
                "SELECT id FROM work_thread_links WHERE channel_thread_id=? AND id IN (SELECT value FROM json_each(?)) ORDER BY id LIMIT 1",
                (thread_id, json!(ids).to_string()), |row| row.get::<_, i64>(0)).optional()?;
            let link = id
                .map(|id| WorkThreadLink::find(tx.conn(), id))
                .transpose()?
                .flatten()
                .filter(|link| link.channel_thread_id == thread_id);
            if let Some(link) = link {
                link.destroy(tx)?;
                Ok(true)
            } else {
                Ok(false)
            }
        })
        .await
        .map_err(db_error)?;
    if !deleted {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    success(c, &thread, "Link removed.", true).await
}
async fn success(c: &mut Ctx, thread: &ChannelThread, notice: &str, back: bool) -> Result {
    c.flash().set_notice(notice);
    let path = format!("/rooms/{}/threads/{}", thread.room_id, thread.id);
    if back { c.redirect_back_or_to(&path) } else { c.redirect_to(&path) }
}
async fn invalid(c: &mut Ctx, thread: &ChannelThread, error: String) -> Result {
    c.flash().set_alert(error);
    c.redirect_to(&format!("/rooms/{}/threads/{}", thread.room_id, thread.id))
}
