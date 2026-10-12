import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HuddleSoundboardPlayed } from "../../gen/HuddleSoundboardPlayed.ts";
import type { SoundboardCatalog } from "../../gen/SoundboardCatalog.ts";
import type { SoundboardSound } from "../../gen/SoundboardSound.ts";
import { soundboard } from "../../sync/huddles.ts";
import { ActionError } from "../../sync/run.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { callStore, initialCallState } from "./call-store.ts";
import {
  loadSoundboardEnabled,
  loadSoundboardVolume,
  SOUNDBOARD_ENABLED_STORAGE_KEY,
  SOUNDBOARD_VOLUME_STORAGE_KEY,
  storeSoundboardEnabled,
  storeSoundboardVolume,
} from "./engine/preferences.ts";
import {
  clipVolume,
  createSoundboardPlayer,
  MAX_OVERLAPPING,
  type PlayContext,
  PlayLimiter,
  playToCall,
  resetPlayCooldown,
  SENDER_INTERVAL_MS,
  STALE_AFTER_MS,
  screenPlay,
} from "./soundboard-player.ts";
import { resetSoundboard, setSoundboardEnabled, setSoundboardVolume } from "./soundboard-store.ts";

const PLAYED_AT = "2026-10-11T12:00:00.000Z";

const NOW = Date.parse(PLAYED_AT) + 500;

function play(overrides: Partial<HuddleSoundboardPlayed> = {}): HuddleSoundboardPlayed {
  return {
    roomId: 8,
    soundId: 1,
    userId: 3,
    identity: "maya-tab",
    playedAt: PLAYED_AT,
    ...overrides,
  };
}

function context(overrides: Partial<PlayContext> = {}): PlayContext {
  return {
    callRoomId: 8,
    identities: new Set(["me-tab", "maya-tab", "jonah-tab"]),
    userFor: (identity) => ({ "me-tab": 1, "maya-tab": 3, "jonah-tab": 4 })[identity] ?? null,
    ignored: () => false,
    enabled: true,
    deafened: false,
    serverNow: NOW,
    ...overrides,
  };
}

const AIRHORN: SoundboardSound = {
  id: 1,
  name: "Airhorn",
  emoji: "📯",
  volume: 0.5,
  uploaderId: 1,
  audioUrl: "/blobs/airhorn.wav",
  createdAt: PLAYED_AT,
  updatedAt: PLAYED_AT,
};

/** A player over a catalog held in `held`, recording what it plays. */
function harness(options: { held?: SoundboardSound[]; context?: Partial<PlayContext> } = {}) {
  let held = options.held ?? [AIRHORN];
  let clock = 0;
  const played: { roomId: number; url: string; volume: number }[] = [];
  const endings: (() => void)[] = [];

  const refresh = vi.fn(async (): Promise<SoundboardCatalog> => {
    held = [...held, { ...AIRHORN, id: 2, name: "Applause", audioUrl: "/blobs/applause.wav" }];

    return { sounds: held, capacity: 48 };
  });

  let current = context(options.context);

  const player = createSoundboardPlayer({
    context: () => current,
    sound: (soundId) => held.find((sound) => sound.id === soundId),
    refresh,
    volume: () => 50,
    play: (roomId, url, volume) => {
      played.push({ roomId, url, volume });

      return new Promise<void>((resolve) => {
        endings.push(resolve);
      });
    },
    now: () => clock,
  });

  return {
    player,
    played,
    refresh,
    endAll: async () => {
      for (const end of endings.splice(0)) end();
      await Promise.resolve();
      await Promise.resolve();
    },
    tick: (ms: number) => {
      clock += ms;
    },
    setContext: (next: Partial<PlayContext>) => {
      current = context(next);
    },
  };
}

describe("soundboard volume", () => {
  it("multiplies the sound's default by this browser's volume", () => {
    expect(clipVolume(1, 100)).toBe(1);
    expect(clipVolume(0.5, 50)).toBe(0.25);
    expect(clipVolume(0.8, 0)).toBe(0);
  });

  it("clamps both and treats a missing number as full", () => {
    expect(clipVolume(3, 250)).toBe(1);
    expect(clipVolume(-1, 50)).toBe(0);
    expect(clipVolume(Number.NaN, 50)).toBe(0.5);
    expect(clipVolume(0.5, Number.POSITIVE_INFINITY)).toBe(0.5);
  });
});

