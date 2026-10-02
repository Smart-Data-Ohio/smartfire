//! WorkThreadsController's includes: gather each page's row facts before rendering.
use super::{Presenter, board_posts};
use campfire_db::{ChannelThread, Result, Room, User, WorkThreadLink};
use campfire_views::{channel_threads::board::Links, work_threads::Row};
use std::collections::{BTreeSet, HashMap};

pub fn rows(p: &Presenter<'_>, threads: &[ChannelThread], viewer: &User) -> Result<Vec<Row>> {
    let ids = threads.iter().map(|thread| thread.id).collect::<Vec<_>>();
    let counts = ChannelThread::board_reply_counts(p.conn, &ids)?;
    let owner_ids = threads
        .iter()
        .filter_map(|thread| thread.work_owner_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let owners = User::where_ids(p.conn, &owner_ids)?
        .into_iter()
        .map(|user| (user.id, user))
        .collect::<HashMap<_, _>>();
    let mut groups: HashMap<i64, Vec<&ChannelThread>> = HashMap::new();
    for thread in threads {
        groups.entry(thread.room_id).or_default().push(thread);
    }
    let mut rooms = HashMap::new();
    for (room_id, group) in groups {
        let room = Room::find(p.conn, room_id)?;
        let name = p.room_display_name(&room, Some(viewer))?;
        let available = ChannelThread::board_owner_active_map(p.conn, room_id, group)?;
        rooms.insert(room_id, (room, name, available));
    }
    let mut links: HashMap<i64, Vec<WorkThreadLink>> = HashMap::new();
    for link in WorkThreadLink::for_threads(p.conn, &ids)? {
        links.entry(link.channel_thread_id).or_default().push(link);
    }
    threads
        .iter()
        .map(|thread| {
            let (room, room_name, available) = &rooms[&thread.room_id];
            let owner = thread.work_owner_id.and_then(|id| owners.get(&id));
            let owner_label = match owner {
                Some(owner) if available.get(&owner.id).copied().unwrap_or(false) => {
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
                room_name: room_name.clone(),
                owner_label,
                agent: owner.is_some_and(User::is_bot),
                count: counts.get(&thread.id).copied().unwrap_or_default(),
                updated_at: thread.updated_at.jiff(),
                links: Links {
                    items: board_posts::link_items(
                        p,
                        room.id,
                        links.remove(&thread.id).unwrap_or_default(),
                    )?,
                    events: Vec::new(),
                },
            })
        })
        .collect()
}
