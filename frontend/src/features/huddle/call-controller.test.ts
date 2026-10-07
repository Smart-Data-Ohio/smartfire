import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { meFixture } from "../../api/testing.ts";
import type { HuddleCredentials } from "../../gen/HuddleCredentials.ts";
import { mutations, store } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import {
  AUTH_CHECK_INTERVAL_MS,
  CallController,
  type CallEnvironment,
  SERVER_MUTED_NOTICE,
} from "./call-controller.ts";
import { callStore, initialCallState } from "./call-store.ts";
import { EMPTY_DEVICE_LISTS } from "./engine/devices.ts";
import { FakeTransport } from "./engine/fake.ts";

const ROOM = 8;

const OTHER_ROOM = 9;

function credentials(roomId: number, canPublish = true): HuddleCredentials {
  return {
    url: "mock://livekit",
    token: JSON.stringify({ identity: `viewer-${roomId}`, userId: 7, canPublish, simulate: false }),
    identity: `viewer-${roomId}`,
    grantId: roomId * 10,
    roomId,
    roomName: `Room ${roomId}`,
    canPublish,
  };
}

interface Harness {
  readonly env: CallEnvironment;
  readonly controller: CallController;
  readonly transports: FakeTransport[];
  readonly calls: string[];
  readonly grants: { canPublish: boolean };
}

function harness(overrides: Partial<CallEnvironment> = {}): Harness {
  const transports: FakeTransport[] = [];
  const calls: string[] = [];
  const grants = { canPublish: true };

  const env: CallEnvironment = {
    join: async (roomId) => {
      calls.push(`join ${roomId}`);

      return credentials(roomId, grants.canPublish);
    },
    leave: async (roomId) => {
      calls.push(`leave ${roomId}`);
    },
    leaveOnUnload: (roomId) => {
      calls.push(`leave-on-unload ${roomId}`);
    },
    check: async () => undefined,
    startStream: async () => {
      throw new Error("not used");
    },
    stopStream: async () => undefined,
    stopStreamOnUnload: () => undefined,
    preload: async () => undefined,
    transport: async (_url, roomId) => {
      const transport = new FakeTransport(roomId);

      transports.push(transport);

      return transport;
    },
    shouldCheckDevices: async () => false,
    getUserMedia: async () => {
      throw new Error("no devices in tests");
    },
    listDevices: async () => EMPTY_DEVICE_LISTS,
    holdRoom: (roomId) => {
      calls.push(`hold ${roomId}`);
    },
    releaseRoom: (roomId) => {
      calls.push(`release ${roomId}`);
    },
    ...overrides,
  };

  return { env, controller: new CallController(env), transports, calls, grants };
}

/** Lets pending promises run (only intervals are faked, so a timeout still fires). */
function settle(): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
}

function key(type: "keydown" | "keyup", init: KeyboardEventInit): KeyboardEvent {
  return new KeyboardEvent(type, { cancelable: true, ...init });
}

function pushToTalk(enabled: boolean): void {
  store.setState({
    me: {
      ...meFixture,
      preferences: {
        ...meFixture.preferences,
        voiceMode: enabled ? "push_to_talk" : "voice_activity",
      },
    },
  });
}

