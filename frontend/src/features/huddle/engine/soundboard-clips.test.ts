import * as LiveKit from "livekit-client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { liveKitTransport } from "./livekit.ts";

/** The real SDK, its rooms joining nothing: `connect` resolves without a network. */
function offlineLiveKit() {
  const rooms: LiveKit.Room[] = [];

  class OfflineRoom extends LiveKit.Room {
    constructor(options?: Partial<LiveKit.RoomOptions>) {
      super(options);
      this.connect = () => Promise.resolve();
      this.disconnect = () => Promise.resolve();
      rooms.push(this);
    }
  }

  return { lk: { ...LiveKit, Room: OfflineRoom }, rooms };
}

/** Every clip the transport made, with its speaker still being set until `sinkSet()`. */
const clips: FakeClip[] = [];

class FakeClip extends EventTarget {
  volume = 1;
  playing = false;
  played = 0;
  sink = "";
  readonly src: string;
  #sinkSet: () => void = () => undefined;

  constructor(src: string) {
    super();
    this.src = src;
    clips.push(this);
  }

  setSinkId(sink: string): Promise<void> {
    this.sink = sink;

    return new Promise((resolve) => {
      this.#sinkSet = resolve;
    });
  }

  sinkSet(): void {
    this.#sinkSet();
  }

  play(): Promise<void> {
    this.playing = true;
    this.played += 1;

    return Promise.resolve();
  }

  pause(): void {
    if (this.playing) {
      this.playing = false;
      this.dispatchEvent(new Event("pause"));
    }
  }
}

/** Lets the transport's pending `await`s run. */
async function settle(): Promise<void> {
  for (let turn = 0; turn < 5; turn += 1) {
    await Promise.resolve();
  }
}

async function connected(speaker: string) {
  const { lk, rooms } = offlineLiveKit();
  const transport = liveKitTransport(lk);

  await transport.connect("wss://livekit.test", "token", false, 64_000);

  const room = rooms[0];

  if (room === undefined) throw new Error("no room");

  vi.spyOn(room, "getActiveDevice").mockReturnValue(speaker);

  return transport;
}

beforeEach(() => {
  clips.length = 0;
  vi.stubGlobal("Audio", FakeClip);
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("soundboard clips on the LiveKit transport", () => {
  it("play on the call's speaker at the given volume", async () => {
    const transport = await connected("speaker-2");

    void transport.playClip("/blobs/airhorn.wav", 0.4);

    const [clip] = clips;

    expect(clip?.sink).toBe("speaker-2");
    clip?.sinkSet();
    await settle();
    expect(clip).toMatchObject({ playing: true, volume: 0.4 });
    await transport.disconnect();
  });

  it("stop when the call is deafened, and don't come back on undeafening", async () => {
    const transport = await connected("default");
    const ended = vi.fn();

    void transport.playClip("/blobs/airhorn.wav", 1).then(ended);
    await settle();
    expect(clips[0]?.playing).toBe(true);

    transport.setDeafened(true);
    await settle();
    expect(clips[0]?.playing).toBe(false);
    expect(ended).toHaveBeenCalledOnce();

    transport.setDeafened(false);
    await settle();
    expect(clips[0]?.played).toBe(1);

    // While deafened a new clip doesn't start at all.
    transport.setDeafened(true);
    await transport.playClip("/blobs/rimshot.wav", 1);
    expect(clips).toHaveLength(1);
    await transport.disconnect();
  });

  it("never start once the call is left while the speaker was still being set", async () => {
    const transport = await connected("speaker-2");
    const ended = vi.fn();

    void transport.playClip("/blobs/airhorn.wav", 1).then(ended);
    await transport.disconnect();
    await settle();
    expect(ended).toHaveBeenCalledOnce();

    clips[0]?.sinkSet();
    await settle();
    expect(clips[0]?.played).toBe(0);
  });
});
