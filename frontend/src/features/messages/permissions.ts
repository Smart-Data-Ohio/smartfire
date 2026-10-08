import type { ThreadStatus } from "../../gen/ThreadStatus.ts";
import type { UserRole } from "../../gen/UserRole.ts";
import type { MessageDTO, RoomKind } from "../../store/model.ts";

/** Who's looking, and where the message sits: what the permission rules need. */
export interface MessageContext {
  readonly viewerId: number | null;
  readonly viewerRole: UserRole | null;
  /** The room's kind, when its detail is loaded. */
  readonly roomKind: RoomKind | null;
  /** The status of the thread a reply sits in; `null` on the root timeline or when unknown. */
  readonly threadStatus: ThreadStatus | null;
}

/** What the viewer may do with one message. */
export interface MessagePermissions {
  readonly edit: boolean;
  readonly remove: boolean;
  readonly pin: boolean;
  readonly save: boolean;
  readonly react: boolean;
  readonly forward: boolean;
  /** Start or open a thread from it (root messages outside direct rooms). */
  readonly thread: boolean;
  /** Mark the room unread from it (root timeline only). */
  readonly markUnread: boolean;
  /** Create a Fizzy card from it (any message but a system note, connected or not, even locked). */
  readonly fizzy: boolean;
}

/**
 * The contract's rules (`MessageDTO` docs, `messages#ensure_can_edit` / `ensure_can_delete`):
 * edit is the creator's, never on a system note, never in a locked thread; delete is the
 * creator's or an administrator's, never on a system note; pin, save, react and forward are for
 * any human member. Bots never act through this UI.
 */
export function messagePermissions(
  message: MessageDTO,
  context: MessageContext,
): MessagePermissions {
  const human = context.viewerId !== null && context.viewerRole !== "bot";
  const own = human && message.creatorId === context.viewerId;
  const admin = human && context.viewerRole === "administrator";
  const note = message.systemNote;
  const locked = message.threadId !== null && context.threadStatus === "locked";
  const root = message.threadId === null;

  return {
    edit: own && !note && !locked,
    remove: (own || admin) && !note,
    pin: human,
    save: human,
    react: human,
    forward: human && !note,
    thread: human && !note && (root || message.thread !== null) && context.roomKind !== "direct",
    markUnread: human && root,
    fizzy: human && !note,
  };
}
