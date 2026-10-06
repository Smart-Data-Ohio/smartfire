//! `AgentsChannel` (reference/app/channels/agents_channel.rb): agent directory and profile
//! updates, for every signed-in human (not bots).
use campfire_cable::{Channel, ChannelResult, Params, Subscription};

use super::CableUser;

pub struct AgentsChannel;

/// `AgentsChannel::STREAM_NAME`
pub const STREAM_NAME: &str = "agents:all";

#[async_trait::async_trait]
impl Channel<CableUser> for AgentsChannel {
    /// `stream_from STREAM_NAME` unless the user is a bot, else `reject`.
    async fn subscribed(&mut self, sub: &mut Subscription<CableUser>) -> ChannelResult {
        if sub.current_user().bot() {
            sub.reject();
        } else {
            sub.stream_from(STREAM_NAME);
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
