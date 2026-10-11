import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HuddleCredentials } from "../../../gen/HuddleCredentials.ts";
import { mutations } from "../../../store/store.ts";
import {
  CallController,
  type CallEnvironment,
  endCallOnAccountChange,
} from "../call-controller.ts";
import { callStore, initialCallState } from "../call-store.ts";
import { EMPTY_DEVICE_LISTS } from "./devices.ts";
import { liveKitTransport } from "./livekit.ts";
import type { CallTransport } from "./transport.ts";

class FakeTrack {
  readonly stop = vi.fn();
}

/** Every capture waits on `capture`, which a test settles once the call around it has ended. */
let capture = Promise.withResolvers<FakeTrack[]>();

class FakeLocalParticipant {
  readonly trackPublications = new Map<string, { track: FakeTrack }>();
  readonly createTracks = vi.fn(() => capture.promise);
  readonly createScreenTracks = vi.fn(() => capture.promise);
  readonly publishTrack = vi.fn(async () => ({}));
  readonly setMicrophoneEnabled = vi.fn(async () => undefined);
  readonly getTrackPublication = (): undefined => undefined;
}

const rooms: FakeRoom[] = [];

class FakeRoom {
  readonly localParticipant = new FakeLocalParticipant();
  readonly remoteParticipants = new Map<string, never>();
  readonly connect = vi.fn(async () => undefined);
  readonly disconnect = vi.fn(async () => undefined);
  readonly startAudio = vi.fn(async () => undefined);

  constructor() {
    rooms.push(this);
  }

  on(): void {}

  off(): void {}
}

/** Just enough of livekit-client for the transport to connect, capture and publish. */
const fakeSdk = {
  Room: FakeRoom,
  RoomEvent: new Proxy({}, { get: (_target, name) => String(name) }),
  Track: {
    Source: { Microphone: "microphone", Camera: "camera", ScreenShare: "screen_share" },
    Kind: { Audio: "audio", Video: "video" },
  },
  TrackEvent: { Ended: "ended" },
  DisconnectReason: {},
  AudioPresets: { music: {} },
  ScreenSharePresets: { h1080fps15: { encoding: {} } },
  VideoPresets: { h720: { resolution: {}, encoding: {} } },
  VideoPreset: class {},
  VideoQuality: {},
};

function transport(): CallTransport {
  // SAFETY: a stand-in for the SDK, not the SDK's type: these paths (connect, capture, publish,
  // mute, disconnect) only reach the members `fakeSdk` provides, and any other would throw.
  return liveKitTransport(fakeSdk as never);
}

const ROOM = 8;

function credentials(roomId: number): HuddleCredentials {
  return {
    url: "wss://livekit.test",
    token: "token",
    identity: `viewer-${roomId}`,
    grantId: roomId * 10,
    roomId,
    roomName: `Room ${roomId}`,
    canPublish: true,
  };
}

function controller(): CallController {
  const env: CallEnvironment = {
    join: async (roomId) => credentials(roomId),
    leave: async () => undefined,
    leaveOnUnload: () => undefined,
    check: async () => undefined,
    startStream: async () => {
      throw new Error("not used");
    },
    stopStream: async () => undefined,
    stopStreamOnUnload: () => undefined,
    preload: async () => undefined,
    transport: async () => transport(),
    shouldCheckDevices: async () => false,
    getUserMedia: async () => {
      throw new Error("no devices in tests");
    },
    listDevices: async () => EMPTY_DEVICE_LISTS,
    holdRoom: () => undefined,
    releaseRoom: () => undefined,
    opensCallView: () => true,
  };

  return new CallController(env);
}

function settle(): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
}

/** The room the current call's transport opened, once its microphone capture has started. */
async function capturing(): Promise<FakeRoom> {
  await vi.waitFor(() => {
    expect(rooms.at(-1)?.localParticipant.createTracks).toHaveBeenCalled();
  });

  const room = rooms.at(-1);

  if (room === undefined) {
    throw new Error("no room");
  }

  return room;
}

beforeEach(() => {
  rooms.length = 0;
  capture = Promise.withResolvers();
  callStore.setState(initialCallState, true);
  mutations.reset();
});

afterEach(() => {
  mutations.reset();
});

describe("a capture that settles after the call ended", () => {
  it("is stopped, never published, when another account answers the page mid-join", async () => {
    const call = controller();
    const stop = endCallOnAccountChange(call);
    const joining = call.join(ROOM, "Lounge", null);
    const room = await capturing();

    mutations.markAccountChanged();
    await settle();
    expect(room.disconnect).toHaveBeenCalled();

    const microphone = new FakeTrack();

    capture.resolve([microphone]);
    await joining;
    await settle();

    expect(room.localParticipant.publishTrack).not.toHaveBeenCalled();
    expect(microphone.stop).toHaveBeenCalled();
    expect(callStore.getState().phase).toBe("idle");
    stop();
  });

  it("is stopped, never published, after Leave during the capture", async () => {
    const call = controller();
    const joining = call.join(ROOM, "Lounge", null);
    const room = await capturing();

    await call.leave();
    expect(room.disconnect).toHaveBeenCalled();

    const microphone = new FakeTrack();

    capture.resolve([microphone]);
    await joining;
    await settle();

    expect(room.localParticipant.publishTrack).not.toHaveBeenCalled();
    expect(microphone.stop).toHaveBeenCalled();
    expect(callStore.getState().phase).toBe("idle");
  });

  it("is stopped for a camera and a screen share too", async () => {
    const call = transport();

    await call.connect("wss://livekit.test", "token", false);

    const [room] = rooms;
    const camera = call.setCamera(true);
    const screen = call.setScreenShare(true);

    await call.disconnect();

    const tracks = [new FakeTrack(), new FakeTrack()];

    capture.resolve(tracks);
    await camera;
    await screen;

    expect(room?.localParticipant.publishTrack).not.toHaveBeenCalled();

    for (const track of tracks) {
      expect(track.stop).toHaveBeenCalled();
    }
  });

  it("is stopped when a share captured ahead of its stream lands late", async () => {
    const call = transport();

    Object.defineProperty(navigator, "mediaDevices", {
      configurable: true,
      // Only its presence matters: the fake SDK captures the screen itself.
      value: { getDisplayMedia: () => undefined },
    });

    try {
      await call.connect("wss://livekit.test", "token", false);

      const sharing = call.captureScreen("1080p15");

      await call.disconnect();

      const screen = new FakeTrack();

      capture.resolve([screen]);

      await expect(sharing).rejects.toThrow("call-ended");
      expect(screen.stop).toHaveBeenCalled();
    } finally {
      Reflect.deleteProperty(navigator, "mediaDevices");
    }
  });
});

describe("a capture in a live call", () => {
  it("publishes, and a microphone turned off meanwhile closes after it lands", async () => {
    const call = transport();

    await call.connect("wss://livekit.test", "token", false);

    const [room] = rooms;
    const opening = call.setMicrophone(true);
    const closing = call.setMicrophone(false);
    const microphone = new FakeTrack();

    capture.resolve([microphone]);
    await opening;
    await closing;

    expect(room?.localParticipant.publishTrack).toHaveBeenCalledWith(microphone, undefined);
    expect(microphone.stop).not.toHaveBeenCalled();

    const published = room?.localParticipant.publishTrack.mock.invocationCallOrder[0] ?? 0;
    const muted = room?.localParticipant.setMicrophoneEnabled.mock.invocationCallOrder[0] ?? 0;

    expect(muted).toBeGreaterThan(published);
    await call.disconnect();
  });
});
