//! The classic work link builders and viewing authority, exposed as JSON.

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_app::integrations::github::{
    jobs::FetchPullRequestJob, pull_requests::PullRequest, references,
};
use campfire_controllers::controllers::work_threads::links::resolve_drive_title;
use campfire_db::{CalendarEvent, ChannelThread, Event, NewWorkThreadLink, User, WorkThreadLink};
use campfire_kit::{Ctx, Result, StatusCode};
use campfire_web::concerns;
use campfire_web::controllers::presenters::page::db_error;

use crate::agents::human;
use crate::endpoints::{before_actions, body, now};
use crate::error::{fail, not_found};

endpoint!(
    /// `GET /api/v1/threads/:thread_id/work/links/new` (200); see [`api::WorkLinkForm`].
    new => link_form
);
endpoint!(
    /// `POST /api/v1/threads/:thread_id/work/links` (201); see [`api::CreateWorkLink`].
    create => create_link
);
endpoint!(
    /// `DELETE /api/v1/threads/:thread_id/work/links/:id` returns [`api::ThreadDetail`] (200).
    /// Viewing/tracking errors and 404 link scoping are documented on [`api::CreateWorkLink`].
    destroy => destroy_link
);

fn validation(field: &str, message: &str) -> api::ApiError {
    api::ApiError::Validation {
        message: message.into(),
        fields: [(field.into(), vec![message.into()])].into(),
    }
}

/// `work_threads#scope(false)`: visibility precedes tracking, with no manager check.
async fn scope(c: &mut Ctx) -> Result<(ChannelThread, User)> {
    before_actions(c).await?;
    let Some(viewer) = human(c)? else {
        return Err(fail(c, not_found()));
    };
    let id = c.param_str("thread_id").and_then(concerns::cast_integer);
    let actor = viewer.clone();
    let thread = c
        .app()
        .db
        .read(move |conn| {
            let thread = id
                .map(|id| ChannelThread::find_by_id(conn, id))
                .transpose()?
                .flatten();
            match thread {
                Some(thread) if thread.work_viewable_by(conn, &actor)? => Ok(Some(thread)),
                _ => Ok(None),
            }
        })
        .await
        .map_err(db_error)?
        .ok_or(campfire_kit::Error::NotFound)?;
    if !thread.work() {
        return Err(fail(
            c,
            validation("base", "This thread isn't tracked as work"),
        ));
    }
    Ok((thread, viewer))
}

async fn link_form(c: &mut Ctx) -> Result {
    let (thread, _) = scope(c).await?;
    let at = now(c);
    let events = c
        .app()
        .db
        .read(move |conn| {
            Ok(
                CalendarEvent::work_link_candidates(conn, thread.room_id, thread.id, at)?
                    .into_iter()
                    .map(|event| api::WorkLinkEventCandidate {
                        id: event.id,
                        title: event.title,
                        starts_at: crate::dto::time(event.starts_at),
                        time_zone: event.time_zone,
                    })
                    .collect(),
            )
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &api::WorkLinkForm { events })
}

async fn create_link(c: &mut Ctx) -> Result {
    let (thread, viewer) = scope(c).await?;
    // Validate the discriminator before typed decoding so absent/unknown kinds retain the
    // classic alert and its input field instead of serde's enum error.
    let raw: serde_json::Value = body(c).await?;
    let kind = raw.get("kind").and_then(serde_json::Value::as_str);
    if !kind.is_some_and(|kind| campfire_db::models::work_thread_link::KINDS.contains(&kind)) {
        return Err(fail(
            c,
            validation(
                "kind",
                "Choose a pull request, event, or Drive file to link.",
            ),
        ));
    }
    let input: api::CreateWorkLink = serde_json::from_value(raw).map_err(|error| {
        campfire_kit::Error::BadRequest(format!("The request body isn't valid: {error}"))
    })?;
    let thread_id = thread.id;
    let mut attributes = NewWorkThreadLink {
        channel_thread_id: thread_id,
        created_by_id: viewer.id,
        ..Default::default()
    };
    let field = match input.kind {
        api::WorkLinkKind::PullRequest => {
            attributes.kind = Some("pull_request".into());
            let Some((owner, repo, number)) =
                references::extract(input.pull_request_url.as_deref().unwrap_or_default())
                    .into_iter()
                    .next()
            else {
                return Err(fail(
                    c,
                    validation(
                        "pullRequestUrl",
                        "Enter a GitHub pull request URL, like https://github.com/owner/repo/pull/123.",
                    ),
                ));
            };
            let number = number
                .parse::<i64>()
                .map_err(campfire_kit::Error::internal)?;
            let pr = match c
                .app()
                .db
                .write(move |tx| PullRequest::for_reference(tx, &owner, &repo, number))
                .await
            {
                Ok(pr) => pr,
                Err(campfire_db::Error::RecordInvalid(errors)) => {
                    return Err(fail(
                        c,
                        validation(
                            "pullRequestUrl",
                            &campfire_views::helpers::to_sentence(&errors.full_messages(), " and "),
                        ),
                    ));
                }
                Err(error) => return Err(db_error(error)),
            };
            attributes.github_pull_request_id = Some(pr.id);
            "pullRequestUrl"
        }
        api::WorkLinkKind::Event => {
            attributes.kind = Some("event".into());
            let Some(id) = input.event_id else {
                return Err(fail(c, validation("eventId", "Choose an event to link.")));
            };
            let room_id = thread.room_id;
            let event = c
                .app()
                .db
                .read(move |conn| CalendarEvent::find(conn, id))
                .await
                .map_err(db_error)?;
            if event.room_id != room_id {
                return Err(fail(c, not_found()));
            }
            attributes.event_id = Some(id);
            "eventId"
        }
        api::WorkLinkKind::DriveFile => {
            attributes.kind = Some("drive_file".into());
            let raw = input.drive_url.unwrap_or_default();
            let url = campfire_richtext::ruby::strip(&raw);
            let Some(file_id) =
                campfire_db::models::google_drive_link::file_id(&serde_json::json!(url))
            else {
                return Err(fail(
                    c,
                    validation(
                        "driveUrl",
                        "Enter a Google Drive, Docs, Sheets, Slides, or Forms link.",
                    ),
                ));
            };
            attributes.title = resolve_drive_title(c, &file_id).await;
            attributes.url = Some(url.into());
            "driveUrl"
        }
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
            return Err(fail(
                c,
                validation(field, "That is already linked to this work thread."),
            ));
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
            return Err(fail(c, validation(field, &message)));
        }
        Err(error) => return Err(db_error(error)),
    }
    // WorkThreadLink emits ThreadWorkChange, whose after-commit hook publishes thread.updated.
    let detail = crate::threads::detail(c, viewer, thread_id).await?;
    c.json(StatusCode::CREATED, &detail)
}

async fn destroy_link(c: &mut Ctx) -> Result {
    let (thread, viewer) = scope(c).await?;
    let thread_id = thread.id;
    let id = c.param_str("id").and_then(concerns::cast_integer);
    let deleted = c
        .app()
        .db
        .write(move |tx| {
            let link = id
                .map(|id| WorkThreadLink::find(tx.conn(), id))
                .transpose()?
                .flatten();
            let Some(link) = link.filter(|link| link.channel_thread_id == thread_id) else {
                return Ok(false);
            };
            link.destroy(tx)?;
            Ok(true)
        })
        .await
        .map_err(db_error)?;
    if !deleted {
        return Err(fail(c, not_found()));
    }
    let detail = crate::threads::detail(c, viewer, thread_id).await?;
    c.json(StatusCode::OK, &detail)
}
