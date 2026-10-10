/** The S2 thread endpoints: list, detail, replies, create, update, follow and read state. */
import { Effect } from "effect";
import type { CreateMessage } from "../gen/CreateMessage.ts";
import type { CreateThread } from "../gen/CreateThread.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { MessagePage } from "../gen/MessagePage.ts";
import type { ThreadCreated } from "../gen/ThreadCreated.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { ThreadFilter } from "../gen/ThreadFilter.ts";
import type { ThreadInvolvement } from "../gen/ThreadInvolvement.ts";
import type { ThreadList } from "../gen/ThreadList.ts";
import type { ThreadMembershipState } from "../gen/ThreadMembershipState.ts";
import type { UpdateThread } from "../gen/UpdateThread.ts";
import { call, get, noContent } from "./call.ts";
import { cursorQuery, type PageCursor } from "./endpoints.ts";
import { MessagePage as MessagePageSchema, MessageDTO as MessageSchema } from "./schema/message.ts";
import {
  ThreadCreated as ThreadCreatedSchema,
  ThreadDetail as ThreadDetailSchema,
  ThreadList as ThreadListSchema,
  ThreadMembershipState as ThreadMembershipStateSchema,
} from "./schema/thread.ts";
import { wire } from "./wire.ts";

/** `GET /rooms/:id/threads?state=`: the room's threads, most recently active first. */
export const threads = Effect.fn("api.threads")(function* (roomId: number, state: ThreadFilter) {
  return yield* call(
    get(`/rooms/${roomId}/threads`, { state }),
    wire<ThreadList>(ThreadListSchema),
  );
});

/** `GET /threads/:id`: the pane's header data. */
export const thread = Effect.fn("api.thread")(function* (threadId: number) {
  return yield* call(get(`/threads/${threadId}`), wire<ThreadDetail>(ThreadDetailSchema));
});

/** `GET /threads/:id/messages`: a page of replies, paged like a room's timeline. */
export const threadMessages = Effect.fn("api.threadMessages")(function* (
  threadId: number,
  cursor: PageCursor,
) {
  return yield* call(
    get(`/threads/${threadId}/messages`, cursorQuery(cursor)),
    wire<MessagePage>(MessagePageSchema),
  );
});

/** `POST /threads/:id/messages`: a reply; idempotent on `clientMessageId` like a room post. */
export const createThreadMessage = Effect.fn("api.createThreadMessage")(function* (
  threadId: number,
  body: CreateMessage,
) {
  return yield* call(
    { method: "POST", path: `/threads/${threadId}/messages`, body },
    wire<MessageDTO>(MessageSchema),
  );
});

/** `POST /rooms/:id/threads`: starts a thread on a root message with its first reply. */
export const createThread = Effect.fn("api.createThread")(function* (
  roomId: number,
  body: CreateThread,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/threads`, body },
    wire<ThreadCreated>(ThreadCreatedSchema),
  );
});

/** `PATCH /threads/:id`: rename, set the auto-archive duration, close, reopen, lock or unlock. */
export const updateThread = Effect.fn("api.updateThread")(function* (
  threadId: number,
  body: UpdateThread,
) {
  return yield* call(
    { method: "PATCH", path: `/threads/${threadId}`, body },
    wire<ThreadDetail>(ThreadDetailSchema),
  );
});

/** `DELETE /threads/:id` (204): deletes it and its replies; `thread.removed` follows. */
export const deleteThread = Effect.fn("api.deleteThread")(function* (threadId: number) {
  return yield* call({ method: "DELETE", path: `/threads/${threadId}` }, noContent);
});

/** `POST /threads/:id/join` (follow: `everything`) or `DELETE` (leave). */
export const joinThread = Effect.fn("api.joinThread")(function* (
  threadId: number,
  involvement: ThreadInvolvement | null,
) {
  return yield* call(
    involvement === null
      ? { method: "DELETE", path: `/threads/${threadId}/join` }
      : { method: "POST", path: `/threads/${threadId}/join`, body: { involvement } },
    wire<ThreadMembershipState>(ThreadMembershipStateSchema),
  );
});

/** `POST /threads/:id/read`: the thread is read up to its newest reply. */
export const markThreadRead = Effect.fn("api.markThreadRead")(function* (threadId: number) {
  return yield* call(
    { method: "POST", path: `/threads/${threadId}/read` },
    wire<ThreadMembershipState>(ThreadMembershipStateSchema),
  );
});
