import * as LiveKit from "livekit-client";
import { describe, expect, it } from "vitest";
import { ActionError } from "../../../sync/run.ts";
import { liveKitTransport } from "./livekit.ts";

function refusedTransport(error: Error) {
  class RefusedRoom extends LiveKit.Room {
    override connect = async () => {
      throw error;
    };

    override disconnect = async () => undefined;
  }

  return liveKitTransport({ ...LiveKit, Room: RefusedRoom });
}

describe("gateway admission errors", () => {
  it("translates the SDK validation refusal into the existing full-room error", async () => {
    const transport = refusedTransport(
      LiveKit.ConnectionError.notAllowed("This voice room is full\n", 403),
    );

    await expect(transport.connect("wss://livekit.test", "token", false, 48_000)).rejects.toEqual(
      new ActionError("HuddleRoomFull", "This voice room is full"),
    );
    await transport.disconnect();
  });

  it("recognises the refusal as livekit-client 2.22.3 wraps it after validating the token", async () => {
    // Room.connect rethrows a serverUnreachable error carrying the inner status and message.
    const wrapped = LiveKit.ConnectionError.serverUnreachable(
      "could not establish signal connection",
    );

    wrapped.message = `${wrapped.message}: This voice room is full\n`;
    wrapped.status = 403;

    const transport = refusedTransport(wrapped);

    await expect(transport.connect("wss://livekit.test", "token", false, 48_000)).rejects.toEqual(
      new ActionError("HuddleRoomFull", "This voice room is full"),
    );
    await transport.disconnect();
  });

  it.each([
    LiveKit.ConnectionError.notAllowed("Forbidden\n", 403),
    LiveKit.ConnectionError.notAllowed("could not establish signal connection: Forbidden", 403),
    LiveKit.ConnectionError.notAllowed("This voice room is full\n", 401),
    new Error("This voice room is full"),
  ])("preserves other refusals and connection failures", async (error) => {
    const transport = refusedTransport(error);
    await expect(transport.connect("wss://livekit.test", "token", false, 48_000)).rejects.toBe(
      error,
    );
    await transport.disconnect();
  });
});
