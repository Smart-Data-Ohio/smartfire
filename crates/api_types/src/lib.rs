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
mod admin;
mod agents;
mod attachment;
mod bots;
mod cards;
mod composer;
mod conversation;
mod direct;
mod error;
mod huddle;
mod me;
mod message;
mod organize;
mod panes;
mod people;
mod presence;
mod reaction;
mod read;
mod room;
mod saved;
mod search;
mod settings;
mod sidebar;
mod slack;
mod stage;
mod switcher;
mod sync;
mod thread;
mod user;
mod work;

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
pub use admin::{
    AuditLogEntry, AuditLogFilters, AuditLogPage, CreateIcon, CustomStyles, DeliveryHealth,
    EmailHealth, FizzyHealth, GithubHealth, GoogleHealth, HealthIssue,
    IntegrationsHealth, PeoplePage, Person, PersonChange, PersonRemoved, PersonRole,
    PushChannelExpiry, UpdateLogo, UpdatePerson, UpdateWorkspace, Workspace, WorkspaceIcon,
    WorkspaceIconList,
};
pub use agents::{
    AgentActivitySummary, AgentApproval, AgentApprovalPage, AgentBadge, AgentBudgetUsage,
    AgentCapability, AgentDeliveryOutcome, AgentDirectory, AgentDirectoryRow, AgentExternalResult,
    AgentGrant, AgentGrants, AgentKind, AgentLedgerEvent, AgentLedgerEventType, AgentLedgerPage,
    AgentManagement, AgentProfile, AgentProfileRoom, AgentStatus, AgentStatusChanged, AgentStep,
    AgentStepStatus, AgentStepsChanged, AgentWebhookStatus, ApprovalDecision, ApprovalUpdated,
    DecideApproval,
};
pub use attachment::{Attachment, AttachmentPreview, CreateUpload, DirectUpload};
pub use bots::{
    Bot, BotAgent, BotChange, BotGithub, BotIcon, BotKey, BotList, BotRemoved, BotRoom,
    BotSummary, ConnectGithub, CreateBot, CreateCredential, CreateGrant, Credential,
    CredentialCreated, CredentialList, CredentialState, Grant, GrantList, GrantRoom, UpdateBot,
    UpdateBotAgent,
};
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
pub use huddle::{
    HuddleCredentials, HuddleDetail, HuddleModeration, HuddleNotice, HuddleParticipant,
    HuddlePresence, HuddlePresenceList, HuddleRing, HuddleRingEvent, HuddleRingState,
    HuddleRoleChanged, ModerateHuddle,
};
pub use me::{
    DoNotDisturb, Me, OutOfOffice, Preferences, PresenceSetting, QuietHours, TextSize, Theme,
    VoiceMode,
};
pub use message::{
    CreateMessage, MessageDTO, MessagePage, MessageRead, MessageRemoved, MessageSource,
    UpdateMessage,
};
pub use organize::{
    AssignRoomCategory, CreateRoomCategory, FavoriteList, MoveFavorite, ReorderRoomCategories,
    RoomCategoryList, RoomCategoryRemoved, UpdateInvolvement, UpdateRoomCategory,
};
pub use panes::{FileList, FileType, Member, MemberList, RoomFile, StarState};
pub use people::{DirectoryPerson, PeopleDirectory, PersonProfile, PersonStatus};
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
    AccountSettings, AppearanceSettings, BackupCodes, CallSettings, Connection, DndAllowedPerson,
    GoogleIntegration, InboxSwitch,
    IntegrationChange, IntegrationSettings, IntegrationToken, NotificationSettings, OooPreset,
    ProfileSettings, PushSubscriptionInfo, PushSubscriptionList, Reauthentication, RememberedDevice,
    RoomMembershipRow, SessionInfo, SessionList, Settings, StatusExpiry, StatusSettings, TimeZoneChoice,
    TwoFactorChange, TwoFactorSettings, UpdateAppearance, UpdateAvatar, UpdateCalls,
    UpdateNotifications, UpdateProfile, UpdateStatus,
};
pub use slack::{
    SaveSlackCredentials, SlackConnectionState, SlackConversation, SlackCounts, SlackDisconnected,
    SlackIssue, SlackPeople, SlackPersonal, SlackPlan, SlackPlanConversation, SlackPreset,
    SlackRoomTarget, SlackRun, SlackRunChange, SlackRunKind, SlackRunList, SlackRunMode,
    SlackRunPage, SlackRunRow, SlackRunStatus, SlackRunSummary, SlackSample, SlackSetup,
    SlackSetupChange, StartPersonalSlackImport, StartSlackDryRun, StartSlackImport,
};
pub use sidebar::{RoomCategory, Sidebar, SidebarRow, SidebarRowRemoved};
pub use stage::{
    ChangeStageRole, LowerHand, StageDetail, StageMember, StageState, StageStream,
    StageStreamStopped, StartStageStream, StopStageStream, StreamQuality,
};
pub use switcher::{Switcher, SwitcherPerson, SwitcherRoom, SwitcherRoomKind, SwitcherThread};
pub use sync::{ClientFrame, ResumePoint, ServerFrame, SyncEvent, SyncPayload, Typing};
pub use thread::{
    CreateThread, JoinThread, Thread, ThreadCreated, ThreadDetail, ThreadFilter, ThreadIndicator,
    ThreadIndicatorChanged, ThreadInvolvement, ThreadList, ThreadMembership, ThreadMembershipState,
    ThreadPermissions, ThreadRead, ThreadRemoved, ThreadStatus, ThreadSummary, ThreadUnread,
    UpdateThread,
};
pub use user::{CustomStatus, User, UserList, UserRole, UserStatus};
pub use work::{
    CreateWorkHandoff, UpdateWork, WorkDetail, WorkFacts, WorkFilter, WorkHandoffReceiver,
    WorkHistoryEntry, WorkHistoryHandoff, WorkHistoryKind, WorkLink, WorkLinkKind, WorkList,
    WorkListRow, WorkOwnerCandidate, WorkOwnerSnapshot, WorkPullRequestState,
};

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
mod tests_s4;
#[cfg(test)]
mod tests_s4b;
#[cfg(test)]
mod tests_s5;
#[cfg(test)]
mod tests_s7;
#[cfg(test)]
mod tests_s7_admin;
#[cfg(test)]
mod tests_s7_bots;
#[cfg(test)]
mod tests_s7_slack;

#[cfg(test)]
mod tests_s7_people;
