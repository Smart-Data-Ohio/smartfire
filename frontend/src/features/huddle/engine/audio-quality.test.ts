import * as LiveKit from "livekit-client";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  DEFAULT_AUDIO_BITRATE,
  microphonePreset,
  microphonePublishOptions,
} from "./audio-quality.ts";
import { liveKitTransport } from "./livekit.ts";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("the microphone's encoding", () => {
  it("targets the grant's bitrate", () => {
    expect(microphonePublishOptions(384_000)).toEqual({ audioPreset: { maxBitrate: 384_000 } });
    expect(microphonePreset(96_000)).toEqual({ maxBitrate: 96_000 });
  });

  it("falls back to the SDK's music preset without a usable target", () => {
    expect(DEFAULT_AUDIO_BITRATE).toBe(LiveKit.AudioPresets.music.maxBitrate);

    for (const target of [undefined, null, 0, -64_000, 48_000.5, Number.NaN]) {
      expect(microphonePreset(target)).toEqual({ maxBitrate: 48_000 });
    }
  });
});

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

describe("publishing the microphone", () => {
  it("goes through the SDK's helper with the grant's target, over the room defaults", async () => {
    const enable = vi
      .spyOn(LiveKit.LocalParticipant.prototype, "setMicrophoneEnabled")
      .mockResolvedValue(undefined);

    const { lk, rooms } = offlineLiveKit();
    const transport = liveKitTransport(lk);

    await transport.connect("wss://livekit.test", "token", false, 256_000);
    await transport.setMicrophone(true);
    await transport.setMicrophone(false);

    expect(enable.mock.calls.map(([enabled, , publish]) => [enabled, publish])).toEqual([
      [true, { audioPreset: { maxBitrate: 256_000 } }],
      [false, { audioPreset: { maxBitrate: 256_000 } }],
    ]);

    // Everything else, a screen share's audio included, keeps the defaults it had.
    expect(rooms[0]?.options.publishDefaults).toMatchObject({
      audioPreset: LiveKit.AudioPresets.music,
      dtx: true,
      red: true,
      stopMicTrackOnMute: true,
    });

    await transport.disconnect();

    // The next grant's target replaces it.
    await transport.connect("wss://livekit.test", "token", false, 64_000);
    await transport.setMicrophone(true);
    expect(enable.mock.lastCall?.[2]).toEqual({ audioPreset: { maxBitrate: 64_000 } });
    await transport.disconnect();
  });
});
