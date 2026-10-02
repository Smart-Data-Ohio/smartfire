//! WorkThreadsController's includes: gather each page's row facts before rendering.
use super::{Presenter, board_posts};
use campfire_db::{ChannelThread, Result, Room, User, WorkThreadLink};
use campfire_views::{channel_threads::board::Links, work_threads::Row};
use std::collections::{BTreeSet, HashMap};

pub fn rows(p: &Presenter<'_>, threads: &[ChannelThread], viewer: &User) -> Result<Vec<Row>> {
    let ids = threads.iter().map(|thread| thread.id).collect::<Vec<_>>();
    let counts = ChannelThread::board_reply_counts(p.conn, &ids)?;
    let room_ids = threads
        .iter()
        .map(|thread| thread.room_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let rooms = Room::for_ids(p.conn, &room_ids)?;
    let names = Room::display_names_for(p.conn, &rooms, Some(viewer))?;
    let rooms = rooms
        .into_iter()
        .map(|room| (room.id, room))
        .collect::<HashMap<_, _>>();
    let owners = ChannelThread::work_owners(p.conn, threads)?;
    let records = WorkThreadLink::for_threads(p.conn, &ids)?;
    let sources = board_posts::LinkSources::load(p.conn, &records)?;
    let mut links: HashMap<i64, Vec<WorkThreadLink>> = HashMap::new();
    for link in records {
        links.entry(link.channel_thread_id).or_default().push(link);
    }
    threads
        .iter()
        .map(|thread| {
            let room = rooms
                .get(&thread.room_id)
                .ok_or(campfire_db::Error::RecordNotFound("Room"))?;
            let owner = owners.owner(thread);
            let owner_label = match owner {
                Some(owner) if owners.available(thread) => {
                    format!("Owner: {}", owner.name)
                }
                Some(owner) => format!("Owner unavailable ({})", owner.name),
                None => "Unassigned".into(),
            };
            Ok(Row {
                id: thread.id,
                name: thread.name.clone(),
                path: if room.board() {
                    format!("/rooms/{}/threads/{}", room.id, thread.id)
                } else {
                    format!("/rooms/{}?thread={}", room.id, thread.id)
                },
                status: thread.work_status.clone().unwrap_or_default(),
                status_label: thread.work_status_label(),
                room_name: names[&thread.room_id].clone(),
                owner_label,
                agent: owner.is_some_and(User::is_bot),
                count: counts.get(&thread.id).copied().unwrap_or_default(),
                updated_at: thread.updated_at.jiff(),
                links: Links {
                    items: sources.items(room.id, links.remove(&thread.id).unwrap_or_default())?,
                    events: Vec::new(),
                },
            })
        })
        .collect()
}
