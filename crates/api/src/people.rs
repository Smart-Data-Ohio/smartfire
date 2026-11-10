//! People directory, profiles, private ignore records and classic bans as JSON.
//! All routes require a session. Bans retain the administrator, record, then sudo order,
//! with the API error envelope.

use axum::Router;
use axum::routing::{get, post, put};
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::User;
use campfire_db::models::user::presentation;
use campfire_db::models::{ignore_policy, user_block::UserBlock};
use campfire_kit::{Ctx, Error, Kit, Result, StatusCode, action};
use campfire_people::controllers::qr_code;
use campfire_people::controllers::users::{bans, find_user};
use campfire_runtime::presenters;

use crate::{admin, dto};

/// The people routes, merged into [`crate::routes`].
pub fn routes() -> Router<Kit> {
    Router::new()
        .route("/api/v1/people", get(action(index)))
        .route("/api/v1/people/{id}", get(action(show)))
        .route("/api/v1/blocks", get(action(blocks)))
        .route(
            "/api/v1/people/{id}/block",
            put(action(block)).delete(action(unblock)),
        )
        .route(
            "/api/v1/people/{id}/ban",
            post(action(ban)).delete(action(unban)),
        )
}

endpoint!(
    /// `GET /api/v1/people`
    index => directory
);
endpoint!(
    /// `GET /api/v1/people/{id}`
    show => profile
);
endpoint!(
    /// `POST /api/v1/people/{id}/ban`
    ban => create_ban
);
endpoint!(
    /// `DELETE /api/v1/people/{id}/ban`
    unban => destroy_ban
);
endpoint!(blocks => list_blocks);
endpoint!(block => create_block);
endpoint!(unblock => destroy_block);

async fn list_blocks(c: &mut Ctx) -> Result {
    let viewer = admin::viewer(c).await?.id;
    let now = c.app().db.env().now();
    let secrets = c.app().secrets.clone();
    let list = c
        .app()
        .db
        .read_snapshot(move |conn| {
            let records = UserBlock::for_viewer(conn, viewer)?;
            let users = dto::users(
                conn,
                &secrets,
                records.iter().map(|record| record.blocked_id),
                now,
            )?;
            let revision = conn.query_row(
                "SELECT activity_revision FROM users WHERE id=?",
                [viewer],
                |row| row.get(0),
            )?;
            Ok(api::BlockList {
                viewer_id: viewer,
                revision,
                people: records
                    .into_iter()
                    .filter_map(|record| {
                        users
                            .iter()
                            .find(|user| user.id == record.blocked_id)
                            .cloned()
                            .map(|user| api::BlockedPerson {
                                user,
                                ignored_by_me: true,
                                created_at: dto::time(record.created_at),
                            })
                    })
                    .collect(),
            })
        })
        .await
        .map_err(campfire_runtime::context::db_error)?;
    c.json(StatusCode::OK, &list)
}

async fn create_block(c: &mut Ctx) -> Result {
    change_block(c, true).await
}
async fn destroy_block(c: &mut Ctx) -> Result {
    change_block(c, false).await
}

async fn change_block(c: &mut Ctx, ignored: bool) -> Result {
    let viewer = admin::viewer(c).await?.id;
    let person = find_user(c, "id").await?.id;
    let state = c
        .app()
        .db
        .write(move |tx| {
            let revision = UserBlock::set(tx, viewer, person, ignored)?;
            Ok(api::BlockState {
                viewer_id: viewer,
                user_id: person,
                ignored_by_me: ignored,
                revision,
            })
        })
        .await
        .map_err(campfire_runtime::context::db_error)?;
    campfire_app::cable::sync::blocks_updated(&c.app().cable, viewer, state.clone());
    c.json(StatusCode::OK, &state)
}

async fn directory(c: &mut Ctx) -> Result {
    let viewer = admin::viewer(c).await?;
    let now = c.app().db.env().now();
    let secrets = c.app().secrets.clone();
    let directory = c
        .app()
        .db
        .read(move |conn| {
            let ignored = ignore_policy::ignored_authors(conn, viewer.id)?;
            let people = presentation::directory(conn, viewer.id, now)?
                .into_iter()
                .map(|person| {
                    let person = presenters::people::person(&secrets, person);
                    api::DirectoryPerson {
                        user_id: person.user.id,
                        ignored_by_me: ignored.contains(&person.user.id),
                        online: person.online,
                        starred: person.starred,
                        agent: person.agent,
                    }
                })
                .collect::<Vec<_>>();
            let users = dto::users(conn, &secrets, people.iter().map(|p| p.user_id), now)?;
            Ok(api::PeopleDirectory { people, users })
        })
        .await
        .map_err(Error::internal)?;
    c.json(StatusCode::OK, &directory)
}

