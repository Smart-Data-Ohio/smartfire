//! `Users::SidebarsController` (reference/app/controllers/users/sidebars_controller.rb): the room
//! list, loaded into the `user_sidebar` turbo frame.

use askama::Template;
use campfire_db::Account;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_views::users;

use crate::app::AppCtx;
use crate::channels::user_gid;
use crate::concerns::{self, Before};
use crate::controllers::presenters::{self, view_context};

pub async fn show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    c.respond_to(&[&format::HTML])?;
    let user = concerns::require_current_user(c)?.clone();
    if c.app().config.huddle.configured() {
        let app=c.app().clone();let actor=user.clone();
        let (sidebar,_)=c.app().db.read(move |conn|composition::load(&app,conn,&actor)).await.map_err(Error::internal)?;
        return view_context::page_or_frame(c,StatusCode::OK,|ctx|users::sidebar_composition::Show{ctx,sidebar:&sidebar}.render(),|ctx|{
            let p=users::sidebar_composition::Show{ctx,sidebar:&sidebar};
            campfire_views::layouts::frame(ctx,p.as_head(),p.as_content())
        }).await;
    }
    let secrets = c.app().secrets.clone();
    let zone = view_context::time_zone(c).await?;
    let sidebar_zone = zone.clone();
    let (sidebar, restricted) = {
        let (user, secrets, fragments) = (user.clone(), secrets.clone(), c.app().fragment_cache.clone());
        c.app()
            .db
            .read(move |conn| {
                // The direct rooms' fragments come from the store the render then uses.
                let sidebar = campfire_views::fragment_cache::with(&fragments, || presenters::accounts::sidebar_in_zone(conn, &secrets, &user, &sidebar_zone))?;
                let restricted = Account::first(conn)?.is_some_and(|account| account.settings().restrict_room_creation_to_administrators());
                Ok((sidebar, restricted))
            })
            .await
            .map_err(Error::internal)?
    };

    let data = SidebarData {
        current_user: presenters::user_summary_in_zone(&secrets, &user, &zone),
        // turbo_stream_from :rooms / turbo_stream_from Current.user, :rooms
        rooms_stream: rails_compat::turbo::signed_stream_name(&secrets, &["rooms"]),
        user_rooms_stream: rails_compat::turbo::signed_stream_name(&secrets, &[&user_gid(user.id).to_param(), "rooms"]),
        sidebar,
        // `Current.user.administrator? || !Current.account.settings.restrict_room_creation_to_administrators?`
        can_create_rooms: user.is_administrator() || !restricted,
    };
    view_context::page_or_frame(
        c,
        StatusCode::OK,
        |ctx| data.page(ctx).render(),
        |ctx| {
            let page = data.page(ctx);
            campfire_views::layouts::frame(ctx, page.as_head(), page.as_content())
        },
    )
    .await
}

struct SidebarData {
    current_user: campfire_views::users::UserSummary,
    rooms_stream: String,
    user_rooms_stream: String,
    sidebar: presenters::accounts::Sidebar,
    can_create_rooms: bool,
}

impl SidebarData {
    fn page<'a>(&self, ctx: &'a campfire_views::ViewContext<'a>) -> users::SidebarShow<'a> {
        users::SidebarShow {
            ctx,
            current_user: self.current_user.clone(),
            rooms_stream: self.rooms_stream.clone(),
            user_rooms_stream: self.user_rooms_stream.clone(),
            favorite_memberships:self.sidebar.favorite_memberships.clone(),
            categories:self.sidebar.categories.clone(),
            voice_memberships:self.sidebar.voice_memberships.clone(),
            direct_memberships: self.sidebar.direct_memberships.clone(),
            direct_placeholder_users: self.sidebar.direct_placeholder_users.clone(),
            other_memberships: self.sidebar.other_memberships.clone(),
            can_create_rooms: self.can_create_rooms,
        }
    }
}

#[cfg(test)]
#[path = "sidebars_tests.rs"]
mod tests;

pub(crate) use crate::controllers::presenters::sidebar_composition as composition;
#[cfg(test)]
#[path="sidebars/tests.rs"]
mod composition_tests;
