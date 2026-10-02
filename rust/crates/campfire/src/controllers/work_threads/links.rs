//! Work links use the shared PR identity/fetch APIs. Google owns credentialed Drive reads.
use super::*;
use crate::integrations::github::{
    jobs::FetchPullRequestJob, pull_requests::PullRequest, references,
};
use askama::Template;
use campfire_db::{Event, NewWorkThreadLink, WorkThreadLink};
use futures_util::future::BoxFuture;
use std::sync::{Arc, LazyLock, RwLock};

/// WS14g installs its real Google::Client adapter, including configured/account/Drive policy.
/// Errors are deliberately suppressed by LinksController#resolve_drive_title in Rails.
pub trait DriveTitle: Send + Sync {
    fn title<'a>(
        &'a self,
        user_id: i64,
        file_id: &'a str,
    ) -> BoxFuture<'a, campfire_db::Result<Option<String>>>;
}
#[derive(Default)]
pub struct DriveTitles(RwLock<Option<Arc<dyn DriveTitle>>>);
impl DriveTitles {
    #[allow(dead_code)] // WS14g's production installer is pending its Google client merge.
    pub fn install(&self, adapter: Arc<dyn DriveTitle>) {
        *self.0.write().unwrap_or_else(|p| p.into_inner()) = Some(adapter);
    }
    async fn title(&self, user_id: i64, file_id: &str) -> Option<String> {
        let adapter = self.0.read().unwrap_or_else(|p| p.into_inner()).clone();
        match adapter {
            Some(adapter) => adapter
                .title(user_id, file_id)
                .await
                .ok()
                .flatten()
                .filter(|s| !campfire_richtext::ruby::is_blank(s)),
            None => None, // WS14g: wire credentialed resolution when its Google client merges.
        }
    }
}
/// Google::DriveLink's four pinned patterns; replace this seam with WS14g's parser on merge.
pub fn drive_file_id(url: &str) -> Option<String> {
    static PATTERNS: LazyLock<Vec<regex::Regex>> = LazyLock::new(|| {
        [
        r"(?i)^https://docs\.google\.com/(?:u/[0-9]+/)?(?:document|spreadsheets|presentation|forms)/(?:u/[0-9]+/)?d/([A-Za-z0-9_-]{10,})",
        r"(?i)^https://drive\.google\.com/(?:u/[0-9]+/)?file/(?:u/[0-9]+/)?d/([A-Za-z0-9_-]{10,})",
        r"(?i)^https://drive\.google\.com/(?:u/[0-9]+/)?drive/(?:u/[0-9]+/)?folders/([A-Za-z0-9_-]{10,})",
        r"(?i)^https://drive\.google\.com/(?:u/[0-9]+/)?open\?(?:[^#]*&)?id=([A-Za-z0-9_-]{10,})(?:&|#|\z)",
    ].iter().map(|pattern|regex::Regex::new(pattern).unwrap()).collect()
    });
    PATTERNS
        .iter()
        .find_map(|pattern| pattern.captures(url).map(|capture| capture[1].to_string()))
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
            let Some(file_id) = drive_file_id(url) else {
                return Ok(Build::Invalid(
                    "Enter a Google Drive, Docs, Sheets, Slides, or Forms link.".into(),
                ));
            };
            attributes.title = c
                .app()
                .work_link_drive_titles
                .title(user_id, &file_id)
                .await;
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
    let link = match c
        .app()
        .db
        .write(move |tx| WorkThreadLink::create(tx, attributes))
        .await
    {
        Ok(link) => link,
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
    };
    // Rails saves the link before claiming the separate fetch. A failed fetch leaves the link.
    if let Some(id) = link.github_pull_request_id {
        c.app()
            .db
            .write(move |tx| {
                let mut pr = PullRequest::find(tx.conn(), id)?;
                if pr.claim_fetch_request(tx)? {
                    tx.emit_after_commit(Event::job(&FetchPullRequestJob {
                        pull_request_id: id,
                    }));
                }
                Ok(())
            })
            .await
            .map_err(db_error)?;
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
    if *c.respond_to(&[&format::TURBO_STREAM, &format::HTML])? == format::TURBO_STREAM {
        let thread = thread.clone();
        let id = thread.id;
        let links = messages::present(c, move |p| board_posts::links(p, &thread)).await?;
        page::bare(c, StatusCode::OK, &format::TURBO_STREAM, |ctx| {
            views::LinksChange {
                ctx,
                thread_id: id,
                links: &links,
            }
            .render()
        })
        .await
    } else {
        c.flash().set_notice(notice);
        let path = format!("/rooms/{}/threads/{}", thread.room_id, thread.id);
        if back {
            c.redirect_back_or_to(&path)
        } else {
            c.redirect_to(&path)
        }
    }
}
async fn invalid(c: &mut Ctx, thread: &ChannelThread, error: String) -> Result {
    if *c.respond_to(&[&format::TURBO_STREAM, &format::HTML])? == format::TURBO_STREAM {
        page::bare(
            c,
            StatusCode::UNPROCESSABLE_ENTITY,
            &format::TURBO_STREAM,
            |ctx| {
                views::LinksInvalid {
                    ctx,
                    thread_id: thread.id,
                    error: &error,
                }
                .render()
            },
        )
        .await
    } else {
        c.flash().set_alert(error);
        c.redirect_to(&format!("/rooms/{}/threads/{}", thread.room_id, thread.id))
    }
}
