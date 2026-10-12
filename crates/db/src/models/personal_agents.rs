//! Internal creation primitive for the later member-facing agent API.
use crate::{Agent, AgentKind, NewAgent, NewUser, Result, Tx, User};

pub fn create(tx: &mut Tx<'_>, mut user: NewUser, owner_id: i64) -> Result<Agent> {
    user.role = crate::Role::Bot;
    user.bot_token_digest = None;
    let bot = User::create_with_open_room_grant(tx, user, false)?;
    Agent::create(
        tx,
        NewAgent {
            user_id: bot.id,
            owner_id: Some(owner_id),
            kind: AgentKind::Personal,
            daily_message_cap: Some(200),
            ..Default::default()
        },
    )
}