beforeEach(() => {
  callStore.setState(initialCallState, true);
  mutations.reset();
  pushToTalk(false);
  localStorage.clear();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("joining", () => {
  it("connects, opens the microphone and holds the room's topic", async () => {
    const { controller, calls } = harness();

    await controller.join(ROOM, "Lounge", null);

    const state = callStore.getState();

    expect(state.phase).toBe("connected");
    expect(state.status).toBe("Huddle active");
    expect(state.roomName).toBe(`Room ${ROOM}`);
    expect(state.identity).toBe(`viewer-${ROOM}`);
    expect(state.snapshot.microphoneEnabled).toBe(true);
    expect(state.viewOpen).toBe(true);
    expect(calls).toEqual([`hold ${ROOM}`, `join ${ROOM}`]);

    await controller.leave();
  });

  it("stops at the device check on a first join and connects on Join", async () => {
    const { controller, calls } = harness({ shouldCheckDevices: async () => true });

    await controller.join(ROOM, "Lounge", null);

    expect(callStore.getState().phase).toBe("prejoin");
    expect(calls).not.toContain(`join ${ROOM}`);

    // Nothing could be previewed here; Join stays off until a microphone works.
    await controller.confirmPrejoin();
    expect(callStore.getState().phase).toBe("prejoin");

    const prejoin = callStore.getState().prejoin;

    expect(prejoin).not.toBeNull();

    if (prejoin !== null) {
      callStore.setState({ prejoin: { ...prejoin, canJoin: true } });
    }

    await controller.confirmPrejoin();

    expect(callStore.getState().phase).toBe("connected");
    await controller.leave();
  });

  it("skips the device check for a stage listener", async () => {
    const { controller, grants } = harness({ shouldCheckDevices: async () => true });

    grants.canPublish = false;
    await controller.join(ROOM, "Town Hall", false);

    const state = callStore.getState();

    expect(state.phase).toBe("connected");
    expect(state.canPublish).toBe(false);
    expect(state.snapshot.microphoneEnabled).toBe(false);
    await controller.leave();
  });

  it("drops a join that a newer one overtook", async () => {
    let release: () => void = () => undefined;

    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });

    const requested: number[] = [];

    const { controller, transports } = harness({
      join: async (roomId) => {
        requested.push(roomId);

        if (roomId === ROOM) {
          await gate;
        }

        return credentials(roomId);
      },
    });

    const first = controller.join(ROOM, "Lounge", null);

    await vi.waitFor(() => expect(requested).toEqual([ROOM]));
    await controller.join(OTHER_ROOM, "Other", null);
    release();
    await first;

    expect(callStore.getState().roomId).toBe(OTHER_ROOM);
    expect(callStore.getState().phase).toBe("connected");
    // The overtaken join never built a transport.
    expect(transports).toHaveLength(1);
    await controller.leave();
  });

  it("joining the call it's already in only opens the view", async () => {
    const { controller, calls } = harness();

    await controller.join(ROOM, "Lounge", null);
    controller.setViewOpen(false);
    await controller.join(ROOM, "Lounge", null);

    expect(callStore.getState().viewOpen).toBe(true);
    expect(calls.filter((call) => call.startsWith("join"))).toHaveLength(1);
    await controller.leave();
  });

  it("says why a join failed", async () => {
    const { controller } = harness({
      join: async () => {
        throw new ActionError("NotFound", "Not found");
      },
    });

    await controller.join(ROOM, "Lounge", null);

    const state = callStore.getState();

    expect(state.phase).toBe("failed");
    expect(state.failureTitle).toBe("Couldn’t join huddle");
    expect(state.notice).not.toBeNull();
  });
});

describe("leaving", () => {
  it("disconnects, reports the leave and releases the topic", async () => {
    const { controller, calls } = harness();

    await controller.join(ROOM, "Lounge", null);
    await controller.leave();

    expect(callStore.getState().phase).toBe("idle");
    expect(callStore.getState().roomId).toBeNull();
    expect(calls.slice(-2)).toEqual([`leave ${ROOM}`, `release ${ROOM}`]);
  });

  it("moves the topic hold when switching rooms", async () => {
    const { controller, calls } = harness();

    await controller.join(ROOM, "Lounge", null);
    await controller.join(OTHER_ROOM, "Other", null);

    expect(calls).toContain(`release ${ROOM}`);
    expect(calls.at(-2)).toBe(`hold ${OTHER_ROOM}`);
    await controller.leave();
  });

  it("ends without waiting when the page goes away", async () => {
    const { controller, calls } = harness();

    await controller.join(ROOM, "Lounge", null);
    controller.endForPageChange();

    expect(callStore.getState().phase).toBe("idle");
    expect(calls).toContain(`leave-on-unload ${ROOM}`);
    expect(calls.at(-1)).toBe(`release ${ROOM}`);
  });
});

describe("the connection", () => {
  it("counts down while reconnecting and recovers", async () => {
    const { controller } = harness();

    await controller.join(ROOM, "Lounge", null);
    window.__smartfireHuddle?.reconnecting();

    expect(callStore.getState().phase).toBe("reconnecting");
    expect(callStore.getState().reconnectSeconds).toBe(30);

    window.__smartfireHuddle?.reconnected();
    expect(callStore.getState().phase).toBe("connected");
    expect(callStore.getState().reconnectSeconds).toBeNull();
    await controller.leave();
  });

  it("ends with the reason when the connection drops, and Retry rejoins", async () => {
    const { controller, calls } = harness();

    await controller.join(ROOM, "Lounge", null);
    window.__smartfireHuddle?.disconnect("lost");

    await vi.waitFor(() => expect(callStore.getState().phase).toBe("failed"));
    expect(callStore.getState().failureTitle).toBe("Huddle ended");

    controller.retry();
    await vi.waitFor(() => expect(callStore.getState().phase).toBe("connected"));
    expect(calls.filter((call) => call.startsWith("join"))).toHaveLength(2);
    await controller.leave();
  });

  it("ends the call when the access check says the grant is gone", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });

    let allowed = true;

    const { controller } = harness({
      check: async () => {
        if (!allowed) {
          throw new ActionError("Forbidden", "Forbidden");
        }
      },
    });

    await controller.join(ROOM, "Lounge", null);
    allowed = false;
    await vi.advanceTimersByTimeAsync(AUTH_CHECK_INTERVAL_MS);
    await settle();

    expect(callStore.getState().phase).toBe("failed");
    expect(callStore.getState().notice).toBe("Your access to this room ended.");
  });

  it("keeps the call through a failed check that isn't a denial", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });

    const check = vi.fn(async () => {
      throw new ActionError("NetworkError", "offline");
    });

    const { controller } = harness({ check });

    await controller.join(ROOM, "Lounge", null);
    await vi.advanceTimersByTimeAsync(AUTH_CHECK_INTERVAL_MS);
    await settle();

    expect(check).toHaveBeenCalledTimes(2);
    expect(callStore.getState().phase).toBe("connected");
    await controller.leave();
  });
});

