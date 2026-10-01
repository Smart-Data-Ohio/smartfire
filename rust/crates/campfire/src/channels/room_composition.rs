//! Room directory and OOO partials behind WS8's existing callback descriptions.
//! Personal rows are rendered for the named membership, never for the sender.
use crate::{
    app::App,
    controllers::{presenters::page, users::sidebars::composition},
};
use askama::Template;
use campfire_cable::turbo::{Action, Target};
use campfire_db::{
    Account, Membership, Room, User,
    broadcasts::{Broadcast, Partial, TurboAction},
};
pub(super) fn deliver(app: &App, broadcast: &Broadcast) -> anyhow::Result<bool> {
    let Broadcast::Turbo(frame) = broadcast else {
        return Ok(false);
    };
    // The native directory renderer owns ordinary room rows and headers. The
    // huddle composition adds the participant and call controls only when Rails'
    // Huddle.configured? branch is active.
    if matches!(
        frame.partial,
        Some(Partial::DirectSidebar { .. } | Partial::RoomHeader { .. })
    ) && !app.config.huddle.configured()
    {
        return Ok(false);
    }
    if !matches!(
        frame.partial,
        Some(
            Partial::DirectSidebar { .. } | Partial::RoomHeader { .. } | Partial::OooNotice { .. }
        )
    ) {
        return Ok(false);
    }
    let partial = frame.partial.clone().unwrap();
    let copy = app.clone();
    let html = app.db.read_blocking(move |conn| {
        let account = Account::first(conn)?;
        match partial {
            Partial::DirectSidebar {
                membership_id,
                member_ids,
            } => {
                let membership = Membership::find(conn, membership_id)?;
                let row = composition::for_membership(&copy, conn, &membership, Some(&member_ids))?;
                Ok(page::render_detached(&copy, account.as_ref(), |ctx| {
                    row.render_fragment(ctx, copy.config.huddle.configured())
                }))
            }
            Partial::RoomHeader {
                room_id,
                for_user_id,
            } => {
                let room = Room::find(conn, room_id)?;
                let viewer = User::find(conn, for_user_id)?;
                let nav =
                    crate::controllers::rooms::call_navigation::model(&copy, conn, &room, &viewer)?;
                Ok(page::render_detached(&copy, account.as_ref(), |ctx| {
                    nav.identity(ctx)
                }))
            }
            Partial::OooNotice { user_id } => {
                let notice =
                    crate::controllers::rooms::shell::notice(conn, user_id, copy.db.env().now())?;
                Ok(campfire_views::rooms::shell::NoticeLine { user: &notice }
                    .render()
                    .expect("OOO notice line"))
            }
            _ => unreachable!(),
        }
    })?;
    let action = match frame.action {
        TurboAction::Append => Action::Append,
        TurboAction::Prepend => Action::Prepend,
        TurboAction::Replace => Action::Replace,
        TurboAction::Update => Action::Update,
        TurboAction::Remove => Action::Remove,
    };
    let streams: Vec<_> = frame.streamables.iter().map(|s| s.to_param()).collect();
    let streams: Vec<_> = streams.iter().map(String::as_str).collect();
    let extra = if frame.maintain_scroll {
        vec![("maintain_scroll", Some("true"))]
    } else {
        Vec::new()
    };
    app.cable.broadcast_action_to(
        &streams,
        action,
        Target::Target(&frame.target),
        Some(&html),
        &extra,
    );
    Ok(true)
}
