//! `ReadRoomsChannel` (reference/app/channels/read_rooms_channel.rb).
use campfire_cable::{Channel, ChannelResult, Params, Subscription};

use super::CableUser;

pub struct ReadRoomsChannel;

/// The user's own stream of rooms read in another window.
pub use crate::cable::read_rooms_stream_name as stream_name_for;

#[async_trait::async_trait]
impl Channel<CableUser> for ReadRoomsChannel {
    async fn subscribed(&mut self, sub: &mut Subscription<CableUser>) -> ChannelResult {
        let stream = stream_name_for(sub.current_user().id);
        sub.stream_from(stream);
        Ok(())
    }

    /// `subscribed` is public, so it's an action too.
    async fn perform(&mut self, action: &str, _data: &Params, sub: &mut Subscription<CableUser>) -> ChannelResult<bool> {
        match action {
            "subscribed" => self.subscribed(sub).await.map(|()| true),
            _ => Ok(false),
        }
    }
}
