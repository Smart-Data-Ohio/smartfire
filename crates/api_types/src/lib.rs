//! The JSON the single-page app (`frontend/`) reads from `/api/v1` and the `/api/v1/sync` socket.
//!
//! Plain serde structs and enums with no behaviour and no dependency on the rest of the workspace:
//! the handlers that build them from the domain models live above this crate. Each type derives
//! ts-rs's `TS` and is exported to `frontend/src/gen/<Type>.ts` by
//! `cargo test -p campfire_api_types export_bindings` (`pnpm gen` in `frontend/`). CI regenerates
//! the files and fails if the committed copies differ. The front end's hand-written Effect Schemas
//! (`frontend/src/api/schema/`) are pinned to the generated types at compile time.
//!
//! Wire conventions:
//! - keys are camelCase;
//! - ids are JSON numbers (SQLite rowids, below 2^53; `TS_RS_LARGE_INT=number` in
//!   `.cargo/config.toml` maps `i64` to `number`);
//! - timestamps are [`Timestamp`] strings;
//! - an absent value is `null`, never an omitted key;
//! - enums are lowercase string literals.

mod error;
mod me;
mod message;
mod presence;
mod read;
mod room;
mod sidebar;
mod sync;
mod user;

pub use error::{ApiError, ApiErrorResponse};
pub use me::{
    DoNotDisturb, Me, OutOfOffice, Preferences, PresenceSetting, QuietHours, TextSize, Theme,
    VoiceMode,
};
pub use message::{CreateMessage, MessageDTO, MessagePage, MessageRemoved};
pub use presence::{Presence, PresenceList, UserPresence};
pub use read::{MarkUnread, ReadState, RoomRead, RoomUnread};
pub use room::{Involvement, Membership, Room, RoomDetail, RoomKind, StageRole, UnreadDivider};
pub use sidebar::{RoomCategory, Sidebar, SidebarRow, SidebarRowRemoved};
pub use sync::{ClientFrame, ResumePoint, ServerFrame, SyncEvent, SyncPayload, Typing};
pub use user::{CustomStatus, User, UserList, UserRole, UserStatus};

/// A UTC instant as Rails' JSON encodes it: RFC 3339 with millisecond precision and a `Z`
/// suffix, e.g. `"2026-09-26T12:26:46.848Z"` (what `json_time` produces across the app).
pub type Timestamp = String;

#[cfg(test)]
mod tests;
