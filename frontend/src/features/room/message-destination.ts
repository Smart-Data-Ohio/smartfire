import type { MessageDTO } from "../../gen/MessageDTO.ts";

/** Where `/app/r/:roomId/m/:messageId` should go once the message is known. */
export type MessageAnchor =
  | { readonly kind: "here" }
  | { readonly kind: "elsewhere"; readonly href: string };

/** The viewer already belongs to the room, or is still on its join preview. */
export type RoomAccess = "member" | "unjoined";

/**
 * What the room route should do with a permalink.
 *
 * `focus` opens this room. `messageId` is the permalink, or `null` when the present is right
 * (the URL names no message, or a member's read failed and the anchor is dropped).
 * `redirect` leaves for the conversation the message actually belongs to.
 */
export type PermalinkTarget =
  | { readonly kind: "focus"; readonly messageId: number | null }
  | { readonly kind: "redirect"; readonly href: string };

/** A permalink read: not finished, found, or unreachable. */
export type PermalinkRead =
  | { readonly status: "pending" }
  | { readonly status: "missing" }
  | {
      readonly status: "found";
      readonly message: Pick<MessageDTO, "id" | "roomId" | "threadId">;
    };

/**
 * A timeline permalink stays when the message is a root message of that room. A message in
 * another room, or a reply, opens where the message actually is.
 */
export function messageAnchor(
  urlRoomId: number,
  message: Pick<MessageDTO, "id" | "roomId" | "threadId">,
): MessageAnchor {
  if (message.roomId === urlRoomId && message.threadId === null) {
    return { kind: "here" };
  }

  return { kind: "elsewhere", href: messageDestination(message) };
}

/**
 * Keeps a permalink that was set while the room was still unjoined.
 *
 * The message read 404s until the viewer is a member, so that failure must not drop the target:
 * joining loads around the id still on the URL. Once they belong, a root message of this room
 * stays here, a message that lives elsewhere is redirected, and a still-missing message opens
 * the present.
 */
export function permalinkTarget(
  roomId: number,
  focusMessageId: number | null,
  access: RoomAccess,
  read: PermalinkRead,
): PermalinkTarget {
  if (focusMessageId === null || read.status === "pending") {
    return { kind: "focus", messageId: focusMessageId };
  }

  if (read.status === "found") {
    const anchor = messageAnchor(roomId, read.message);

    return anchor.kind === "here"
      ? { kind: "focus", messageId: focusMessageId }
      : { kind: "redirect", href: anchor.href };
  }

  if (access === "unjoined") {
    return { kind: "focus", messageId: focusMessageId };
  }

  return { kind: "focus", messageId: null };
}

/** The conversation permalink for a reachable message, with the incoming query kept. */
export function messageDestination(
  message: Pick<MessageDTO, "id" | "roomId" | "threadId">,
  search = "",
  hash = "",
): string {
  const params = new URLSearchParams(search);

  const path =
    message.threadId === null
      ? `/app/r/${message.roomId}/m/${message.id}`
      : `/app/r/${message.roomId}/t/${message.threadId}`;

  if (message.threadId !== null) {
    params.set("m", String(message.id));
  }

  const query = params.toString();
  const fragment = hash === "" ? "" : `#${hash.replace(/^#/, "")}`;

  return `${path}${query === "" ? "" : `?${query}`}${fragment}`;
}
