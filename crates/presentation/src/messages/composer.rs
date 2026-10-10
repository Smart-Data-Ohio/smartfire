// The message-owned composer. Feature owners supply the schedule control and Drive availability.
use super::{RoomKind, room_dom_id};

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
    pub fn thread_value(&self) -> String {
        self.thread
            .as_ref()
            .map(|thread| thread.id.to_string())
            .unwrap_or_default()
    }
    pub fn scoped_id(&self, prefix: &str, room_default: &str) -> String {
        self.thread.as_ref().map_or_else(
            || room_default.into(),
            |thread| format!("{prefix}_channel_thread_{}", thread.id),
        )
    }
    pub fn composer_frame_id(&self) -> String {
        self.scoped_id("composer_frame", "composer-frame")
    }
    pub fn attach_menu_id(&self) -> String {
        self.scoped_id("attach_menu", "attach-menu")
    }
    pub fn reply_notify_id(&self) -> String {
        let room_id = self.room_param_key.as_ref().map_or_else(
            || room_dom_id(self.room_kind, self.room_id, "reply_notify"),
            |key| format!("reply_notify_{key}_{}", self.room_id),
        );
        self.scoped_id("reply_notify", &room_id)
    }
    pub fn drive_available(&self) -> bool {
        self.drive != DriveFlow::None
    }
    pub fn drive_share(&self) -> bool {
        self.drive == DriveFlow::Share
    }
    pub fn drive_metadata(&self) -> bool {
        self.drive == DriveFlow::Metadata
    }
    pub fn message_path(&self) -> String {
        self.thread.as_ref().map_or_else(
            || format!("/rooms/{}/messages", self.room_id),
            |thread| format!("/rooms/{}/threads/{}/messages", self.room_id, thread.id),
        )
    }
    pub fn slash_list_path(&self) -> String {
        format!(
            "/autocompletable/slash_commands?room_id={}{}",
            self.room_id,
            self.thread
                .as_ref()
                .map(|thread| format!("&thread_id={}", thread.id))
                .unwrap_or_default()
        )
    }
}
