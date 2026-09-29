//! `campfire_db::RichText` over `campfire_richtext`, for the models' Action Text needs
//! (`plain_text_body` for FTS, push and webhooks; `mentionees`).
//!
//! The models call this on the database writer thread inside their transaction, or on a reader
//! they hold. Record lookups use that same connection with the one resolver implementation the
//! controllers use (`controllers::presenters::DbResolver`): checking out another connection here
//! deadlocks once every pooled reader is waiting on the writer.

use std::sync::Arc;

use campfire_db::{BasicRichText, Connection, RichText};
use campfire_kit::SharedClock;
use campfire_richtext::RenderContext;
use rails_compat::Secrets;

use crate::controllers::presenters::DbResolver;

pub struct AppRichText {
    secrets: Arc<Secrets>,
    clock: SharedClock,
}

impl AppRichText {
    pub fn new(secrets: Arc<Secrets>, clock: SharedClock) -> Self {
        Self { secrets, clock }
    }

    fn with_context<T>(&self, conn: &Connection, f: impl FnOnce(&RenderContext) -> T) -> T {
        let resolver = DbResolver { conn, secrets: &self.secrets, now: self.clock.now() };
        f(&resolver.render_context(None))
    }
}

impl RichText for AppRichText {
    /// `message.body.to_plain_text`. Where Rails would raise, the save would fail; the models
    /// can't fail here, so it's logged and the tag-stripped text is used instead.
    fn to_plain_text(&self, conn: &Connection, html: &str, user_names: campfire_db::rich_text::UserNames<'_>) -> String {
        match self.with_context(conn, |ctx| campfire_richtext::to_plain_text(html, ctx)) {
            Ok(text) => text,
            Err(error) => {
                tracing::error!(%error, "to_plain_text raised");
                BasicRichText.to_plain_text(conn, html, user_names)
            }
        }
    }

    /// `body.attachables.grep(User).uniq`: verified SGIDs only.
    fn mentioned_user_ids(&self, conn: &Connection, html: &str) -> Vec<i64> {
        match self.with_context(conn, |ctx| campfire_richtext::mentioned_users(html, ctx)) {
            Ok(users) => users.into_iter().map(|user| user.id).collect(),
            Err(error) => {
                tracing::error!(%error, "mentioned_users raised");
                Vec::new()
            }
        }
    }
}
