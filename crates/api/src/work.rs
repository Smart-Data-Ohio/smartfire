//! The S4 work endpoints on `/api/v1` (`campfire_api_types::work` documents each one), and the
//! work parts of the thread DTOs: [`facts`] for `Thread.work` and [`detail`] for
//! `ThreadDetail.work`. The list reads `work_threads#index`'s query, the writes go through the
//! classic model writers (`ChannelThread#update_work`, `#update_result`, `#hand_off`) under
//! `channel_threads#update`'s and `work_threads#create_handoff`'s checks, and every change
//! publishes `thread.updated` from the model (`ThreadWorkChange`).

use std::collections::{BTreeSet, HashMap};

use campfire_api_types as api;
use campfire_app::app::{AppCtx, AppState};
use campfire_app::integrations::github::pull_requests::PullRequest;
use campfire_db::models::audit_log::{Actor, Context};
use campfire_db::models::channel_thread::{WORK_UPDATE_FORBIDDEN, WorkChanges};
use campfire_db::{
    Agent, CalendarEvent, ChannelThread, Connection, HandoffPackage, Room, Timestamp, User,
    WorkHandoff, WorkThreadEvent, WorkThreadLink,
};
use campfire_kit::{Ctx, Result, StatusCode};
use campfire_runtime::presenters::Presenter;
use campfire_runtime::context::db_error;
use rails_compat::Secrets;

use crate::agents::human;
use crate::dto;
use crate::endpoints::{before_actions, body, now};
use crate::error::{fail, not_found, record_invalid};
use crate::threads::{FORBIDDEN_UPDATE, scope};

endpoint!(
    /// `GET /api/v1/work`
    index => list_work
);
endpoint!(
    /// `PATCH /api/v1/threads/:thread_id/work`
    update => update_work
);
endpoint!(
    /// `POST /api/v1/threads/:thread_id/work/handoff`
    handoff => create_handoff
);

/// The handoff summary a history entry shows (`truncate(200)`).
const HANDOFF_SUMMARY: usize = 200;

fn present(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| !campfire_richtext::ruby::is_blank(value))
        .map(str::to_string)
}

/// `channel_threads.work_status` on the wire; `None` for a value the contract doesn't know.
fn status(value: Option<&str>) -> Option<api::WorkStatus> {
    Some(match value? {
        "planned" => api::WorkStatus::Planned,
        "in_progress" => api::WorkStatus::InProgress,
        "blocked" => api::WorkStatus::Blocked,
        "done" => api::WorkStatus::Done,
        _ => return None,
    })
}

pub(crate) fn stored_status(status: api::WorkStatus) -> &'static str {
    match status {
        api::WorkStatus::Planned => "planned",
        api::WorkStatus::InProgress => "in_progress",
        api::WorkStatus::Blocked => "blocked",
        api::WorkStatus::Done => "done",
    }
}

/// A URL the client may put in an `href`: `https://` only, as the classic page shows `run_url`
/// (`board_posts.rs`).
fn https(url: &str) -> bool {
    url.starts_with("https://")
}

/// A link's URL: `https://`, or a site-relative path (never `//host`, which leaves the site).
fn safe_link(url: &str) -> bool {
    https(url) || (url.starts_with('/') && !url.starts_with("//") && !url.starts_with("/\\"))
}

/// `board_posts::LinkSources`' source preload and link projection, without view types.
struct WorkLinkSources {
    pull_requests: HashMap<i64, PullRequest>,
    events: HashMap<i64, CalendarEvent>,
}

