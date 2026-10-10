//! Board post reads, kept outside the templates and write services.
use super::Presenter;
use crate::integrations::github::pull_requests::PullRequest;
use campfire_db::{
    CalendarEvent, ChannelThread, Connection, Result, Room, Timestamp, User,
    WorkThreadLink,
};
use campfire_presentation::channel_threads::board::{History, Link, Links, NewPost};
use std::collections::{BTreeSet, HashMap};

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
pub fn history(p: &Presenter<'_>, id: i64) -> Result<Vec<History>> {
    history_records(p, campfire_db::WorkThreadEvent::for_thread(p.conn, id)?)
}
pub fn history_records(
    p: &Presenter<'_>,
    records: Vec<campfire_db::WorkThreadEvent>,
) -> Result<Vec<History>> {
    // Rails includes(:actor); preload once, preserving the missing-actor fallback.
    let ids = records
        .iter()
        .filter_map(|record| record.actor_id)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let actors = if ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        User::where_ids(p.conn, &ids)?.into_iter().map(|user| (user.id,user.name)).collect()
    };
    records
        .into_iter()
        .map(|record| {
            let kind = record.event_type;
            let actor = record.actor_id;
            let (from, to) = (record.from_status, record.to_status);
            let (from_id, to_id) = (record.from_owner_id, record.to_owner_id);
            let (from_name, to_name) = (record.from_owner_name, record.to_owner_name);
            let metadata = record.metadata;
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
                    .and_then(|id| actors.get(&id).cloned())
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
        })
        .collect()
}
pub fn links(p: &Presenter<'_>, thread: &ChannelThread) -> Result<Links> {
    let rows = campfire_db::WorkThreadLink::for_thread(p.conn, thread.id)?;
    let items = link_items(p, thread.room_id, rows)?;
    let events = CalendarEvent::work_link_candidates(
        p.conn,
        thread.room_id,
        thread.id,
        Timestamp::from_jiff(p.now),
    )?
    .into_iter()
    .map(|e| {
        let zone = campfire_presentation::time::Zone::for_user(Some(&e.time_zone));
        Ok((
            format!(
                "{} — {}",
                e.title,
                zone.format(e.starts_at.jiff(), "%b %-d, %Y, %-I:%M %p")
            ),
            e.id,
        ))
    })
    .collect::<Result<Vec<_>>>()?;
    Ok(Links { items, events })
}

/// The row context renders linked items, without the panel's unused event choices.
pub fn link_items(
    p: &Presenter<'_>,
    room_id: i64,
    rows: Vec<campfire_db::WorkThreadLink>,
) -> Result<Vec<Link>> {
    LinkSources::load(p.conn, &rows)?.items(room_id, rows)
}

/// Rails includes(:github_pull_request, :event), shared across all links in a render scope.
pub struct LinkSources {
    pull_requests: HashMap<i64, PullRequest>,
    events: HashMap<i64, CalendarEvent>,
}

impl LinkSources {
    pub fn load(conn: &Connection, rows: &[WorkThreadLink]) -> Result<Self> {
        let pr_ids = rows
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
            pull_requests: PullRequest::for_ids(conn, &pr_ids)?
                .into_iter()
                .map(|pr| (pr.id, pr))
                .collect(),
            events: CalendarEvent::for_ids(conn, &event_ids)?
                .into_iter()
                .map(|event| (event.id, event))
                .collect(),
        })
    }

    pub fn items(&self, room_id: i64, rows: Vec<WorkThreadLink>) -> Result<Vec<Link>> {
        let mut items = Vec::new();
        for record in rows {
            let (id, kind, pr, event, url, title) = (
                record.id,
                record.kind,
                record.github_pull_request_id,
                record.event_id,
                record.url,
                record.title,
            );
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
                    let pr = pr
                        .and_then(|id| self.pull_requests.get(&id))
                        .ok_or(campfire_db::Error::RecordNotFound("Github::PullRequest"))?;
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
                    link.state =
                        Some(campfire_presentation::github::state_label(pr.state.as_deref()).into());
                    if pr.private == Some(false) {
                        link.title = pr
                            .title
                            .clone()
                            .filter(|s| !campfire_richtext::ruby::is_blank(s));
                    }
                }
                "event" => {
                    let event = event
                        .and_then(|id| self.events.get(&id))
                        .ok_or(campfire_db::Error::RecordNotFound("Event"))?;
                    link.label = event.title.clone();
                    link.url = format!("/rooms/{room_id}/events/{}", event.id);
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
        Ok(items)
    }
}