async fn profile(c: &mut Ctx) -> Result {
    let viewer = admin::viewer(c).await?;
    let user = find_user(c, "id").await?;
    reply(c, &viewer, user.id).await
}

async fn reply(c: &mut Ctx, viewer: &User, id: i64) -> Result {
    let now = c.app().db.env().now();
    let secrets = c.app().secrets.clone();
    let zone = campfire_runtime::presenters::current::time_zone(c).await?;
    let viewer_id = viewer.id;
    let administrator = viewer.can_administer(None, false);
    #[cfg(feature = "test-support")]
    crate::test_hooks::before_profile_read(id).await;
    let (mut profile, active) = c
        .app()
        .db
        .read(move |conn| {
            // `db.read` only leases a connection. One deferred snapshot keeps the user,
            // status and DTO from mixing fields when a ban or unban commits between queries.
            let snapshot = conn.unchecked_transaction()?;
            let conn = &snapshot;
            let user = User::find(conn, id)?;
            // `users/show.html:21,45-49,51,58-65`: all these sections are inside the person branch.
            let person = !user.is_bot() && !user.is_deactivated();
            let active = person && user.is_active();
            let other = user.id != viewer_id;
            let can_manage_bot = user.is_bot()
                && user.is_active()
                && (administrator
                    || campfire_db::Agent::for_user(conn, user.id)?
                        .is_some_and(|agent| agent.owner_id == Some(viewer_id)));
            let email_address = (administrator && person)
                .then(|| user.email_address.clone())
                .flatten();
            let status = if person {
                Some(presenters::status_settings::profile_status_in_zone(
                    conn,
                    user.id,
                    viewer_id,
                    now,
                    zone.tz(),
                )?)
            } else {
                None
            };
            let dnd_allowed = status
                .as_ref()
                .filter(|_| active && other)
                .map(|s| s.dnd_allowed);
            let status = status.map(|status| api::PersonStatus {
                presence: match status.presence.as_str() {
                    "online" => api::Presence::Online,
                    "idle" => api::Presence::Idle,
                    "dnd" => api::Presence::Dnd,
                    _ => api::Presence::Offline,
                },
                status_text: status.status_text,
            });
            let user = dto::users(conn, &secrets, [user.id], now)?
                .pop()
                .ok_or(campfire_db::Error::RecordNotFound("User"))?;
            Ok((
                api::PersonProfile {
                    ignored_by_me: ignore_policy::ignored(conn, viewer_id, user.id)?,
                    user,
                    status,
                    dnd_allowed,
                    email_address,
                    transfer_url: None,
                    transfer_qr_svg: None,
                    can_ban: administrator && person && other,
                    can_manage_bot,
                },
                active,
            ))
        })
        .await
        .map_err(Error::internal)?;
    profile.transfer_url = (administrator && active).then(|| {
        // `users/profiles/_transfer.html:1`, exactly as G2's `AccountSettings.transfer_url`.
        let transfer =
            presenters::accounts::transfer_id(&c.app().secrets, profile.user.id, c.now());
        c.url_for(&campfire_routes::session_transfer(&transfer))
    });
    profile.transfer_qr_svg = profile
        .transfer_url
        .as_deref()
        .map(|url| {
            qr_code::transfer_svg(url).ok_or_else(|| {
                Error::internal(std::io::Error::other(
                    "the sign-in link doesn't fit a QR code",
                ))
            })
        })
        .transpose()?;
    c.json(StatusCode::OK, &profile)
}

async fn create_ban(c: &mut Ctx) -> Result {
    change_ban(c, true).await
}

async fn destroy_ban(c: &mut Ctx) -> Result {
    change_ban(c, false).await
}

async fn change_ban(c: &mut Ctx, banned: bool) -> Result {
    let viewer = admin::administrator(c).await?;
    let user = find_user(c, "id").await?;
    admin::require_sudo(c).await?;
    let id = user.id;
    // Classic has no server-side self, bot or deactivated guard; keep the same write path.
    bans::set_banned(c, user, banned).await?;
    reply(c, &viewer, id).await
}