impl WorkLinkSources {
    fn load(conn: &Connection, rows: &[WorkThreadLink]) -> campfire_db::Result<Self> {
        let pull_request_ids = rows
            .iter()
            .filter(|link| link.kind == "pull_request")
            .filter_map(|link| link.github_pull_request_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let event_ids = rows
            .iter()
            .filter(|link| link.kind == "event")
            .filter_map(|link| link.event_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        Ok(Self {
            pull_requests: PullRequest::for_ids(conn, &pull_request_ids)?
                .into_iter()
                .map(|pull_request| (pull_request.id, pull_request))
                .collect(),
            events: CalendarEvent::for_ids(conn, &event_ids)?
                .into_iter()
                .map(|event| (event.id, event))
                .collect(),
        })
    }

    fn items(
        &self,
        room_id: i64,
        rows: Vec<WorkThreadLink>,
    ) -> campfire_db::Result<Vec<api::WorkLink>> {
        let mut links = Vec::with_capacity(rows.len());
        for record in rows {
            let kind = match record.kind.as_str() {
                "pull_request" => api::WorkLinkKind::PullRequest,
                "event" => api::WorkLinkKind::Event,
                "drive_file" => api::WorkLinkKind::DriveFile,
                _ => return Err(campfire_db::Error::Other("Invalid work link kind".into())),
            };
            let mut link = api::WorkLink {
                id: record.id,
                kind,
                label: String::new(),
                url: String::new(),
                pull_request_state: None,
                title: None,
                event_starts_at: None,
                event_time_zone: None,
                event_cancelled: false,
            };
            match kind {
                api::WorkLinkKind::PullRequest => {
                    let pull_request = record
                        .github_pull_request_id
                        .and_then(|id| self.pull_requests.get(&id))
                        .ok_or(campfire_db::Error::RecordNotFound("Github::PullRequest"))?;
                    link.label = format!(
                        "{}#{}",
                        pull_request.display_full_name()?,
                        pull_request.number
                    );
                    link.url = present(pull_request.html_url.as_deref()).unwrap_or_else(|| {
                        format!(
                            "https://github.com/{}/pull/{}",
                            pull_request.full_name(),
                            pull_request.number
                        )
                    });
                    link.pull_request_state = Some(match pull_request.state.as_deref() {
                        Some("merged") => api::WorkPullRequestState::Merged,
                        Some("closed") => api::WorkPullRequestState::Closed,
                        Some("draft") => api::WorkPullRequestState::Draft,
                        _ => api::WorkPullRequestState::Open,
                    });
                    if pull_request.private == Some(false) {
                        link.title = present(pull_request.title.as_deref());
                    }
                }
                api::WorkLinkKind::Event => {
                    let event = record
                        .event_id
                        .and_then(|id| self.events.get(&id))
                        .ok_or(campfire_db::Error::RecordNotFound("Event"))?;
                    link.label = event.title.clone();
                    link.url = format!("/rooms/{room_id}/events/{}", event.id);
                    link.event_starts_at = Some(dto::time(event.starts_at));
                    link.event_time_zone = Some(event.time_zone.clone());
                    link.event_cancelled = event.cancelled();
                }
                api::WorkLinkKind::DriveFile => {
                    link.url = record.url.unwrap_or_default();
                    link.label =
                        present(record.title.as_deref()).unwrap_or_else(|| link.url.clone());
                }
            }
            if safe_link(&link.url) {
                links.push(link);
            }
        }
        Ok(links)
    }
}

/// The work facts of each tracked thread among `threads`, by thread id (`thread_with_facts`'
/// work fields, the board row's links).
pub fn facts(
    conn: &Connection,
    secrets: &Secrets,
    threads: &[ChannelThread],
    now: Timestamp,
) -> campfire_db::Result<HashMap<i64, api::WorkFacts>> {
    let tracked: Vec<ChannelThread> = threads
        .iter()
        .filter(|thread| thread.work() && status(thread.work_status.as_deref()).is_some())
        .cloned()
        .collect();
    if tracked.is_empty() {
        return Ok(HashMap::new());
    }
    let available = ChannelThread::work_owners(conn, &tracked)?;
    let owners: HashMap<i64, api::User> = dto::users(
        conn,
        secrets,
        tracked.iter().filter_map(|thread| thread.work_owner_id),
        now,
    )?
    .into_iter()
    .map(|user| (user.id, user))
    .collect();
    let ids: Vec<i64> = tracked.iter().map(|thread| thread.id).collect();
    // Ordinary threads can store tags through the classic metadata writer, but only boards
    // expose them as work facts. Load the board tags with one query for the whole page.
    let mut statement = conn.prepare_cached(
        "SELECT thread_tags.channel_thread_id, thread_tags.name FROM thread_tags
         JOIN channel_threads ON channel_threads.id=thread_tags.channel_thread_id
         JOIN rooms ON rooms.id=channel_threads.room_id
         WHERE rooms.type='Rooms::Board' AND thread_tags.channel_thread_id IN (SELECT value FROM json_each(?))
         ORDER BY thread_tags.name",
    )?;
    let tags = statement.query_map([serde_json::json!(ids).to_string()], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut grouped_tags: HashMap<i64, Vec<String>> = HashMap::new();
    for tag in tags {
        let (id, name) = tag?;
        grouped_tags.entry(id).or_default().push(name);
    }
    let mut message_counts = ChannelThread::board_reply_counts(conn, &ids)?;
    let rows = WorkThreadLink::for_threads(conn, &ids)?;
    let sources = WorkLinkSources::load(conn, &rows)?;
    let mut grouped: HashMap<i64, Vec<WorkThreadLink>> = HashMap::new();
    for row in rows {
        grouped.entry(row.channel_thread_id).or_default().push(row);
    }
    let mut out = HashMap::with_capacity(tracked.len());
    for thread in &tracked {
        let links = sources.items(
            thread.room_id,
            grouped.remove(&thread.id).unwrap_or_default(),
        )?;
        let Some(status) = status(thread.work_status.as_deref()) else {
            continue;
        };
        out.insert(
            thread.id,
            api::WorkFacts {
                status,
                owner: thread.work_owner_id.and_then(|id| owners.get(&id).cloned()),
                owner_active: available.available(thread),
                run_url: thread.run_url.clone().filter(|url| https(url)),
                result_updated_at: thread.result_updated_at.map(dto::time),
                links,
                tags: grouped_tags.remove(&thread.id).unwrap_or_default(),
                message_count: message_counts.remove(&thread.id).unwrap_or_default(),
                // Owner/link rows can disappear without touching the thread; taking their
                // live timestamps would make this revision go backwards after a deletion.
                updated_at: dto::row_version(thread.updated_at),
            },
        );
    }
    Ok(out)
}

/// The five work flags of `thread_with_facts`' permissions, for a viewer reading the thread from
/// a room they belong to: (convert, manage, update status, assign, remove).
pub fn permissions(thread: &ChannelThread, room: &Room, viewer: &User) -> [bool; 5] {
    let work = thread.work();
    let assignment = thread.work_assignment_manageable_in_room(room, viewer, true);
    let manageable = thread.work_manageable_in_room(room, viewer, true);
    [
        !work && assignment,
        work && manageable,
        work && manageable,
        work && assignment,
        work && assignment && !room.board(),
    ]
}

fn owner_snapshot(id: Option<i64>, name: Option<String>) -> Option<api::WorkOwnerSnapshot> {
    let name = name.filter(|name| !name.is_empty());
    (id.is_some() || name.is_some()).then_some(api::WorkOwnerSnapshot { user_id: id, name })
}

fn history_entry(record: WorkThreadEvent) -> Option<api::WorkHistoryEntry> {
    let kind = match record.event_type.as_str() {
        "work_update" => api::WorkHistoryKind::Update,
        "work_assignment" => api::WorkHistoryKind::Assignment,
        "work_handoff" => api::WorkHistoryKind::Handoff,
        "result_updated" => api::WorkHistoryKind::Result,
        _ => return None,
    };
    let metadata = &record.metadata;
    let text = |key: &str| present(metadata[key].as_str());
    // `board_posts::history_records`' counts: a number, or a string Ruby would cast.
    let number = |key: &str| {
        metadata[key].as_i64().unwrap_or_else(|| {
            metadata[key]
                .as_str()
                .and_then(campfire_runtime::concerns::cast_integer)
                .unwrap_or(0)
        })
    };
    let handoff = (kind == api::WorkHistoryKind::Handoff).then(|| api::WorkHistoryHandoff {
        summary: campfire_richtext::ruby::truncate(
            &text("handoff_summary").unwrap_or_default(),
            HANDOFF_SUMMARY,
            "...",
        ),
        link_count: number("handoff_links_count"),
        question_count: number("handoff_questions_count"),
    });
    Some(api::WorkHistoryEntry {
        id: record.id,
        kind,
        created_at: dto::time(record.created_at),
        actor_id: record.actor_id,
        from_status: status(record.from_status.as_deref()),
        to_status: status(record.to_status.as_deref()),
        from_owner: owner_snapshot(record.from_owner_id, record.from_owner_name),
        to_owner: owner_snapshot(record.to_owner_id, record.to_owner_name),
        note: text("note"),
        handoff,
    })
}

/// The result rendered as the classic board post renders it.
fn result_html(
    conn: &Connection,
    app: &AppState,
    room: &Room,
    source: &str,
) -> campfire_db::Result<String> {
    let body = app
        .db
        .env()
        .rich_text
        .render_markdown(conn, source, room.id)
        .map_err(campfire_db::Error::Other)?;
    let presenter = Presenter::new(conn, app, None);
    let resolver = presenter.resolver();
    let html =
        campfire_runtime::rich_text::markdown_presentation(conn, &body, &resolver.render_context(None))
            .map_err(campfire_db::Error::Other)?;
    Ok(dto::inline_mentions(&html))
}

/// `ThreadDetail.work` for a tracked thread, with the ids of the people it refers to (the
/// result's editor, the history's actors, the owner candidates and the handoff receivers).
pub fn detail(
    conn: &Connection,
    app: &AppState,
    thread: &ChannelThread,
    room: &Room,
    permissions: &api::ThreadPermissions,
) -> campfire_db::Result<(api::WorkDetail, Vec<i64>)> {
    let result_markdown = present(thread.result_markdown.as_deref());
    let result_html = result_markdown
        .as_deref()
        .map(|source| result_html(conn, app, room, source))
        .transpose()?;
    let history: Vec<api::WorkHistoryEntry> = WorkThreadEvent::for_thread(conn, thread.id)?
        .into_iter()
        .filter_map(history_entry)
        .collect();
    let owner_candidates = if permissions.can_assign_work {
        owner_candidates(conn, room.id)?
    } else {
        Vec::new()
    };
    let handoff_receivers = if permissions.can_manage_work {
        let mut receivers = Vec::new();
        for (_, agent_id) in WorkHandoff::receivers_for(conn, thread)? {
            if let Some(agent) = Agent::find(conn, agent_id)? {
                receivers.push(api::WorkHandoffReceiver {
                    agent_id,
                    user_id: agent.user_id,
                });
            }
        }
        receivers
    } else {
        Vec::new()
    };
    let people = thread
        .result_updated_by_id
        .into_iter()
        .chain(history.iter().filter_map(|entry| entry.actor_id))
        .chain(owner_candidates.iter().map(|candidate| candidate.user_id))
        .chain(handoff_receivers.iter().map(|receiver| receiver.user_id))
        .collect();
    Ok((
        api::WorkDetail {
            result_html,
            result_updated_by_id: result_markdown.as_ref().and(thread.result_updated_by_id),
            result_markdown,
            steps: dto::thread_steps(conn, thread.id)?,
            history,
            owner_candidates,
            handoff_receivers,
        },
        people,
    ))
}

/// The classic new-post and work-assignment pickers share the same ordered candidates.
pub(crate) fn owner_candidates(
    conn: &Connection,
    room_id: i64,
) -> campfire_db::Result<Vec<api::WorkOwnerCandidate>> {
    let (humans, agents) = ChannelThread::work_owner_candidates_for(conn, room_id)?;
    let profiles: HashMap<i64, Agent> =
        Agent::for_users(conn, &agents.iter().map(|user| user.id).collect::<Vec<_>>())?
            .into_iter()
            .map(|agent| (agent.user_id, agent))
            .collect();
    Ok(humans
        .iter()
        .chain(&agents)
        .map(|user| {
            let agent = profiles.get(&user.id);
            api::WorkOwnerCandidate {
                user_id: user.id,
                provider: agent.and_then(|agent| present(agent.provider.as_deref())),
                description: agent.and_then(|agent| present(agent.description.as_deref())),
            }
        })
        .collect())
}

async fn list_work(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    // `visible_work_threads` lists nothing for a bot, as the classic page shows it.
    let viewer = campfire_runtime::concerns::require_current_user(c)?.clone();
    // `work_threads#index`: an unknown or missing state is `open`.
    let state = match c.param_str("state").unwrap_or_default() {
        state @ ("all" | "done" | "agents" | "boards") => state.to_string(),
        _ => "open".to_string(),
    };
    let (secrets, now) = (c.app().secrets.clone(), now(c));
    let list = c
        .app()
        .db
        .read(move |conn| {
            let threads = ChannelThread::visible_work_threads(conn, &viewer, &state)?;
            let room_ids: BTreeSet<i64> = threads.iter().map(|thread| thread.room_id).collect();
            let rooms = Room::for_ids(conn, &room_ids.into_iter().collect::<Vec<_>>())?;
            let names = Room::display_names_for(conn, &rooms, Some(&viewer))?;
            let rooms: HashMap<i64, Room> = rooms.into_iter().map(|room| (room.id, room)).collect();
            let mut facts = facts(conn, &secrets, &threads, now)?;
            let mut rows = Vec::with_capacity(threads.len());
            for thread in &threads {
                let Some(room) = rooms.get(&thread.room_id) else {
                    continue;
                };
                let Some(work) = facts.remove(&thread.id) else {
                    continue;
                };
                rows.push(api::WorkListRow {
                    thread: dto::thread(thread, room, now, Some(work)),
                    room_name: names.get(&room.id).cloned().unwrap_or_default(),
                    board: room.board(),
                    updated_at: dto::row_version(thread.updated_at),
                });
            }
            Ok(api::WorkList {
                users: dto::users(
                    conn,
                    &secrets,
                    threads.iter().map(|thread| thread.creator_id),
                    now,
                )?,
                threads: rows,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &list)
}

/// The thread `:thread_id` names, for a human member of its room; a 404 for anyone else
/// (`ChannelThread#work_viewable_by`).
async fn work_scope(c: &mut Ctx) -> Result<(ChannelThread, Room, User)> {
    let (thread, room, viewer) = scope(c).await?;
    if human(c)?.is_none() || !viewer.is_active() {
        return Err(fail(c, not_found()));
    }
    Ok((thread, room, viewer))
}

/// A 422 on `field` whose message is a whole sentence, as the classic page shows it.
fn sentence(field: &str, message: &str) -> api::ApiError {
    api::ApiError::Validation {
        message: message.into(),
        fields: [(field.to_string(), vec![message.to_string()])].into(),
    }
}

/// `work_threads#scope`'s bare 422 for an untracked thread, worded.
const UNTRACKED: &str = "This thread isn't tracked as work";

fn forbidden(c: &mut Ctx, message: &str) -> campfire_kit::Error {
    fail(
        c,
        api::ApiError::Forbidden {
            message: message.into(),
        },
    )
}

async fn update_work(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (thread, _, viewer) = work_scope(c).await?;
    let input: api::UpdateWork = body(c).await?;
    let thread_id = thread.id;
    let actor = viewer.clone();
    let result = c
        .app()
        .db
        .write(move |tx| {
            let mut thread = ChannelThread::find(tx.conn(), thread_id)?;
            // `channel_threads#update`'s work checks, read in its write.
            let manager = thread.work_manageable_by(tx.conn(), &actor)?;
            let assignment = thread.work_assignment_manageable_by(tx.conn(), &actor)?;
            let tags_allowed = if thread.board_post(tx.conn())? {
                manager
            } else {
                assignment
            };
            let allowed = (input.result_markdown.is_none() || manager)
                && (input.tags.is_none() || tags_allowed)
                && ((input.status.is_none() && input.owner_id.is_none()) || manager)
                && (input.owner_id.is_none() || assignment)
                && (input
                    .status
                    .as_ref()
                    .is_none_or(|status| status.is_some() == thread.work_status.is_some())
                    || assignment);
            if !allowed {
                return Err(campfire_db::Error::Other(FORBIDDEN_UPDATE.into()));
            }
            if let Some(tags) = input.tags {
                let before = thread.tag_names(tx.conn())?;
                thread.update_metadata(tx, None, None, Some(&tags))?;
                if before != thread.tag_names(tx.conn())? {
                    campfire_db::models::channel_thread::ThreadWorkChange::emit(tx, thread_id);
                }
            }
            if let Some(markdown) = input.result_markdown {
                thread.update_result(tx, &actor, markdown)?;
            }
            if input.status.is_some() || input.owner_id.is_some() {
                let changes = WorkChanges {
                    status: input
                        .status
                        .map(|status| status.map(|status| stored_status(status).to_string())),
                    owner_id: input.owner_id.map(|owner| {
                        owner.map_or(serde_json::Value::Null, serde_json::Value::from)
                    }),
                };
                thread.update_work(tx, &actor, changes)?;
            }
            Ok(())
        })
        .await;
    match result {
        Ok(()) => {}
        Err(campfire_db::Error::Other(text))
            if text == FORBIDDEN_UPDATE || text == WORK_UPDATE_FORBIDDEN =>
        {
            return Err(forbidden(c, FORBIDDEN_UPDATE));
        }
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            let error = record_invalid(
                &errors,
                &[("work_owner", "ownerId"), ("work_status", "status")],
            );
            return Err(fail(c, error));
        }
        Err(error) => return Err(db_error(error)),
    }
    let detail = crate::threads::detail(c, viewer, thread_id).await?;
    c.json(StatusCode::OK, &detail)
}

async fn create_handoff(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (thread, _, viewer) = work_scope(c).await?;
    // `work_threads#scope`: tracked, then managed by the viewer.
    if !thread.work() {
        return Err(fail(c, sentence("base", UNTRACKED)));
    }
    let (thread_id, sender) = (thread.id, viewer.clone());
    let manageable = c
        .app()
        .db
        .read(move |conn| thread.work_manageable_by(conn, &sender))
        .await
        .map_err(db_error)?;
    if !manageable {
        return Err(forbidden(c, WORK_UPDATE_FORBIDDEN));
    }
    let input: api::CreateWorkHandoff = body(c).await?;
    let context = Context {
        actor: Some(Actor::from(&viewer)),
        ip_address: Some(c.request.remote_ip()?.to_string()),
        user_agent: c.request.user_agent().map(str::to_string),
    };
    let sender = viewer.clone();
    let result = c
        .app()
        .db
        .write(move |tx| {
            let mut thread = ChannelThread::find(tx.conn(), thread_id)?;
            let receiver = Agent::find(tx.conn(), input.receiver_agent_id)?;
            if let Some(error) = WorkHandoff::receiver_error(tx.conn(), &thread, receiver.as_ref())?
            {
                return Ok(Err(error));
            }
            let receiver = receiver.expect("receiver policy passed");
            let package = HandoffPackage {
                summary: input.summary,
                links: serde_json::json!(input.links),
                open_questions: serde_json::json!(input.open_questions),
            };
            thread.hand_off(tx, &sender, &receiver, package, &context)?;
            Ok(Ok(()))
        })
        .await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(message)) => {
            return Err(fail(c, sentence("receiverAgentId", message)));
        }
        Err(campfire_db::Error::Other(text)) if text == WORK_UPDATE_FORBIDDEN => {
            return Err(forbidden(c, WORK_UPDATE_FORBIDDEN));
        }
        // `check_handoff_target`, when the thread changed under the request.
        Err(campfire_db::Error::RecordNotFound("Work thread is not tracked")) => {
            return Err(fail(c, sentence("base", UNTRACKED)));
        }
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            let error = record_invalid(&errors, &[("work_owner", "receiverAgentId")]);
            return Err(fail(c, error));
        }
        Err(error) => return Err(db_error(error)),
    }
    let detail = crate::threads::detail(c, viewer, thread_id).await?;
    c.json(StatusCode::CREATED, &detail)
}
