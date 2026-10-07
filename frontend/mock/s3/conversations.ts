/**
 * What cross-room lists (saved items, scheduled messages) and inbox rows call a room and thread,
 * as the classic pages do: the viewer-relative room name and the thread's name. Rooms the viewer
 * can't see any more are named too, so a stranded or dropped row still reads right.
 */
import type { ConversationName } from "../../src/gen/ConversationName.ts";
import type { S2Context } from "../s2/context.ts";

/** What naming needs from the server: the world and its viewer-relative room names. */
export type Namer = Pick<S2Context, "world" | "displayName">;

/** A room and, for a thread, its id. */
export interface ConversationRef {
  readonly roomId: number;
  readonly threadId: number | null;
}

/** One name per distinct `(roomId, threadId)` among `refs`, in first-seen order. */
export function conversationNames(ctx: Namer, refs: Iterable<ConversationRef>): ConversationName[] {
  const world = ctx.world();
  const seen = new Map<string, ConversationName>();

  for (const ref of refs) {
    const key = `${ref.roomId}:${ref.threadId ?? ""}`;
    const record = world.rooms.get(ref.roomId);

    if (seen.has(key) || record === undefined) continue;

    const thread = ref.threadId === null ? undefined : world.threads.get(ref.threadId);

    seen.set(key, {
      roomId: record.room.id,
      threadId: ref.threadId,
      roomKind: record.room.kind,
      roomName: ctx.displayName(record),
      roomIconName: record.room.iconName,
      threadName: thread?.name ?? null,
    });
  }

  return [...seen.values()];
}

/** An inbox row's heading: the room's name, then ` · ` and the thread's (or an event's title). */
export function conversationTitle(
  ctx: Namer,
  roomId: number,
  threadId: number | null,
  suffix: string | null = null,
): string {
  const world = ctx.world();
  const record = world.rooms.get(roomId);
  const room = record === undefined ? "Unknown room" : ctx.displayName(record);
  const thread = threadId === null ? null : (world.threads.get(threadId)?.name ?? null);
  const tail = suffix ?? thread;

  return tail === null ? room : `${room} · ${tail}`;
}
