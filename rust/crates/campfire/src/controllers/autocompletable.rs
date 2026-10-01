//! Markdown composer autocomplete sources from our Rails app.
pub mod icons;
pub mod slash_commands;
pub mod users {
    use crate::app::AppCtx;
    use crate::concerns::{self, Before, cast_integer};
    use crate::controllers::presenters::{self, page::db_error, pagination::Page};
    use campfire_db::{Room, autocomplete_users};
    use campfire_kit::{Ctx, Error, Param, Result, StatusCode, format};
    pub async fn index(c: &mut Ctx) -> Result {
        concerns::before_actions(c, Before::default()).await?;
        let viewer = concerns::require_current_user(c)?.id;
        let room_id = if let Some(raw) = c.param("room_id").filter(|p| p.is_present()) {
            let id = raw
                .to_s()
                .and_then(|s| cast_integer(&s))
                .ok_or(Error::NotFound)?;
            Some(
                c.app()
                    .db
                    .read(move |conn| Room::find_for_user(conn, viewer, id))
                    .await
                    .map_err(db_error)?
                    .ok_or(Error::NotFound)?
                    .id,
            )
        } else {
            None
        };
        let query = c
            .param("query")
            .filter(|p| p.is_present())
            .and_then(Param::to_s);
        let counted = query.clone();
        let count = c
            .app()
            .db
            .read(move |conn| autocomplete_users::count(conn, room_id, counted.as_deref()))
            .await
            .map_err(db_error)?;
        let page = Page::new(c.param_str("page"), count, &[20]);
        let offset = page.offset();
        let limit = page.limit();
        let (records, unique) = c
            .app()
            .db
            .read(move |conn| {
                autocomplete_users::page(conn, room_id, query.as_deref(), offset, limit)
            })
            .await
            .map_err(db_error)?;
        let zone = crate::controllers::message_features::user_zone(c).await?;
        let users = records
            .iter()
            .map(|u| {
                let mut mention = presenters::accounts::mention_user(&c.app().secrets, u);
                // User timestamps are TimeWithZone in the request's SetTimeZone scope.
                mention.user.avatar_path = campfire_routes::fresh_user_avatar(
                    presenters::avatar_token(&c.app().secrets, u.id),
                    zone.to_fs(u.updated_at.jiff(), "number"),
                );
                mention
            })
            .collect::<Vec<_>>();
        c.respond_to(&[&format::JSON])?;
        page.apply_headers(c);
        let body = campfire_views::autocompletable::markdown_users_index_json(
            &users,
            &unique,
            &c.url_for(""),
        );
        Ok(c.render(StatusCode::OK, &format::JSON, body))
    }
}
