/** Message, thread and summary fixtures for the threads and panes tests. */
import type { MessageDTO, Thread, ThreadMembership } from "../../store/model.ts";

export function messageFixture(id: number, extra: Partial<MessageDTO> = {}): MessageDTO {
  const createdAt = "2026-10-06T09:00:00.000Z";

  return {
    id,
    roomId: 4,
    threadId: null,
    creatorId: 2,
    clientMessageId: `client-${id}`,
    bodyHtml: `<p>Message ${id}</p>`,
    markdownSource: `Message ${id}`,
    systemNote: false,
    action: false,
    streaming: false,
    embedsSuppressed: false,
    replyToMessageId: null,
    forwardedFromMessageId: null,
    forwardedAt: null,
    forwardNote: null,
    editedAt: null,
    attachment: null,
    reactions: [],
    boosts: [],
    pinned: false,
    thread: null,
    poll: null,
    cards: [],
    createdAt,
    updatedAt: createdAt,
    ...extra,
  };
}

export function threadFixture(id: number, extra: Partial<Thread> = {}): Thread {
  return {
    id,
    roomId: 4,
    parentMessageId: id * 10,
    creatorId: 2,
    name: `Thread ${id}`,
    status: "active",
    replyCount: 1,
    lastActivityAt: "2026-10-06T09:00:00.000Z",
    autoArchiveAfterMinutes: 1440,
    createdAt: "2026-10-06T08:00:00.000Z",
    ...extra,
  };
}

export function membershipFixture(
  threadId: number,
  extra: Partial<ThreadMembership> = {},
): ThreadMembership {
  return {
    threadId,
    involvement: "mentions",
    unreadAt: null,
    joinedAt: "2026-10-06T08:00:00.000Z",
    ...extra,
  };
}
