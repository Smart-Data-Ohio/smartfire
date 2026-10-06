import { Schema } from "effect";
import type { ThreadIndicator as GeneratedThreadIndicator } from "../../gen/ThreadIndicator.ts";
import type { ThreadStatus as GeneratedThreadStatus } from "../../gen/ThreadStatus.ts";
import { ThreadId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/**
 * The thread pieces a message refers to, kept apart from `thread.ts` (which needs messages) so
 * the two modules don't import each other.
 */

/** `locked`, else `closed` (closed, or idle past auto-archive), else `active`. */
export const ThreadStatus = Schema.Literals(["active", "closed", "locked"]);

export type ThreadStatus = typeof ThreadStatus.Type;

export type ThreadStatusPin = Assert<Pinned<typeof ThreadStatus, GeneratedThreadStatus>>;

/** A root message's reply indicator: count, last reply, up to 3 recent repliers. */
export const ThreadIndicator = Schema.Struct({
  threadId: ThreadId,
  replyCount: Schema.Int,
  lastReplyAt: Timestamp,
  replierIds: Schema.Array(UserId),
});

export type ThreadIndicator = typeof ThreadIndicator.Type;

export type ThreadIndicatorPin = Assert<Pinned<typeof ThreadIndicator, GeneratedThreadIndicator>>;
