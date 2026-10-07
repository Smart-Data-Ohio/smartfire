import { Schema } from "effect";

/** Ids are SQLite rowids, integers below 2^53 on the wire; each kind is branded apart. */
export const UserId = Schema.Int.pipe(Schema.brand("UserId"));

export type UserId = typeof UserId.Type;

export const RoomId = Schema.Int.pipe(Schema.brand("RoomId"));

export type RoomId = typeof RoomId.Type;

export const MembershipId = Schema.Int.pipe(Schema.brand("MembershipId"));

export type MembershipId = typeof MembershipId.Type;

export const MessageId = Schema.Int.pipe(Schema.brand("MessageId"));

export type MessageId = typeof MessageId.Type;

export const ThreadId = Schema.Int.pipe(Schema.brand("ThreadId"));

export type ThreadId = typeof ThreadId.Type;

export const RoomCategoryId = Schema.Int.pipe(Schema.brand("RoomCategoryId"));

export type RoomCategoryId = typeof RoomCategoryId.Type;

export const BoostId = Schema.Int.pipe(Schema.brand("BoostId"));

export type BoostId = typeof BoostId.Type;

export const SavedItemId = Schema.Int.pipe(Schema.brand("SavedItemId"));

export type SavedItemId = typeof SavedItemId.Type;

export const ScheduledMessageId = Schema.Int.pipe(Schema.brand("ScheduledMessageId"));

export type ScheduledMessageId = typeof ScheduledMessageId.Type;

export const ActivityItemId = Schema.Int.pipe(Schema.brand("ActivityItemId"));

export type ActivityItemId = typeof ActivityItemId.Type;

export const RecentSearchId = Schema.Int.pipe(Schema.brand("RecentSearchId"));

export type RecentSearchId = typeof RecentSearchId.Type;

export const PollId = Schema.Int.pipe(Schema.brand("PollId"));

export type PollId = typeof PollId.Type;

export const PollOptionId = Schema.Int.pipe(Schema.brand("PollOptionId"));

export type PollOptionId = typeof PollOptionId.Type;

export const EventId = Schema.Int.pipe(Schema.brand("EventId"));

export type EventId = typeof EventId.Type;

export const GithubPullRequestId = Schema.Int.pipe(Schema.brand("GithubPullRequestId"));

export type GithubPullRequestId = typeof GithubPullRequestId.Type;

export const FizzyCardId = Schema.Int.pipe(Schema.brand("FizzyCardId"));

export type FizzyCardId = typeof FizzyCardId.Type;

export const AgentId = Schema.Int.pipe(Schema.brand("AgentId"));

export type AgentId = typeof AgentId.Type;

export const AgentStepId = Schema.Int.pipe(Schema.brand("AgentStepId"));

export type AgentStepId = typeof AgentStepId.Type;

export const AgentApprovalId = Schema.Int.pipe(Schema.brand("AgentApprovalId"));

export type AgentApprovalId = typeof AgentApprovalId.Type;

export const AgentEventId = Schema.Int.pipe(Schema.brand("AgentEventId"));

export type AgentEventId = typeof AgentEventId.Type;

export const WorkLinkId = Schema.Int.pipe(Schema.brand("WorkLinkId"));

export type WorkLinkId = typeof WorkLinkId.Type;

export const WorkEventId = Schema.Int.pipe(Schema.brand("WorkEventId"));

export type WorkEventId = typeof WorkEventId.Type;

export const SessionId = Schema.Int.pipe(Schema.brand("SessionId"));

export type SessionId = typeof SessionId.Type;

export const PushSubscriptionId = Schema.Int.pipe(Schema.brand("PushSubscriptionId"));

export type PushSubscriptionId = typeof PushSubscriptionId.Type;

export const RememberedDeviceId = Schema.Int.pipe(Schema.brand("RememberedDeviceId"));

export type RememberedDeviceId = typeof RememberedDeviceId.Type;

export const WorkspaceIconId = Schema.Int.pipe(Schema.brand("WorkspaceIconId"));

export type WorkspaceIconId = typeof WorkspaceIconId.Type;

export const AuditLogEntryId = Schema.Int.pipe(Schema.brand("AuditLogEntryId"));

export type AuditLogEntryId = typeof AuditLogEntryId.Type;

export const CredentialId = Schema.Int.pipe(Schema.brand("CredentialId"));

export type CredentialId = typeof CredentialId.Type;

export const AgentGrantId = Schema.Int.pipe(Schema.brand("AgentGrantId"));

export type AgentGrantId = typeof AgentGrantId.Type;

export const SlackRunId = Schema.Int.pipe(Schema.brand("SlackRunId"));

export type SlackRunId = typeof SlackRunId.Type;
