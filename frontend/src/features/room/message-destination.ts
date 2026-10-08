import type { MessageDTO } from "../../gen/MessageDTO.ts";

/** Where `/app/r/:roomId/m/:messageId` should go once the message is known. */
export type MessageAnchor =
  | { readonly kind: "here" }
  | { readonly kind: "elsewhere"; readonly href: string };

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