describe("the play limiter", () => {
  it("spaces each person's plays a second apart", () => {
    const limiter = new PlayLimiter();

    expect(limiter.admit(3, 0)).toBe(true);
    limiter.finished();
    expect(limiter.admit(3, SENDER_INTERVAL_MS - 1)).toBe(false);
    expect(limiter.admit(4, SENDER_INTERVAL_MS - 1)).toBe(true);
    limiter.finished();
    expect(limiter.admit(3, SENDER_INTERVAL_MS)).toBe(true);
  });

  it("caps the sounds playing at once, and frees a slot when one ends", () => {
    const limiter = new PlayLimiter();

    for (let sender = 1; sender <= MAX_OVERLAPPING; sender += 1) {
      expect(limiter.admit(sender, 0)).toBe(true);
    }

    expect(limiter.admit(99, 0)).toBe(false);
    expect(limiter.playing).toBe(MAX_OVERLAPPING);
    limiter.finished();
    expect(limiter.admit(99, 0)).toBe(true);
  });

  it("never counts below zero", () => {
    const limiter = new PlayLimiter();

    limiter.finished();
    expect(limiter.playing).toBe(0);
  });
});

describe("screening a relayed play", () => {
  it("passes a live, recent play from someone in this call", () => {
    expect(screenPlay(play(), true, context())).toBeNull();
  });

  it("skips a play replayed after a reconnect", () => {
    expect(screenPlay(play(), false, context())).toBe("replayed");
  });

  it("skips a play older than the staleness window by the server's clock", () => {
    const late = context({ serverNow: Date.parse(PLAYED_AT) + STALE_AFTER_MS + 1 });

    expect(screenPlay(play(), true, late)).toBe("stale");
    expect(screenPlay(play(), true, context({ serverNow: null }))).toBeNull();
  });

  it("skips a play for another call, or with no call", () => {
    expect(screenPlay(play({ roomId: 9 }), true, context())).toBe("other-call");
    expect(screenPlay(play(), true, context({ callRoomId: null }))).toBe("other-call");
  });

  it("skips every play with sounds turned off, or deafened", () => {
    expect(screenPlay(play(), true, context({ enabled: false }))).toBe("off");
    expect(screenPlay(play(), true, context({ deafened: true }))).toBe("deafened");
  });

  it("skips a sender who isn't in this call, or whose tab belongs to someone else", () => {
    expect(screenPlay(play({ identity: "gone-tab" }), true, context())).toBe("not-in-call");
    expect(screenPlay(play({ userId: 4 }), true, context())).toBe("not-in-call");
  });

  it("skips someone the viewer ignores", () => {
    const ignoring = context({ ignored: (userId) => userId === 3 });

    expect(screenPlay(play(), true, ignoring)).toBe("ignored");
    expect(screenPlay(play({ userId: 4, identity: "jonah-tab" }), true, ignoring)).toBeNull();
  });
});

describe("the soundboard player", () => {
  it("plays a known sound at its volume times this browser's", async () => {
    const { player, played } = harness();

    expect(await player.received(play(), true)).toBeNull();
    expect(played).toEqual([{ roomId: 8, url: "/blobs/airhorn.wav", volume: 0.25 }]);
  });

  it("refetches the catalog once for an unknown sound, then plays it", async () => {
    const { player, played, refresh, tick } = harness();

    expect(await player.received(play({ soundId: 2 }), true)).toBeNull();
    expect(refresh).toHaveBeenCalledTimes(1);
    expect(played.at(-1)?.url).toBe("/blobs/applause.wav");

    tick(SENDER_INTERVAL_MS);
    expect(await player.received(play({ soundId: 7 }), true)).toBe("unknown-sound");
    expect(await player.received(play({ soundId: 7 }), true)).toBe("unknown-sound");
    expect(refresh).toHaveBeenCalledTimes(2);
  });

  it("drops an ignored sender's play without looking the sound up", async () => {
    const { player, played, refresh } = harness({
      context: { ignored: (userId) => userId === 3 },
    });

    expect(await player.received(play({ soundId: 2 }), true)).toBe("ignored");
    expect(refresh).not.toHaveBeenCalled();
    expect(played).toEqual([]);
  });

  it("limits one person to a play a second and three at once", async () => {
    const { player, played, tick, endAll } = harness();

    expect(await player.received(play(), true)).toBeNull();
    expect(await player.received(play(), true)).toBe("limited");

    expect(await player.received(play({ userId: 4, identity: "jonah-tab" }), true)).toBeNull();
    expect(await player.received(play({ userId: 1, identity: "me-tab" }), true)).toBeNull();
    tick(SENDER_INTERVAL_MS);
    expect(await player.received(play(), true)).toBe("limited");
    expect(played).toHaveLength(3);

    await endAll();
    expect(await player.received(play(), true)).toBeNull();
  });

  it("asks again for a sound whose refetch failed, rather than giving up on it", async () => {
    const { player, played, refresh, tick } = harness();

    refresh.mockRejectedValueOnce(new Error("offline"));

    expect(await player.received(play({ soundId: 2 }), true)).toBe("unknown-sound");
    tick(SENDER_INTERVAL_MS);
    expect(await player.received(play({ soundId: 2 }), true)).toBeNull();
    expect(refresh).toHaveBeenCalledTimes(2);
    expect(played.at(-1)?.url).toBe("/blobs/applause.wav");
  });

  it("rechecks after a refetch: the viewer may have turned sounds off meanwhile", async () => {
    const { player, played, refresh, setContext } = harness();

    refresh.mockImplementationOnce(async () => {
      setContext({ enabled: false });

      return { sounds: [AIRHORN], capacity: 48 };
    });

    expect(await player.received(play({ soundId: 2 }), true)).toBe("unknown-sound");
    expect(played).toEqual([]);
  });
});

