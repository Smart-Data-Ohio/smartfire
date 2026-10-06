import { Schema } from "effect";
import type { ActivityEventType as GeneratedActivityEventType } from "../../gen/ActivityEventType.ts";
import type { ActivityItem as GeneratedActivityItem } from "../../gen/ActivityItem.ts";
import type { ActivityItemChanged as GeneratedActivityItemChanged } from "../../gen/ActivityItemChanged.ts";
import type { ActivityItemRemoved as GeneratedActivityItemRemoved } from "../../gen/ActivityItemRemoved.ts";
import type { ActivityList as GeneratedActivityList } from "../../gen/ActivityList.ts";
import type { ActivitySource as GeneratedActivitySource } from "../../gen/ActivitySource.ts";
import type { ActivitySourceType as GeneratedActivitySourceType } from "../../gen/ActivitySourceType.ts";
import type { ActivityState as GeneratedActivityState } from "../../gen/ActivityState.ts";
import type { ActivityTab as GeneratedActivityTab } from "../../gen/ActivityTab.ts";
import type { ActivityUnreadCount as GeneratedActivityUnreadCount } from "../../gen/ActivityUnreadCount.ts";
import type { UpdateActivityItem as GeneratedUpdateActivityItem } from "../../gen/UpdateActivityItem.ts";
import { ActivityItemId, EventId, MessageId, RoomId, ThreadId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";

/** `activity_items.event_type`: all twenty. */
export const ActivityEventType = Schema.Literals([
  "mention",
  "reply",
  "thread_activity",
  "keyword_alert",
  "work_update",
  "work_assignment",
  "work_sla",
  "huddle_started",
  "huddle_missed",
  "event_invitation",
  "event_update",
  "event_cancelled",
  "event_reminder",
  "pr_review_request",
  "agent_approval_request",
  "agent_budget_exceeded",
  "message_reminder",
  "scheduled_message_dropped",
  "two_factor_lockout",
  "new_sign_in",
]);

export type ActivityEventType = typeof ActivityEventType.Type;

export type ActivityEventTypePin = Assert<
  Pinned<typeof ActivityEventType, GeneratedActivityEventType>
>;

export const ActivitySourceType = Schema.Literals([
  "message",
  "saved_item",
  "work_thread_event",
  "board_sla_nudge",
  "huddle_grant",
  "event",
  "agent_approval",
  "agent_budget_notice",
  "scheduled_message",
  "two_factor_credential",
  "session",
]);

export type ActivitySourceType = typeof ActivitySourceType.Type;

export type ActivitySourceTypePin = Assert<
  Pinned<typeof ActivitySourceType, GeneratedActivitySourceType>
>;

/** Derived from the timestamps: handled, else read, else unread. */
export const ActivityState = Schema.Literals(["unread", "read", "handled"]);

export type ActivityState = typeof ActivityState.Type;

export type ActivityStatePin = Assert<Pinned<typeof ActivityState, GeneratedActivityState>>;

/** The inbox's type tabs; `scheduled_message_dropped` shows only under `all`. */
export const ActivityTab = Schema.Literals([
  "all",
  "mentions",
  "threads",
  "events",
  "agents",
  "github",
  "huddles",
  "reminders",
  "security",
]);

export type ActivityTab = typeof ActivityTab.Type;

export type ActivityTabPin = Assert<Pinned<typeof ActivityTab, GeneratedActivityTab>>;

/** What an item is about, with the classic row's title, body and timestamp. */
export const ActivitySource = Schema.Struct({
  sourceType: ActivitySourceType,
  sourceId: Schema.Int,
  roomId: Schema.NullOr(RoomId),
  threadId: Schema.NullOr(ThreadId),
  messageId: Schema.NullOr(MessageId),
  eventId: Schema.NullOr(EventId),
  creatorId: Schema.NullOr(UserId),
  title: Schema.String,
  body: Schema.String,
  occurredAt: Timestamp,
  status: Schema.NullOr(Schema.String),
  path: Schema.String,
});

export type ActivitySource = typeof ActivitySource.Type;

export type ActivitySourcePin = Assert<Pinned<typeof ActivitySource, GeneratedActivitySource>>;

export const ActivityItem = Schema.Struct({
  id: ActivityItemId,
  eventType: ActivityEventType,
  state: ActivityState,
  readAt: Schema.NullOr(Timestamp),
  handledAt: Schema.NullOr(Timestamp),
  createdAt: Timestamp,
  updatedAt: Timestamp,
  source: Schema.NullOr(ActivitySource),
});

export type ActivityItem = typeof ActivityItem.Type;

export type ActivityItemPin = Assert<Pinned<typeof ActivityItem, GeneratedActivityItem>>;

/** `GET /api/v1/activity?status=&type=&before=`: up to 100, newest `updatedAt` first. */
export const ActivityList = Schema.Struct({
  items: Schema.Array(ActivityItem),
  users: Schema.Array(User),
  unreadCount: Schema.Int,
  nextCursor: Schema.NullOr(ActivityItemId),
});

export type ActivityList = typeof ActivityList.Type;

export type ActivityListPin = Assert<Pinned<typeof ActivityList, GeneratedActivityList>>;

/** `GET /api/v1/activity/unread_count`: the badge. */
export const ActivityUnreadCount = Schema.Struct({ unreadCount: Schema.Int });

export type ActivityUnreadCount = typeof ActivityUnreadCount.Type;

export type ActivityUnreadCountPin = Assert<
  Pinned<typeof ActivityUnreadCount, GeneratedActivityUnreadCount>
>;

/** The body of `PATCH /api/v1/activity/:id`. */
export const UpdateActivityItem = Schema.Struct({ state: ActivityState });

export type UpdateActivityItem = typeof UpdateActivityItem.Type;

export type UpdateActivityItemPin = Assert<
  Pinned<typeof UpdateActivityItem, GeneratedUpdateActivityItem>
>;

/** A state change's or `open`'s reply, and the `activity.item` event. */
export const ActivityItemChanged = Schema.Struct({ item: ActivityItem, unreadCount: Schema.Int });

export type ActivityItemChanged = typeof ActivityItemChanged.Type;

export type ActivityItemChangedPin = Assert<
  Pinned<typeof ActivityItemChanged, GeneratedActivityItemChanged>
>;

/** The `activity.removed` event: the item went with its source. */
export const ActivityItemRemoved = Schema.Struct({ id: ActivityItemId, unreadCount: Schema.Int });

export type ActivityItemRemoved = typeof ActivityItemRemoved.Type;

export type ActivityItemRemovedPin = Assert<
  Pinned<typeof ActivityItemRemoved, GeneratedActivityItemRemoved>
>;
