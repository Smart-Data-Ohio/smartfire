//! WS8bm's shared renderer boundary for after-commit message descriptions.
//!
//! This is the complete-message arm of WS8bm's `domain_partial` seam. WS8bm owns
//! the remaining message partials; agent-step callbacks use this same renderer.

use campfire_db::Message;
use campfire_views::messages as views;

use crate::app::App;
use crate::controllers::presenters::{Presenter, page};

pub fn domain_partial(
    app: &App,
    partial: &campfire_db::broadcasts::Partial,
) -> campfire_db::Result<Option<String>> {
    use campfire_db::broadcasts::Partial;
    let id = match partial {
        Partial::Message { message_id } | Partial::MessageReplace { message_id } => *message_id,
        _ => return Ok(None),
    };
    app.db.read_blocking(|conn| {
        let Some(message) = Message::find_by_id(conn, id)? else {
            return Ok(None);
        };
        let view = Presenter::new(conn, app, None).message(&message)?;
        let account = campfire_db::Account::first(conn)?;
        Ok(Some(page::render_detached(app, account.as_ref(), |ctx| {
            views::uncached_message(ctx, &view)
        })))
    })
}