describe("soundboard preferences", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("keep the volume 0–100, full by default", () => {
    expect(loadSoundboardVolume()).toBe(100);
    storeSoundboardVolume(35);
    expect(loadSoundboardVolume()).toBe(35);
    localStorage.setItem(SOUNDBOARD_VOLUME_STORAGE_KEY, "400");
    expect(loadSoundboardVolume()).toBe(100);
    localStorage.setItem(SOUNDBOARD_VOLUME_STORAGE_KEY, "-5");
    expect(loadSoundboardVolume()).toBe(0);
    localStorage.setItem(SOUNDBOARD_VOLUME_STORAGE_KEY, "loud");
    expect(loadSoundboardVolume()).toBe(100);
  });

  it("keep sounds on unless turned off, and forget the switch when back on", () => {
    expect(loadSoundboardEnabled()).toBe(true);
    storeSoundboardEnabled(false);
    expect(localStorage.getItem(SOUNDBOARD_ENABLED_STORAGE_KEY)).toBe("off");
    expect(loadSoundboardEnabled()).toBe(false);
    storeSoundboardEnabled(true);
    expect(loadSoundboardEnabled()).toBe(true);
    expect(localStorage.length).toBe(0);
  });

  it("write through from the store", () => {
    setSoundboardVolume(42.4);
    setSoundboardEnabled(false);
    resetSoundboard();

    expect(loadSoundboardVolume()).toBe(42);
    expect(loadSoundboardEnabled()).toBe(false);
  });
});

describe("playing to the call", () => {
  beforeEach(() => {
    localStorage.clear();
    resetSoundboard();
    resetPlayCooldown();
    callStore.setState(
      { ...initialCallState, phase: "connected", roomId: 8, identity: "me-tab", canPublish: true },
      true,
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
    callStore.setState(initialCallState, true);

    for (const each of toastSnapshot()) removeToast(each.id);
  });

  it("asks the server to relay the play, once a second", () => {
    const relay = vi.spyOn(soundboard, "play").mockResolvedValue(undefined);

    expect(playToCall(1)).toBe(true);
    expect(playToCall(2)).toBe(false);
    expect(relay).toHaveBeenCalledExactlyOnceWith(8, "me-tab", 1);
  });

  it("sends nothing with sounds off, outside a live call, or unable to speak", () => {
    const relay = vi.spyOn(soundboard, "play").mockResolvedValue(undefined);

    setSoundboardEnabled(false);
    expect(playToCall(1)).toBe(false);
    setSoundboardEnabled(true);

    callStore.setState({ canPublish: false });
    expect(playToCall(1)).toBe(false);

    callStore.setState({ canPublish: true, phase: "connecting" });
    expect(playToCall(1)).toBe(false);
    expect(relay).not.toHaveBeenCalled();
  });

  it("says when the server refuses a play too soon", async () => {
    vi.spyOn(soundboard, "play").mockRejectedValue(
      new ActionError("RateLimited", "Slow down and try again"),
    );

    playToCall(1);

    await vi.waitFor(() =>
      expect(toastSnapshot().map((each) => each.title)).toEqual(["Too many sounds"]),
    );
  });
});
