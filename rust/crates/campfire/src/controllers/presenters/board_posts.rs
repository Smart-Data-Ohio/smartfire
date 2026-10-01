//! Board post reads, kept outside the templates and write services.
use super::Presenter;
use campfire_db::{CalendarEvent, ChannelThread, Message, Result, Room, Timestamp, User};
use campfire_views::{
    channel_threads::board::{History, Link, Links, NewPost, Post},
    helpers as h,
};
use rusqlite::params;
use serde_json::Value;

pub fn new_post(p: &Presenter<'_>, room: &Room, viewer: &User) -> Result<NewPost> {
    let (humans, agents) = ChannelThread::work_owner_candidates_for(p.conn, room.id)?;
    Ok(NewPost {
        room_id: room.id,
        room_name: p.room_display_name(room, Some(viewer))?,
        status: "planned".into(),
        humans: humans.into_iter().map(|u| (u.name, u.id)).collect(),
        agents: agents.into_iter().map(|u| (u.name, u.id)).collect(),
        tag_suggestions: ChannelThread::board_tag_counts(p.conn, room.id)?
            .into_iter()
            .map(|(name, _)| name)
            .collect(),
        ..Default::default()
    })
}

pub fn post(
    p: &Presenter<'_>,
    room: &Room,
    thread: &ChannelThread,
    viewer: &User,
    records: &[Message],
    picker: bool,
) -> Result<Post> {
    let row = super::boards::rows(p, room.id, std::slice::from_ref(thread))?.remove(0);
    let choices = new_post(p, room, viewer)?;
    let result = thread
        .result_markdown
        .as_deref()
        .filter(|text| !campfire_richtext::ruby::is_blank(text))
        .map(|source| {
            let body = p
                .app()
                .db
                .env()
                .rich_text
                .render_markdown(p.conn, source, room.id)
                .map_err(campfire_db::Error::Other)?;
            let resolver = p.resolver();
            crate::rich_text::markdown_presentation(
                p.conn,
                &body,
                &resolver.render_context(p.request_host.clone()),
            )
            .map(h::raw)
            .map_err(campfire_db::Error::Other)
        })
        .transpose()?;
    Ok(Post {
        id: thread.id,
        room_id: room.id,
        room_name: choices.room_name,
        room_updated_at: room.updated_at.jiff(),
        name: thread.name.clone(),
        lifecycle: row.lifecycle,
        count: row.replies,
        status: row.work_status,
        status_label: row.work_label,
        owner_label: row.owner_label,
        owner_agent: row.agent,
        owner_id: thread.work_owner_id,
        tags: row.tags,
        run_url: thread
            .run_url
            .clone()
            .filter(|url| url.starts_with("https://")),
        can_manage: thread.work_manageable_by(p.conn, viewer)?,
        can_assign: thread.work_assignment_manageable_by(p.conn, viewer)?,
        can_lifecycle: thread.manageable_by(p.conn, viewer)?,
        joined: thread.membership_for(p.conn, viewer.id)?.is_some(),
        humans: choices.humans,
        agents: choices.agents,
        result,
        result_markdown: thread.result_markdown.clone(),
        result_at: thread.result_updated_at.map(|at| at.jiff()),
        result_by: thread
            .result_updated_by_id
            .map(|id| p.user(id).map(|u| u.name))
            .transpose()?,
        user: p.user_view(viewer.id)?,
        messages: p.messages(records)?,
        steps: p.thread_steps(thread.id)?,
        composer: p.composer_facts(
            room,
            viewer,
            Some(thread),
            p.composer_drive_flow(viewer, picker && !viewer.is_bot())?,
        )?,
        history: history(p, thread.id)?,
        links: links(p, thread)?,
        error: None,
    })
}
fn history(p: &Presenter<'_>, id: i64) -> Result<Vec<History>> {
    let mut statement=p.conn.prepare("SELECT event_type,actor_id,from_status,to_status,from_owner_id,to_owner_id,from_owner_name,to_owner_name,metadata FROM work_thread_events WHERE channel_thread_id=? ORDER BY created_at DESC,id DESC")?;
    let records = statement
        .query_map([id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<i64>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<i64>>(4)?,
                r.get::<_, Option<i64>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, Option<String>>(8)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    records
        .into_iter()
        .map(
            |(kind, actor, from, to, from_id, to_id, from_name, to_name, metadata)| {
                let metadata: Value = metadata
                    .as_deref()
                    .map(serde_json::from_str)
                    .transpose()
                    .map_err(|e| campfire_db::Error::Other(e.to_string()))?
                    .unwrap_or(Value::Null);
                let text = |key: &str| {
                    metadata[key]
                        .as_str()
                        .filter(|s| !campfire_richtext::ruby::is_blank(s))
                        .map(str::to_owned)
                };
                let number = |key: &str| {
                    metadata[key].as_i64().unwrap_or_else(|| {
                        metadata[key]
                            .as_str()
                            .and_then(crate::concerns::cast_integer)
                            .unwrap_or(0)
                    })
                };
                let status = |s: &Option<String>| {
                    match s.as_deref() {
                        Some("planned") => "Planned",
                        Some("in_progress") => "In progress",
                        Some("blocked") => "Blocked",
                        Some("done") => "Done",
                        _ => "Ordinary thread",
                    }
                    .to_string()
                };
                Ok(History {
                    kind,
                    actor: actor
                        .map(|id| User::find_by_id(p.conn, id))
                        .transpose()?
                        .flatten()
                        .map(|u| u.name)
                        .unwrap_or("Former member".into()),
                    from_status: status(&from),
                    to_status: status(&to),
                    status_changed: from != to,
                    owner_changed: from_id != to_id,
                    from_owner: from_name.unwrap_or("Unassigned".into()),
                    to_owner: to_name.unwrap_or("Unassigned".into()),
                    note: text("note"),
                    summary: text("handoff_summary"),
                    links: number("handoff_links_count"),
                    questions: number("handoff_questions_count"),
                })
            },
        )
        .collect()
}
fn links(p: &Presenter<'_>, thread: &ChannelThread) -> Result<Links> {
    let mut stmt=p.conn.prepare("SELECT id,kind,github_pull_request_id,event_id,url,title FROM work_thread_links WHERE channel_thread_id=? ORDER BY id")?;
    let rows = stmt
        .query_map([thread.id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut items = Vec::new();
    for (id, kind, pr, event, url, title) in rows {
        let mut link = Link {
            id,
            kind: kind.clone(),
            label: String::new(),
            url: String::new(),
            remove_label: String::new(),
            state: None,
            title: None,
            event_time: None,
            event_zone: None,
            cancelled: false,
        };
        match kind.as_str() {
            "pull_request" => {
                let pr = crate::integrations::github::pull_requests::PullRequest::find(
                    p.conn,
                    pr.ok_or(campfire_db::Error::RecordNotFound("Github::PullRequest"))?,
                )?;
                link.label = format!("{}#{}", pr.display_full_name()?, pr.number);
                link.url = pr
                    .html_url
                    .as_deref()
                    .filter(|s| !campfire_richtext::ruby::is_blank(s))
                    .map(str::to_owned)
                    .unwrap_or_else(|| {
                        format!("https://github.com/{}/pull/{}", pr.full_name(), pr.number)
                    });
                link.remove_label = format!(
                    "Remove link to pull request {}#{}",
                    pr.full_name(),
                    pr.number
                );
                link.state = Some(campfire_views::github::state_label(pr.state.as_deref()).into());
                if pr.private == Some(false) {
                    link.title = pr.title.filter(|s| !campfire_richtext::ruby::is_blank(s));
                }
            }
            "event" => {
                let event = CalendarEvent::find(
                    p.conn,
                    event.ok_or(campfire_db::Error::RecordNotFound("Event"))?,
                )?;
                link.label = event.title.clone();
                link.url = format!("/rooms/{}/events/{}", thread.room_id, event.id);
                link.remove_label = format!("Remove link to event {}", event.title);
                link.event_time = Some(event.starts_at.jiff());
                link.event_zone = Some(event.time_zone.clone());
                link.cancelled = event.cancelled();
            }
            "drive_file" => {
                link.url = url.unwrap_or_default();
                link.label = title
                    .filter(|s| !campfire_richtext::ruby::is_blank(s))
                    .unwrap_or_else(|| link.url.clone());
                link.remove_label = format!("Remove link to Drive file {}", link.label);
            }
            _ => return Err(campfire_db::Error::Other("Invalid work link kind".into())),
        }
        items.push(link);
    }
    let mut stmt=p.conn.prepare("SELECT id FROM events WHERE room_id=? AND cancelled_at IS NULL AND COALESCE(ends_at,starts_at)>=? AND id NOT IN (SELECT event_id FROM work_thread_links WHERE channel_thread_id=? AND event_id IS NOT NULL) ORDER BY starts_at,id")?;
    let ids = stmt
        .query_map(
            params![thread.room_id, Timestamp::from_jiff(p.now), thread.id],
            |r| r.get::<_, i64>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let events = ids
        .into_iter()
        .map(|id| {
            let e = CalendarEvent::find(p.conn, id)?;
            let zone = campfire_views::time::Zone::for_user(Some(&e.time_zone));
            Ok((
                format!(
                    "{} — {}",
                    e.title,
                    zone.format(e.starts_at.jiff(), "%b %-d, %Y, %-I:%M %p")
                ),
                id,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Links { items, events })
}
