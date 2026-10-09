import { store } from "../../store/store.ts";
import { joinHint } from "./alerts.ts";
import { callController } from "./call-controller.ts";

/** How `/huddle` went: the call started (or came to the front), or why it didn't. */
export type HuddleCommandOutcome =
  | { readonly kind: "joining" }
  | { readonly kind: "refused"; readonly title: string; readonly description: string };

type Join = (roomId: number, roomName: string, canPublishHint: boolean | null) => Promise<void>;

const join: Join = (roomId, roomName, hint) => callController.join(roomId, roomName, hint);

/**
 * `/huddle`'s `start_huddle` result: what the room header's call button does, for the room the
 * command ran in (classic hands the same room to its huddle controller). The server already said
 * huddles are configured; joining checks membership. Boards have no call button, so no call.
 * A call already up in the room comes to the front rather than leaving, whatever the room's kind.
 */
export function startHuddleFromCommand(
  roomId: number,
  roomName: string,
  joinCall: Join = join,
): HuddleCommandOutcome {
  if (store.getState().sidebar.rows[roomId]?.room.kind === "board") {
    return {
      kind: "refused",
      title: "Boards don't have calls",
      description: "Start a huddle from a channel or a direct message.",
    };
  }

  void joinCall(roomId, roomName, joinHint(roomId));

  return { kind: "joining" };
}
