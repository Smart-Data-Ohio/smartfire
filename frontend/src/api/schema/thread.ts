import { Schema } from "effect";
import type { CreateThread as GeneratedCreateThread } from "../../gen/CreateThread.ts";
import type { JoinThread as GeneratedJoinThread } from "../../gen/JoinThread.ts";
import type { Thread as GeneratedThread } from "../../gen/Thread.ts";
import type { ThreadCreated as GeneratedThreadCreated } from "../../gen/ThreadCreated.ts";
import type { ThreadDetail as GeneratedThreadDetail } from "../../gen/ThreadDetail.ts";
import type { ThreadFilter as GeneratedThreadFilter } from "../../gen/ThreadFilter.ts";
import type { ThreadIndicatorChanged as GeneratedThreadIndicatorChanged } from "../../gen/ThreadIndicatorChanged.ts";
import type { ThreadInvolvement as GeneratedThreadInvolvement } from "../../gen/ThreadInvolvement.ts";
import type { ThreadList as GeneratedThreadList } from "../../gen/ThreadList.ts";
import type { ThreadMembership as GeneratedThreadMembership } from "../../gen/ThreadMembership.ts";
import type { ThreadMembershipState as GeneratedThreadMembershipState } from "../../gen/ThreadMembershipState.ts";
import type { ThreadPermissions as GeneratedThreadPermissions } from "../../gen/ThreadPermissions.ts";
import type { ThreadRead as GeneratedThreadRead } from "../../gen/ThreadRead.ts";
import type { ThreadRemoved as GeneratedThreadRemoved } from "../../gen/ThreadRemoved.ts";
import type { ThreadSummary as GeneratedThreadSummary } from "../../gen/ThreadSummary.ts";
import type { ThreadUnread as GeneratedThreadUnread } from "../../gen/ThreadUnread.ts";
import type { UpdateThread as GeneratedUpdateThread } from "../../gen/UpdateThread.ts";
import { MessageId, RoomId, ThreadId, UserId } from "./ids.ts";
import { CreateMessage, MessageDTO } from "./message.ts";
import type { Assert, Pinned } from "./pin.ts";
import { ThreadIndicator, ThreadStatus } from "./thread-parts.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";

export { ThreadIndicator, ThreadStatus } from "./thread-parts.ts";

/** A channel thread as every room member sees it. */
export const Thread = Schema.Struct({
  id: ThreadId,
  roomId: RoomId,
  parentMessageId: Schema.NullOr(MessageId),
  creatorId: UserId,
  name: Schema.String,
  status: ThreadStatus,
  replyCount: Schema.Int,
  lastActivityAt: Timestamp,
  autoArchiveAfterMinutes: Schema.Int,
  createdAt: Timestamp,
});

export type Thread = typeof Thread.Type;

export type ThreadPin = Assert<Pinned<typeof Thread, GeneratedThread>>;

/** `nothing` (muted), `mentions` (the default, not following), `everything` (following). */
export const ThreadInvolvement = Schema.Literals(["nothing", "mentions", "everything"]);

export type ThreadInvolvement = typeof ThreadInvolvement.Type;

export type ThreadInvolvementPin = Assert<
  Pinned<typeof ThreadInvolvement, GeneratedThreadInvolvement>
>;

/** The viewer's place in a thread: involvement and unread state (no read position). */
export const ThreadMembership = Schema.Struct({
  threadId: ThreadId,
  involvement: ThreadInvolvement,
  unreadAt: Schema.NullOr(Timestamp),
  joinedAt: Timestamp,
});

export type ThreadMembership = typeof ThreadMembership.Type;

export type ThreadMembershipPin = Assert<
  Pinned<typeof ThreadMembership, GeneratedThreadMembership>
>;

export const ThreadSummary = Schema.Struct({
  thread: Thread,
  membership: Schema.NullOr(ThreadMembership),
});

export type ThreadSummary = typeof ThreadSummary.Type;

export type ThreadSummaryPin = Assert<Pinned<typeof ThreadSummary, GeneratedThreadSummary>>;

export const ThreadFilter = Schema.Literals(["active", "closed", "locked", "all"]);

export type ThreadFilter = typeof ThreadFilter.Type;

export type ThreadFilterPin = Assert<Pinned<typeof ThreadFilter, GeneratedThreadFilter>>;

/** `GET /api/v1/rooms/:id/threads?state=`: most recently active first. */
export const ThreadList = Schema.Struct({
  threads: Schema.Array(ThreadSummary),
  users: Schema.Array(User),
});

export type ThreadList = typeof ThreadList.Type;