describe("a role change", () => {
  it("rejoins for a fresh token and says a host muted them", async () => {
    const { controller, grants, calls } = harness();

    await controller.join(ROOM, "Lounge", null);
    grants.canPublish = false;
    await controller.roleChanged(ROOM, null, true);

    let state = callStore.getState();

    expect(calls.filter((call) => call.startsWith("join"))).toHaveLength(2);
    expect(state.phase).toBe("connected");
    expect(state.canPublish).toBe(false);
    expect(state.notice).toBe(SERVER_MUTED_NOTICE);

    grants.canPublish = true;
    await controller.roleChanged(ROOM, null, false);
    state = callStore.getState();

    expect(state.canPublish).toBe(true);
    expect(state.notice).toBeNull();
    await controller.leave();
  });

  it("ignores a change for a room it isn't in", async () => {
    const { controller, calls } = harness();

    await controller.join(ROOM, "Lounge", null);
    await controller.roleChanged(OTHER_ROOM, "listener", false);

    expect(calls.filter((call) => call.startsWith("join"))).toHaveLength(1);
    await controller.leave();
  });
});

describe("the microphone", () => {
  it("deafening closes the microphone and undeafening reopens it", async () => {
    const { controller } = harness();

    await controller.join(ROOM, "Lounge", null);
    await controller.toggleDeafen();

    expect(callStore.getState().deafened).toBe(true);
    expect(callStore.getState().snapshot.microphoneEnabled).toBe(false);

    await controller.toggleDeafen();
    expect(callStore.getState().deafened).toBe(false);
    expect(callStore.getState().snapshot.microphoneEnabled).toBe(true);
    await controller.leave();
  });

  it("unmuting while deafened undeafens", async () => {
    const { controller } = harness();

    await controller.join(ROOM, "Lounge", null);
    await controller.toggleMute();
    await controller.toggleDeafen();
    await controller.toggleMute();

    expect(callStore.getState().deafened).toBe(false);
    expect(callStore.getState().snapshot.microphoneEnabled).toBe(true);
    await controller.leave();
  });

  it("Ctrl+Shift+M toggles the microphone", async () => {
    const { controller } = harness();

    await controller.join(ROOM, "Lounge", null);

    const event = key("keydown", { key: "M", ctrlKey: true, shiftKey: true });

    controller.keyDown(event);
    expect(event.defaultPrevented).toBe(true);
    await vi.waitFor(() => expect(callStore.getState().snapshot.microphoneEnabled).toBe(false));
    await controller.leave();
  });

  it("push-to-talk joins closed, opens while held and closes on release", async () => {
    pushToTalk(true);

    const { controller } = harness();

    await controller.join(ROOM, "Lounge", null);
    expect(callStore.getState().snapshot.microphoneEnabled).toBe(false);

    controller.keyDown(key("keydown", { key: "`", code: "Backquote" }));
    await vi.waitFor(() => expect(callStore.getState().snapshot.microphoneEnabled).toBe(true));

    controller.keyUp(key("keyup", { key: "`", code: "Backquote" }));
    await vi.waitFor(() => expect(callStore.getState().snapshot.microphoneEnabled).toBe(false));
    await controller.leave();
  });

  it("push-to-talk matches the dead backtick by position and releases on blur", async () => {
    pushToTalk(true);

    const { controller } = harness();

    await controller.join(ROOM, "Lounge", null);
    controller.keyDown(key("keydown", { key: "Dead", code: "Backquote" }));
    await vi.waitFor(() => expect(callStore.getState().snapshot.microphoneEnabled).toBe(true));

    controller.blurred();
    await vi.waitFor(() => expect(callStore.getState().snapshot.microphoneEnabled).toBe(false));
    await controller.leave();
  });

  it("push-to-talk does nothing while deafened, typing or with a modifier", async () => {
    pushToTalk(true);

    const { controller } = harness();

    await controller.join(ROOM, "Lounge", null);

    const input = document.createElement("textarea");

    document.body.append(input);
    controller.keyDown(key("keydown", { key: "`", code: "Backquote", altKey: true }));

    const typed = key("keydown", { key: "`", code: "Backquote" });

    input.dispatchEvent(typed);
    controller.keyDown(typed);
    await controller.toggleDeafen();
    controller.keyDown(key("keydown", { key: "`", code: "Backquote" }));
    await Promise.resolve();

    expect(callStore.getState().snapshot.microphoneEnabled).toBe(false);
    input.remove();
    await controller.leave();
  });

  it("a microphone opened by hand stays open after a push-to-talk hold", async () => {
    pushToTalk(true);

    const { controller } = harness();

    await controller.join(ROOM, "Lounge", null);
    await controller.toggleMute();
    expect(callStore.getState().snapshot.microphoneEnabled).toBe(true);

    controller.keyDown(key("keydown", { key: "`", code: "Backquote" }));
    controller.keyUp(key("keyup", { key: "`", code: "Backquote" }));
    await Promise.resolve();

    expect(callStore.getState().snapshot.microphoneEnabled).toBe(true);
    await controller.leave();
  });
});

