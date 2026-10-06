//! `Users::PushSubscriptions::TestNotificationsController`
//! (reference/app/controllers/users/push_subscriptions/test_notifications_controller.rb).

use campfire_db::{Event, PushSubscription, models::push_subscription::TestNotificationJob};
use campfire_kit::{Ctx, Error, Result};

use crate::app::AppCtx;
use crate::concerns::{self, Before, cast_integer};

/// Preserve the Rails payload/scope, with delivery persisted on our durable queue.
pub async fn create(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let user_id = concerns::require_current_user(c)?.id;
    // `Current.user.push_subscriptions.find(params[:push_subscription_id])`
    let id = c
        .param_str("push_subscription_id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    let subscription = c
        .app()
        .db
        .read(move |conn| match PushSubscription::find(conn, id) {
            Ok(subscription) if subscription.user_id == user_id => Ok(Some(subscription)),
            Ok(_) | Err(campfire_db::Error::RecordNotFound(_)) => Ok(None),
            Err(error) => Err(error),
        })
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)?;

    let location = c.url_for(&campfire_routes::user_push_subscriptions());
    let job = TestNotificationJob {
        subscription_id: subscription.id,
        user_id,
        body: uuid::Uuid::new_v4().to_string(),
        path: location.clone(),
    };
    c.app()
        .db
        .write(move |tx| {
            tx.emit_after_commit(Event::job(&job));
            Ok(())
        })
        .await
        .map_err(Error::internal)?;
    c.redirect_to(&location)
}
