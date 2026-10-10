//! HTML adapter only; shared X cards use persisted, session-independent facts.
use crate::{
    app::App,
};
#[cfg(any(test, feature = "test-support"))]
use campfire_db::Message;
#[cfg(any(test, feature = "test-support"))]
pub fn container(
    app: &App,
    conn: &campfire_db::Connection,
    message: &Message,
) -> campfire_db::Result<String> {
    let view = super::Presenter::new(conn, app, None).message(message)?;
    Ok(super::page::render_detached(app, None, |ctx| {
        campfire_views::twitter::cards(ctx, &view).0
    }))
}
pub fn broadcast_updates(app: &App, post_id: i64) -> anyhow::Result<()> {
    let app = app.clone();
    app.db.clone().read_blocking(move |conn| {
        use crate::integrations::message_batches::{self, Reference};
        let mut after = None;
        loop {
            let messages = message_batches::next(conn, Reference::TwitterPost(post_id), after)?;
            app.broadcasts.sync_message_cards(conn, &messages);
            if messages.len() < message_batches::SIZE { break; }
            after = messages.last().map(|message| message.id);
        }
        Ok(())
    })?;
    Ok(())
}

pub use campfire_runtime::presenters::twitter_cards::*;

#[cfg(any(test, feature = "test-support"))]
use crate::controllers::presenters::Rendering;