describe("call state", () => {
  it("a toggle cut off by a reconnect doesn't leave its control busy", async () => {
    const { controller, transports } = harness();

    await controller.join(ROOM, "Lounge", null);

    const [first] = transports;

    if (first === undefined) {
      throw new Error("no transport");
    }

    let release: () => void = () => undefined;

    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });

    first.setCamera = async () => {
      await gate;
    };

    const toggling = controller.toggleCamera();

    expect(callStore.getState().busy.camera).toBe(true);

    await controller.reconnectNow();

    expect(callStore.getState().phase).toBe("connected");
    expect(callStore.getState().busy.camera).toBe(false);

    release();
    await toggling;

    expect(callStore.getState().busy.camera).toBe(false);
    await controller.leave();
  });

  it("switching calls reports leaving the old one", async () => {
    const { controller, calls } = harness();

    await controller.join(ROOM, "Lounge", null);
    await controller.join(OTHER_ROOM, "Other", null);

    expect(calls.indexOf(`leave ${ROOM}`)).toBeGreaterThan(-1);
    expect(calls.indexOf(`leave ${ROOM}`)).toBeLessThan(calls.indexOf(`join ${OTHER_ROOM}`));
    await controller.leave();
  });

  it("a page closed mid-connect still reports the leave", async () => {
    let release: () => void = () => undefined;

    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });

    const { controller, calls } = harness({
      join: async (roomId) => {
        await gate;

        return credentials(roomId);
      },
    });

    const joining = controller.join(ROOM, "Lounge", null);

    await vi.waitFor(() => expect(callStore.getState().phase).toBe("connecting"));
    controller.endForPageChange();
    release();
    await joining;

    expect(calls).toContain(`leave-on-unload ${ROOM}`);
    expect(callStore.getState().phase).toBe("idle");
  });

  it("keeps the people muted for me in the store", async () => {
    mutations.setHuddlePresence({
      roomId: ROOM,
      participants: [{ userId: 3, membershipId: 30, identities: ["maya-1"], serverMuted: false }],
      live: false,
    });

    const { controller } = harness();

    await controller.join(ROOM, "Lounge", null);
    controller.toggleParticipantMute(3);
    expect(callStore.getState().localMutes).toEqual([3]);

    await controller.leave();
    await controller.join(ROOM, "Lounge", null);
    controller.presenceChanged();
    // Remembered for the next call.
    expect(callStore.getState().localMutes).toEqual([3]);

    controller.toggleParticipantMute(3);
    expect(callStore.getState().localMutes).toEqual([]);
    await controller.leave();
  });

  it("starts the call timer on connect and keeps it through a rejoin", async () => {
    const { controller } = harness();

    await controller.join(ROOM, "Lounge", null);

    const startedAt = callStore.getState().startedAt;

    expect(startedAt).not.toBeNull();

    await controller.roleChanged(ROOM, "speaker", false);
    expect(callStore.getState().startedAt).toBe(startedAt);

    await controller.leave();
    expect(callStore.getState().startedAt).toBeNull();
  });

  it("a camera picked while the microphone prompt is open keeps the microphone's outcome", async () => {
    let refuse: () => void = () => undefined;

    const { controller } = harness({
      shouldCheckDevices: async () => true,
      getUserMedia: async (constraints) => {
        if (constraints.audio !== false) {
          await new Promise<void>((resolve) => {
            refuse = resolve;
          });
        }

        throw new Error("no device");
      },
    });

    void controller.join(ROOM, "Lounge", null);
    await vi.waitFor(() => expect(callStore.getState().phase).toBe("prejoin"));
    await settle();
    await controller.selectDevice("videoinput", "camera-1");
    refuse();

    await vi.waitFor(() =>
      expect(callStore.getState().prejoin?.error).toBe(
        "The microphone could not be started. Check your device and try again.",
      ),
    );
    expect(callStore.getState().prejoin?.retry).toBe(true);
    await controller.leave();
  });
});
