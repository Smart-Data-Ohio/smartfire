// The message-owned composer. Feature owners supply the schedule control and Drive availability.
use super::RoomKind;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub enum DriveFlow {
    #[default]
    None,
    Share,
    Metadata,
}
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct Thread {
    pub id: i64,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct Facts {
    pub room_id: i64,
    pub room_kind: RoomKind,
    /// `Room.model_name.param_key`; the legacy room kind collapses Voice/Stage/Board.
    #[serde(default)]
    pub room_param_key: Option<String>,
    pub room_name: String,
    pub thread: Option<Thread>,
    /// The complete registry and ordered room agent-command names, supplied by the presenter.
    pub slash_commands: Vec<String>,
    pub drive: DriveFlow,
}
impl Facts {
    pub fn message_path(&self) -> String {
        self.thread.as_ref().map_or_else(
            || format!("/rooms/{}/messages", self.room_id),
            |thread| format!("/rooms/{}/threads/{}/messages", self.room_id, thread.id),
        )
    }
}
