//! JSON twins of `users#index`, `users#show` and `users/bans`, using the classic presenters and
//! ban transaction. All routes require a session; writes keep the classic administrator, record,
//! then sudo order, with the API error envelope.

use axum::Router;
use axum::routing::{get, post};
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::User;
use campfire_db::models::user::presentation;
use campfire_kit::{Ctx, Error, Kit, Result, StatusCode, action};
use campfire_people::controllers::users::{bans, find_user};
use campfire_web::controllers::presenters;

use crate::{admin, dto};

/// The people routes, merged into [`crate::routes`].
pub fn routes() -> Router<Kit> {
    Router::new()
        .route("/api/v1/people", get(action(index)))
        .route("/api/v1/people/{id}", get(action(show)))
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

async fn directory(c: &mut Ctx) -> Result {
    let viewer = admin::viewer(c).await?;
    let now = c.app().db.env().now();
    let secrets = c.app().secrets.clone();
    let directory = c
        .app()
        .db
        .read(move |conn| {
            let people = presentation::directory(conn, viewer.id, now)?
                .into_iter()
                .map(|person| {
                    let person = presenters::people::person(&secrets, person);
                    api::DirectoryPerson {
                        user_id: person.user.id,
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
    reply(c, &viewer, user).await
}

async fn reply(c: &mut Ctx, viewer: &User, user: User) -> Result {
    let now = c.app().db.env().now();
    let secrets = c.app().secrets.clone();
    let zone = presenters::view_context::time_zone(c).await?;
    let viewer_id = viewer.id;
    let administrator = viewer.can_administer(None, false);
    // `users/show.html:21,45-49,51,58-65`: all these sections are inside the person branch.
    let person = !user.is_bot() && !user.is_deactivated();
    let active = person && user.is_active();
    let other = user.id != viewer_id;
    let email_address = (administrator && person)
        .then(|| user.email_address.clone())
        .flatten();
    let transfer_url = (administrator && active).then(|| {
        // `users/profiles/_transfer.html:1`, exactly as G2's `AccountSettings.transfer_url`.
        let transfer = presenters::accounts::transfer_id(&c.app().secrets, user.id, c.now());
        c.url_for(&campfire_routes::session_transfer(&transfer))
    });
    let (user, status) = c
        .app()
        .db
        .read(move |conn| {
            let status = if person {
                Some(presenters::status_settings::profile_status_in_zone(
                    conn,
                    &secrets,
                    user.id,
                    viewer_id,
                    now,
                    zone.tz(),
                )?)
            } else {
                None
            };
            let user = dto::users(conn, &secrets, [user.id], now)?
                .pop()
                .ok_or(campfire_db::Error::RecordNotFound("User"))?;
            Ok((user, status))
        })
        .await
        .map_err(Error::internal)?;
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
    c.json(
        StatusCode::OK,
        &api::PersonProfile {
            user,
            status,
            dnd_allowed,
            email_address,
            transfer_url,
            can_ban: administrator && person && other,
        },
    )
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
    admin::require_sudo(c)?;
    let id = user.id;
    // Classic has no server-side self, bot or deactivated guard; keep the same write path.
    bans::set_banned(c, user, banned).await?;
    let user = c
        .app()
        .db
        .read(move |conn| User::find(conn, id))
        .await
        .map_err(Error::internal)?;
    reply(c, &viewer, user).await
}
