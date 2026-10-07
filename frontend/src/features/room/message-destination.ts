import type { MessageDTO } from "../../gen/MessageDTO.ts";

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
