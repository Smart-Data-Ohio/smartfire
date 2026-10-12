import * as LiveKit from "livekit-client";
import { afterEach, expect, it, vi } from "vitest";
import { liveKitTransport } from "./livekit.ts";
import type { StatEntry } from "./stats.ts";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function report(bytes: number): RTCStatsReport {
  return new Map<string, StatEntry>([
    [
      "video",
      {
        id: "video",
        type: "outbound-rtp",
        kind: "video",
        ssrc: 42,
        bytesSent: bytes,
        frameWidth: 1280,
        frameHeight: 720,
        framesPerSecond: 30,
        codecId: "codec",
        rid: "f",
      },
    ],
    ["codec", { type: "codec", mimeType: "video/VP8" }],
  ]);
}

it("ignores pending samples from before a fresh read without changing its bitrate baseline", async () => {
  class MediaStreamStub extends EventTarget {
    readonly tracks: MediaStreamTrack[];
    constructor(tracks: MediaStreamTrack[] = []) {
      super();
      this.tracks = tracks;
    }
    getTracks() {
      return this.tracks;
    }
    getVideoTracks() {
      return this.tracks.filter((track) => track.kind === "video");
    }
    getAudioTracks() {
      return this.tracks.filter((track) => track.kind === "audio");
    }
  }
  vi.stubGlobal("MediaStream", MediaStreamStub);
  const rooms: LiveKit.Room[] = [];
  class OfflineRoom extends LiveKit.Room {
    constructor(options?: Partial<LiveKit.RoomOptions>) {
      super(options);
      this.connect = () => Promise.resolve();
      this.disconnect = () => Promise.resolve();
      rooms.push(this);
    }
  }
  const media = Object.assign(new EventTarget(), {
    id: "camera",
    kind: "video",
    enabled: true,
    contentHint: "",
    readyState: "live",
    getSettings: () => ({ width: 1280, height: 720 }),
    getConstraints: () => ({}),
    stop: vi.fn(),
  });
  const track: unknown = Reflect.construct(LiveKit.LocalVideoTrack, [media]);
  if (!(track instanceof LiveKit.LocalVideoTrack)) throw new Error("SDK track missing");
  const publication: unknown = Reflect.construct(LiveKit.LocalTrackPublication, [
    LiveKit.Track.Kind.Video,
    { sid: "camera", name: "camera", source: 2, muted: false, width: 1280, height: 720 },
    track,
  ]);
  if (!(publication instanceof LiveKit.LocalTrackPublication))
    throw new Error("SDK publication missing");
  const transport = liveKitTransport({ ...LiveKit, Room: OfflineRoom });
  await transport.connect("wss://livekit.test", "token", false, 48_000);
  const room = rooms[0];
  if (room === undefined) throw new Error("SDK room missing");
  room.localParticipant.trackPublications.set("camera", publication);
  room.localParticipant.videoTrackPublications.set("camera", publication);
  const old = Promise.withResolvers<RTCStatsReport | undefined>();
  const fresh = Promise.withResolvers<RTCStatsReport | undefined>();
  const read = vi
    .spyOn(track, "getRTCStatsReport")
    .mockImplementationOnce(() => old.promise)
    .mockImplementationOnce(() => fresh.promise)
    .mockResolvedValueOnce(report(21_000));
  const now = vi.spyOn(Date, "now").mockReturnValue(1000);
  try {
    const beforeClose = transport.sampleStats(false);
    const reopened = transport.sampleStats(true);
    now.mockReturnValue(2000);
    fresh.resolve(report(20_000));
    const initial = await reopened;
    expect(initial?.videoTracks[0]?.layers[0]?.bitrateBps).toBeNull();
    expect(initial?.videoTracks[0]?.layers[0]?.codec).toBe("video/VP8");
    old.resolve(report(1000));
    expect(await beforeClose).toBeNull();
    now.mockReturnValue(3000);
    const next = await transport.sampleStats(false);
    expect(next?.videoTracks[0]?.layers[0]?.bitrateBps).toBe(8000);
    expect(next?.txBps).toBe(8000);
    expect(read).toHaveBeenCalledTimes(3);
  } finally {
    await transport.disconnect();
  }
});
