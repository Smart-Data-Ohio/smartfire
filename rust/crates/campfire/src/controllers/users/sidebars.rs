//! `Users::SidebarsController`: the complete per-viewer workspace sidebar.
mod composition;
#[cfg(test)]
mod tests;
use crate::app::AppCtx;
use crate::concerns::{self, Before};
use crate::controllers::presenters::{self, view_context};
use askama::Template;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_views::users;
pub async fn show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    c.respond_to(&[&format::HTML])?;
    let user = concerns::require_current_user(c)?.clone();
    let app = c.app().clone();
    let actor = user.clone();
    let (sidebar, legacy) = c
        .app()
        .db
        .read(move |conn| composition::load(&app, conn, &actor))
        .await
        .map_err(Error::internal)?;
    let summary = presenters::user_summary(&c.app().secrets, &user);
    view_context::page_or_frame(
        c,
        StatusCode::OK,
        |ctx| page(ctx, &sidebar, &summary, &legacy).render(),
        |ctx| {
            let p = page(ctx, &sidebar, &summary, &legacy);
            campfire_views::layouts::frame(ctx, p.as_head(), p.as_content())
        },
    )
    .await
}
fn page<'a>(
    ctx: &'a campfire_views::ViewContext<'a>,
    sidebar: &'a users::sidebar::Sidebar,
    current_user: &users::UserSummary,
    legacy: &presenters::accounts::Sidebar,
) -> users::SidebarShow<'a> {
    users::SidebarShow {
        ctx,
        composition: Some(sidebar),
        current_user: current_user.clone(),
        rooms_stream: String::new(),
        user_rooms_stream: String::new(),
        direct_memberships: legacy.direct_memberships.clone(),
        direct_placeholder_users: legacy.direct_placeholder_users.clone(),
        other_memberships: legacy.other_memberships.clone(),
        call_memberships: Vec::new(),
        can_create_rooms: sidebar.can_create,
    }
}
