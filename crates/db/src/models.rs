//! One module per model under `reference/app/models`.

pub mod account;
pub mod device_class;
pub use device_class::DeviceClass;
pub mod agent;
pub mod agent_budget_notice;
pub mod announcement;
pub mod ignore_policy;
pub mod slack;
pub mod slack_import;
pub mod user_block;
pub mod workspace_invite;
pub use agent_budget_notice::AgentBudgetNotice;
pub mod active_storage;
pub mod activity_item;
pub mod agent_access;
pub mod agent_api_pending;
pub mod agent_approval;
pub mod agent_approvals;
pub mod agent_context;
pub mod agent_credential;
pub mod agent_delivery;
pub mod agent_direct_messages;
pub mod agent_event_access;
pub mod agent_event_polling;
pub mod agent_grant;
pub mod agent_lifecycle;
pub mod agent_payloads;
pub mod agent_polls;
pub mod agent_posting;
pub mod agent_profile;
pub mod agent_reading;
pub mod agent_service;
pub mod agent_slash_command;
pub mod agent_step;
pub mod agent_streaming;
pub mod agent_work;
pub mod agent_work_events;
pub mod agent_working_presence;
pub mod audit_log;
pub mod auth_audit;
pub mod board_automations;
pub mod board_sla_nudge;
pub mod board_sla_rule;
pub mod board_stale_digest;
pub mod board_tag_assignment;
pub mod message_attachment_processing;
pub mod oauth;
pub mod personal_agents;
pub use board_sla_nudge::{BoardSlaNudge, NewBoardSlaNudge};
pub use board_sla_rule::{BoardSlaRule, NewBoardSlaRule};
pub use board_stale_digest::BoardStaleDigest;
pub use board_tag_assignment::{BoardTagAssignment, NewBoardTagAssignment};
pub mod work_handoff;
pub use work_handoff::{HandoffPackage, NewWorkHandoff, WorkHandoff};
pub mod ban;
pub mod boost;
pub mod bot_webhook_fanout;
pub mod channel_thread;
pub mod channel_webhook;
pub mod work_thread_event;
pub mod work_thread_link;
pub use work_thread_link::{NewWorkThreadLink, WorkThreadLink};
pub mod bot_avatar;
pub mod calendar_dispatch;
pub mod calendar_event;
pub mod call_moderation;
pub mod direct_room;
pub mod first_run;
pub mod forwarder;
pub mod google_identity;
pub mod huddle_cleanup;
pub mod huddle_effects;
pub mod huddle_grant;
pub mod huddle_invitations;
pub mod huddle_notices;
pub mod huddle_stream_liveness;
pub mod keyword_alert;
pub mod membership;
pub mod message;
pub mod message_pin;
pub mod message_reference;
pub mod notification_push;
pub mod poll;
pub mod push_subscription;
pub mod rich_text_record;
pub mod room;
pub mod room_category;
pub mod saved_item;
pub mod scheduled_message;
pub mod search;
pub mod search_query;
pub mod session;
pub mod sound;
pub mod stage;
pub mod stage_participation;
pub mod stage_streams;
pub mod stream;
pub mod thread_membership;
pub mod thread_tag;
pub mod two_factor;
pub mod user;
pub use bot_avatar::BotAvatar;
pub mod user_star;
pub use user_star::UserStar;
pub mod dnd_allowed_user;
pub mod user_status_settings;
pub use dnd_allowed_user::DndAllowedUser;
pub mod notification_policy;
pub mod user_device;
pub mod webhook;
pub mod workspace_presence_lease;

pub mod room_directory;
pub mod room_members;
pub mod workspace_icon;

pub use account::{Account, AccountSettings};
pub use active_storage::{Attachment, Blob};
pub use activity_item::ActivityItem;
pub use auth_audit::{AuthAudit, SudoVerifier};
pub use ban::Ban;
pub use boost::Boost;
pub use calendar_event::attendance::EventAttendance;
pub use calendar_event::{CalendarEvent, NewCalendarEvent};
pub use channel_thread::{
    AgentWorkChanges, ChannelThread, NewChannelThread, ThreadPush, ThreadPushCandidate,
    ThreadStatus,
};
pub use first_run::FirstRun;
pub use keyword_alert::KeywordAlert;
pub use membership::{Involvement, Membership, RoomRemovalBroadcast, StageRole};
pub use message::{ContentType, Message, MessageChanges, NewMessage, Timeline};
pub use message_pin::MessagePin;
pub use notification_policy::{NotificationKind, NotificationPolicy};
pub use poll::{NewPoll, Poll, PollOption, PollVote};
pub use push_subscription::{
    MAX_PAYLOAD_BODY_BYTES, MAX_PAYLOAD_TITLE_BYTES, PushPayload, PushSubscription,
};
pub use rich_text_record::RichTextRecord;
pub use room::{Room, RoomType};
pub use room_category::RoomCategory;
pub use saved_item::{NewSavedItem, SavedItem, SavedItemChanges};
pub use scheduled_message::{NewScheduledMessage, ScheduledMessage};
pub use search::Search;
pub use session::{NewSession, Session};
pub use sound::Sound;
pub use thread_membership::{ThreadInvolvement, ThreadMembership};
pub use thread_tag::{BoardTag, BoardTagPolicy, ThreadTag};
pub use two_factor::{
    ChallengeFailure, TwoFactorBackupCode, TwoFactorCredential, TwoFactorRememberedDevice,
    TwoFactorSetupSecret,
};
pub use user::{NewUser, PasswordDigest, Role, Status, User, UserChanges};
pub use user_device::{DeviceSignIn, UserDevice};
pub use user_status_settings::{MeetingCache, UserStatusSettings};
pub use webhook::Webhook;
pub use work_thread_event::WorkThreadEvent;

pub mod room_delete;

pub mod retention;
pub use workspace_presence_lease::WorkspacePresenceLease;

pub mod google_calendar;

pub mod google_account;

pub mod google_drive_link;

pub mod google_connection;

pub mod drive_recipients;

pub mod google_entry;
pub mod google_meeting_cache;
pub use agent::{Agent, AgentChanges, AgentKind, NewAgent};
pub use agent_approval::{AgentApproval, NewApproval};
pub use agent_credential::{AgentCredential, CredentialChanges, NewCredential};
pub use agent_grant::{AgentGrant, GrantChanges, NewGrant};
pub use agent_slash_command::{AgentSlashCommand, NewAgentSlashCommand};
pub use agent_step::{AgentStep, AgentStepChanges, NewAgentStep};
// WS8bm2 read-only rendering preload seam.
pub mod message_rendering;

// WS8bm2 listing/quote read adapters.
pub mod message_quote;
pub mod room_files;

pub mod drive_attachment;
pub mod reminder_policy;
pub use drive_attachment::DriveAttachment;

pub mod workspace_category;
pub use workspace_category::WorkspaceCategory;

pub mod soundboard;
pub mod sticker;
