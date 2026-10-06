//! `HuddleNoticeChannel` (reference/app/channels/huddle_notice_channel.rb): the user's own huddle
//! notices, for active humans only.
use campfire_cable::{Channel, ChannelResult, Params, Subscription};

use super::CableUser;

pub struct HuddleNoticeChannel;

/// `HuddleNoticeChannel.stream_name_for(user_id)`
pub fn stream_name_for(user_id: i64) -> String {
    format!("user_{user_id}_huddle_notices")
}

#[async_trait::async_trait]
impl Channel<CableUser> for HuddleNoticeChannel {
    /// `stream_from` the user's stream if `ActivityItem.active_human?(current_user)`, else `reject`.
    async fn subscribed(&mut self, sub: &mut Subscription<CableUser>) -> ChannelResult {
        if sub.current_user().active_human() {
            let stream = stream_name_for(sub.current_user().id);
            sub.stream_from(stream);
        } else {
            sub.reject();
        }
        Ok(())
    }

    /// `subscribed` is public, so it's an action too.
    async fn perform(&mut self, action: &str, _data: &Params, sub: &mut Subscription<CableUser>) -> ChannelResult<bool> {
        match action {
            "subscribed" if !sub.rejected() => self.subscribed(sub).await.map(|()| true),
            _ => Ok(false),
        }
    }
}