export type ThreadListPin = Assert<Pinned<typeof ThreadList, GeneratedThreadList>>;

export const ThreadPermissions = Schema.Struct({
  canRename: Schema.Boolean,
  canClose: Schema.Boolean,
  canReopen: Schema.Boolean,
  canLock: Schema.Boolean,
  canUnlock: Schema.Boolean,
  canDelete: Schema.Boolean,
});

export type ThreadPermissions = typeof ThreadPermissions.Type;

export type ThreadPermissionsPin = Assert<
  Pinned<typeof ThreadPermissions, GeneratedThreadPermissions>
>;

/** `GET /api/v1/threads/:id`: the pane's header; replies come from `/threads/:id/messages`. */
export const ThreadDetail = Schema.Struct({
  thread: Thread,
  membership: Schema.NullOr(ThreadMembership),
  parentMessage: Schema.NullOr(MessageDTO),
  permissions: ThreadPermissions,
  users: Schema.Array(User),
});

export type ThreadDetail = typeof ThreadDetail.Type;

export type ThreadDetailPin = Assert<Pinned<typeof ThreadDetail, GeneratedThreadDetail>>;

/** The body of `POST /api/v1/rooms/:id/threads`: the parent and the first reply. */
export const CreateThread = Schema.Struct({
  parentMessageId: MessageId,
  name: Schema.NullOr(Schema.String),
  message: CreateMessage,
});

export type CreateThread = typeof CreateThread.Type;

export type CreateThreadPin = Assert<Pinned<typeof CreateThread, GeneratedCreateThread>>;

export const ThreadCreated = Schema.Struct({ detail: ThreadDetail, message: MessageDTO });

export type ThreadCreated = typeof ThreadCreated.Type;

export type ThreadCreatedPin = Assert<Pinned<typeof ThreadCreated, GeneratedThreadCreated>>;

/** The body of `PATCH /api/v1/threads/:id`; `null` leaves a field alone. */
export const UpdateThread = Schema.Struct({
  name: Schema.NullOr(Schema.String),
  status: Schema.NullOr(ThreadStatus),
});

export type UpdateThread = typeof UpdateThread.Type;

export type UpdateThreadPin = Assert<Pinned<typeof UpdateThread, GeneratedUpdateThread>>;

/** The body of `POST /api/v1/threads/:id/join`. */
export const JoinThread = Schema.Struct({ involvement: Schema.NullOr(ThreadInvolvement) });

export type JoinThread = typeof JoinThread.Type;

export type JoinThreadPin = Assert<Pinned<typeof JoinThread, GeneratedJoinThread>>;

/** The reply to joining or reading a thread. */
export const ThreadMembershipState = Schema.Struct({ membership: ThreadMembership });

export type ThreadMembershipState = typeof ThreadMembershipState.Type;

export type ThreadMembershipStatePin = Assert<
  Pinned<typeof ThreadMembershipState, GeneratedThreadMembershipState>
>;

/** The `thread.indicator` event: replaces the parent's `thread` (`null`: thread deleted). */
export const ThreadIndicatorChanged = Schema.Struct({
  roomId: RoomId,
  parentMessageId: MessageId,
  thread: Schema.NullOr(ThreadIndicator),
});

export type ThreadIndicatorChanged = typeof ThreadIndicatorChanged.Type;

export type ThreadIndicatorChangedPin = Assert<
  Pinned<typeof ThreadIndicatorChanged, GeneratedThreadIndicatorChanged>
>;

/** The `thread.removed` event. */
export const ThreadRemoved = Schema.Struct({ threadId: ThreadId, roomId: RoomId });

export type ThreadRemoved = typeof ThreadRemoved.Type;

export type ThreadRemovedPin = Assert<Pinned<typeof ThreadRemoved, GeneratedThreadRemoved>>;

/** The `thread.unread` event: unread for this member, or (`refreshOnly`) just refresh the row. */
export const ThreadUnread = Schema.Struct({
  threadId: ThreadId,
  roomId: RoomId,
  refreshOnly: Schema.Boolean,
});

export type ThreadUnread = typeof ThreadUnread.Type;

export type ThreadUnreadPin = Assert<Pinned<typeof ThreadUnread, GeneratedThreadUnread>>;

/** The `thread.read` event: read in another tab. */
export const ThreadRead = Schema.Struct({ threadId: ThreadId, roomId: RoomId });

export type ThreadRead = typeof ThreadRead.Type;

export type ThreadReadPin = Assert<Pinned<typeof ThreadRead, GeneratedThreadRead>>;
