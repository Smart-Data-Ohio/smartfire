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

mod actions;
mod activity;
mod attachment;
mod cards;
mod composer;
mod conversation;
mod direct;
mod error;
mod me;
mod message;
mod organize;
mod panes;
mod presence;
mod reaction;
mod read;
mod room;
mod saved;
mod search;
mod settings;
mod sidebar;
mod switcher;
mod sync;
mod thread;
mod user;

pub use actions::{
    CreateForwards, ForwardDestination, ForwardDestinationList, ForwardResult, ForwardTarget,
    ForwardThread, Pin, PinList, PinState, SaveMessage, SavedChanged, SavedItem, SavedMark,
    SavedStatus,
};
pub use activity::{
    ActivityAction, ActivityEventType, ActivityItem, ActivityItemChanged, ActivityItemRemoved,
    ActivityList, ActivitySource, ActivitySourceType, ActivityState, ActivityTab,
    ActivityUnreadCount, AgentApprovalStatus, AgentBudgetCap, UpdateActivityItem,
};
pub use attachment::{Attachment, AttachmentPreview, CreateUpload, DirectUpload};
pub use cards::{
    AttendanceResponse, CardFetch, CreatePoll, DriveFileCard, EventAttendance, EventCard,
    FizzyAssignee, FizzyCard, FizzyCardPreview, FizzyCardRef, FizzyCardStatus, GithubCardRef,
    GithubChangedFile, GithubChangedFiles, GithubChecks, GithubPullRequest, GithubPullRequestCard,
    GithubPullRequestStatus, GithubReview, LinkCard, LinkedinCard, MessageCard, MessageCards, Poll,
    PollBallot, PollOption, PollResults, PollUpdated, QuoteCard, QuotePreview, QuotePreviewResult,
    RespondToEvent, VotePoll, XMedia, XMediaKind, XPostCard, XQuote,
};
pub use composer::{
    CreateScheduledMessage, Icon, IconKind, IconList, MessagePreview, PreviewMessage,
    RunSlashCommand, ScheduledMessage, ScheduledMessageFilter, ScheduledMessageList,
    ScheduledMessageRemoved, ScheduledMessageState, SlashCommand, SlashCommandList,
    SlashCommandResult, UpdateScheduledMessage, UserSuggestion, UserSuggestionList,
};
pub use conversation::ConversationName;
pub use direct::{
    AddDirectMembers, CreateDirect, DirectCandidate, DirectCandidateList, RenameDirect,
};
pub use error::{ApiError, ApiErrorResponse};
pub use me::{
    DoNotDisturb, Me, OutOfOffice, Preferences, PresenceSetting, QuietHours, TextSize, Theme,
    VoiceMode,
};
pub use message::{
    CreateMessage, MessageDTO, MessagePage, MessageRemoved, MessageSource, UpdateMessage,
};
pub use organize::{
    AssignRoomCategory, CreateRoomCategory, FavoriteList, MoveFavorite, ReorderRoomCategories,
    RoomCategoryList, RoomCategoryRemoved, UpdateInvolvement, UpdateRoomCategory,
};
pub use panes::{FileList, FileType, Member, MemberList, RoomFile, StarState};
pub use presence::{Presence, PresenceList, UserPresence};
pub use reaction::{Boost, CreateBoost, MessageReactions, Reaction};
pub use read::{MarkUnread, ReadState, RoomRead, RoomUnread};
pub use room::{Involvement, Membership, Room, RoomDetail, RoomKind, StageRole, UnreadDivider};
pub use saved::{SavedFilter, SavedItemList, UpdateSavedItem};
pub use search::{
    RecentSearch, RecentSearchList, RecordSearch, SearchChip, SearchOperator, SearchResults,
    SearchSection, SearchSectionKind, SearchSectionRow, WorkStatus,
};
pub use settings::{
    AppearanceSettings, CallSettings, Connection, DndAllowedPerson, GoogleIntegration, InboxSwitch,
    IntegrationSettings, NotificationSettings, OooPreset, ProfileSettings, PushSubscriptionInfo,
    PushSubscriptionList, SessionInfo, SessionList, Settings, StatusExpiry, StatusSettings,
    TimeZoneChoice, UpdateAppearance, UpdateAvatar, UpdateCalls, UpdateNotifications, UpdateProfile,
    UpdateStatus,
};
pub use sidebar::{RoomCategory, Sidebar, SidebarRow, SidebarRowRemoved};
pub use switcher::{Switcher, SwitcherPerson, SwitcherRoom, SwitcherRoomKind, SwitcherThread};
pub use sync::{ClientFrame, ResumePoint, ServerFrame, SyncEvent, SyncPayload, Typing};
pub use thread::{
    CreateThread, JoinThread, Thread, ThreadCreated, ThreadDetail, ThreadFilter, ThreadIndicator,
    ThreadIndicatorChanged, ThreadInvolvement, ThreadList, ThreadMembership, ThreadMembershipState,
    ThreadPermissions, ThreadRead, ThreadRemoved, ThreadStatus, ThreadSummary, ThreadUnread,
    UpdateThread,
};
pub use user::{CustomStatus, User, UserList, UserRole, UserStatus};

/// A UTC instant as Rails' JSON encodes it: RFC 3339 with millisecond precision and a `Z`
/// suffix, e.g. `"2026-09-26T12:26:46.848Z"` (what `json_time` produces across the app).
pub type Timestamp = String;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_s2;
#[cfg(test)]
mod tests_s3;
#[cfg(test)]
mod tests_s7;
