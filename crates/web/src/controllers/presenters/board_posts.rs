//! Board post reads, kept outside the templates and write services.
use super::Presenter;
use campfire_db::{
    ChannelThread, Message, Result, Room, User,
};
use campfire_views::{
    channel_threads::board::Post,
    helpers as h,
};

pub fn post(
    p: &Presenter<'_>,
    room: &Room,
    thread: &ChannelThread,
    viewer: &User,
    records: &[Message],
    picker: bool,
) -> Result<Post> {
    let row = super::boards::rows(p, room, std::slice::from_ref(thread))?.remove(0);
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
        name: Some(thread.name.clone()),
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
pub use campfire_runtime::presenters::board_posts::*;

use crate::controllers::presenters::Rendering;
