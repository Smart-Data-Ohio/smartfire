//! WS8br's directory partial seam. WS8b-m registers message/list partials separately.
use crate::{
    app::App,
    controllers::presenters::{accounts, page, rooms_directory},
};
use askama::Template;
use campfire_db::{Account, Membership, Room, User, broadcasts::Partial};

/// Read committed rows on the committing thread; no actor/request/session enters these views.
pub(super) fn render(app: &App, partial: &Partial) -> anyhow::Result<Option<String>> {
    match partial {
        Partial::UserStatus { user_id } => app
            .db
            .read_blocking(|conn| {
                let status = crate::controllers::presenters::status_settings::profile_status(
                    conn,
                    &app.secrets,
                    *user_id,
                    *user_id,
                    app.db.env().now(),
                )?;
                campfire_views::users::statuses::StatusBadge {
                    presence: &status.presence,
                    status_text: status.status_text.as_deref(),
                }
                .render()
                .map_err(|error| campfire_db::Error::Other(error.to_string()))
            })
            .map(Some)
            .map_err(Into::into),
        Partial::DirectSidebar {
            membership_id,
            member_ids,
        } => {
            app.db
                .read_blocking(|conn| {
                    let membership = Membership::find(conn, *membership_id)?;
                    let room = Room::find(conn, membership.room_id)?;
                    let mut row = accounts::sidebar_direct(conn, &app.secrets, &membership, &room)?;
                    // The descriptor selects explicit members; Rails renders their fresh
                    // Membership association order, which differs from the domain's user join.
                    if member_ids.is_empty() {
                        row.members = vec![crate::controllers::presenters::user_summary(
                            &app.secrets,
                            &User::find(conn, membership.user_id)?,
                        )];
                    } else {
                        row.members.retain(|member| member_ids.contains(&member.id));
                    }
                    row.label = accounts::sidebar_direct_label(room.name.as_deref(), &row.members);
                    row.menu.menu_room_label = Some(row.label.clone());
                    let account = Account::first(conn)?;
                    Ok(page::render_detached(app, account.as_ref(), |ctx| {
                        campfire_views::users::direct_room(ctx, &row)
                    }))
                })
                .map(Some)
                .map_err(Into::into)
        }
        Partial::RoomHeader {
            room_id,
            for_user_id,
        } => app
            .db
            .read_blocking(|conn| {
                let room = Room::find(conn, *room_id)?;
                let viewer = User::find(conn, *for_user_id)?;
                let header = rooms_directory::header(conn, &room, &viewer)?;
                let account = Account::first(conn)?;
                Ok(page::render_detached(app, account.as_ref(), |ctx| {
                    campfire_views::rooms::header_identity(ctx, &header).to_string()
                }))
            })
            .map(Some)
            .map_err(Into::into),
        _ => Ok(None),
    }
}
