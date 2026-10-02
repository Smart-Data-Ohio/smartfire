//! Shared board row facts for HTTP lists and background Turbo broadcasts.
use super::Presenter;
use campfire_db::{ChannelThread, ThreadTag, Timestamp, User};
use campfire_views::rooms::boards::{Listing, Row};
use std::collections::HashMap;

fn owner_label(
    owners: &campfire_db::models::channel_thread::WorkOwners,
    post: &ChannelThread,
) -> (String, bool) {
    let owner = owners.owner(post);
    let label = match owner {
        Some(owner) if owners.available(post) => owner.name.clone(),
        Some(owner) => format!("Owner unavailable ({})", owner.name),
        None => "Unassigned".into(),
    };
    (label, owner.is_some_and(User::is_bot))
}

/// The ordinary work pane uses only the owner facts, not the board row's counts or tags.
pub fn owner(p: &Presenter<'_>, post: &ChannelThread) -> campfire_db::Result<(String, bool)> {
    let owners = ChannelThread::work_owners(p.conn, std::slice::from_ref(post))?;
    Ok(owner_label(&owners, post))
}

pub fn rows(
    p: &Presenter<'_>,
    room_id: i64,
    posts: &[ChannelThread],
) -> campfire_db::Result<Vec<Row>> {
    let ids = posts.iter().map(|post| post.id).collect::<Vec<_>>();
    let replies = ChannelThread::board_reply_counts(p.conn, &ids)?;
    let links = ChannelThread::board_link_counts(p.conn, &ids)?;
    let owners = ChannelThread::work_owners(p.conn, posts)?;
    let mut tags: HashMap<i64, Vec<String>> = HashMap::new();
    for tag in ThreadTag::for_threads(p.conn, &ids)? {
        tags.entry(tag.channel_thread_id)
            .or_default()
            .push(tag.name);
    }
    posts
        .iter()
        .map(|post| {
            let (owner_label, agent) = owner_label(&owners, post);
            Ok(Row {
                id: post.id,
                room_id,
                name: post.name.clone(),
                work_status: post.work_status.clone().unwrap_or_default(),
                work_label: post.work_status_label(),
                lifecycle: post
                    .status(p.conn, Timestamp::from_jiff(p.now))?
                    .name()
                    .into(),
                owner_id: post.work_owner_id,
                owner_label,
                agent,
                tags: tags.remove(&post.id).unwrap_or_default(),
                replies: replies.get(&post.id).copied().unwrap_or_default(),
                links: links.get(&post.id).copied().unwrap_or_default(),
                updated_at: post.last_activity_at.jiff(),
            })
        })
        .collect()
}

pub struct Filters {
    pub board_view: bool,
    pub status: String,
    pub owner: String,
    pub tag: String,
    pub page: i64,
}

pub fn listing(
    p: &Presenter<'_>,
    room: &campfire_db::Room,
    viewer: &User,
    filters: Filters,
) -> campfire_db::Result<Listing> {
    let Filters {
        board_view,
        status,
        owner,
        tag,
        page,
    } = filters;
    use rusqlite::OptionalExtension;
    let posts = ChannelThread::board_posts_for(
        p.conn,
        room.id,
        if board_view { "all" } else { &status },
        &owner,
        &tag,
        Some(viewer.id),
        page,
    )?;
    let has_more =
        posts.len() > (page * campfire_db::models::channel_thread::BOARD_POSTS_PER_PAGE) as usize;
    let posts = posts
        .into_iter()
        .take((page * campfire_db::models::channel_thread::BOARD_POSTS_PER_PAGE) as usize)
        .collect::<Vec<_>>();
    let mut owner_options = vec![
        ("Anyone".into(), "anyone".into()),
        ("Me".into(), "me".into()),
        ("Agents".into(), "agents".into()),
    ];
    let member_ids = room.user_ids(p.conn)?;
    owner_options.extend(
        User::active_ordered(p.conn)?
            .into_iter()
            .filter(|user| member_ids.contains(&user.id))
            .map(|user| {
                (
                    if user.is_bot() {
                        format!("{} (agent)", user.name)
                    } else {
                        user.name
                    },
                    user.id.to_string(),
                )
            }),
    );
    let digest = p.conn.query_row("SELECT digest_on, message_id FROM board_stale_digests WHERE room_id=? ORDER BY digest_on DESC LIMIT 1", [room.id], |r| Ok((r.get::<_,String>(0)?, r.get::<_,Option<i64>>(1)?))).optional()?;
    let digest = match digest {
        Some((date, Some(id))) => campfire_db::Message::find_by_id(p.conn, id)?
            .map(|message| -> campfire_db::Result<_> {
                let date: jiff::civil::Date = date
                    .parse()
                    .map_err(|error: jiff::Error| campfire_db::Error::Other(error.to_string()))?;
                Ok((
                    date.strftime("%B %-d, %Y").to_string(),
                    message.plain_text_body(p.conn, p.app().db.env().rich_text.as_ref())?,
                ))
            })
            .transpose()?,
        _ => None,
    };
    Ok(Listing {
        room: p.room_view(room, viewer)?,
        board_view,
        status,
        owner,
        tag,
        current_user_id: viewer.id,
        can_administer: viewer.can_administer(Some(room.creator_id), false),
        posts: rows(p, room.id, &posts)?,
        owner_options,
        tag_counts: ChannelThread::board_tag_counts(p.conn, room.id)?,
        any_posts: ChannelThread::board_has_posts(p.conn, room.id)?,
        has_more,
        page,
        digest,
        stream_name: rails_compat::turbo::signed_stream_name(
            &p.app().secrets,
            &[&crate::channels::room_gid(room).to_param(), "messages"],
        ),
    })
}
