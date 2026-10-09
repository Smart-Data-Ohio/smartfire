import { joinHint } from "./alerts.ts";
import { callController } from "./call-controller.ts";

type Join = (roomId: number, roomName: string, canPublishHint: boolean | null) => Promise<void>;

const join: Join = (roomId, roomName, hint) => callController.join(roomId, roomName, hint);

/**
 * `/huddle`'s `start_huddle` result: join the call of the room the command ran in, as classic
 * hands that room to its huddle controller. The server already said huddles are configured, and
 * classic's join endpoints take any live room the user belongs to, board posts included, so
 * there is no kind check here. A call already up in the room comes to the front.
 */
export function startHuddleFromCommand(roomId: number, roomName: string, joinCall: Join = join) {
  void joinCall(roomId, roomName, joinHint(roomId));
}
