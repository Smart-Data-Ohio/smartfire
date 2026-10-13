import * as LiveKit from "livekit-client";
import { afterEach, expect, it, vi } from "vitest";
import { liveKitTransport } from "./livekit.ts";
import {
  installVideoMedia,
  offlineLiveKit,
  videoPublication,
  videoSender,
  videoTrack,
} from "./sdk-testing.ts";
import type { StatEntry } from "./stats.ts";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it("samples actual primary and backup video senders only when connection details asks", async () => {
  installVideoMedia();
  const { lk, rooms } = offlineLiveKit();
  const transport = liveKitTransport(lk);
  await transport.connect("wss://livekit.test", "token", false, 48_000);
  const room = rooms[0];

  if (room === undefined) throw new Error("SDK room missing");
  const track = videoTrack();
  const publication = videoPublication(track);
  room.localParticipant.trackPublications.set(publication.trackSid, publication);
  room.localParticipant.videoTrackPublications.set(publication.trackSid, publication);

  const primary = vi.spyOn(track, "getRTCStatsReport").mockImplementation(async () => {
    const sample = report(10_000);
    sample.set("codec", { type: "codec", mimeType: "video/AV1" });

    return sample;
  });

  const backup = videoSender(track.mediaStreamTrack.clone());

  const sampleBackup = vi.spyOn(backup, "getStats").mockImplementation(
    async () =>
      new Map([
        [
          "backup",
          {
            id: "backup",
            type: "outbound-rtp",
            kind: "video",
            ssrc: 51,
            bytesSent: 1000,
            codecId: "backup-codec",
            frameWidth: 640,
            frameHeight: 360,
          },
        ],
        ["backup-codec", { type: "codec", mimeType: "video/VP8" }],
      ]),
  );

  track.simulcastCodecs.set("vp8", {
    codec: "vp8",
    mediaStreamTrack: track.mediaStreamTrack.clone(),
    sender: backup,
  });

  try {
    transport.snapshot();
    expect(primary).not.toHaveBeenCalled();
    expect(sampleBackup).not.toHaveBeenCalled();
    const stats = await transport.sampleStats(true);
    expect(stats?.videoTracks[0]?.layers.map((layer) => layer.codec).sort()).toEqual([
      "video/AV1",
      "video/VP8",
    ]);
    expect(primary).toHaveBeenCalledTimes(1);
    expect(sampleBackup).toHaveBeenCalledTimes(1);
    expect(stats?.videoTracks).toHaveLength(1);
  } finally {
    await transport.disconnect();
  }
});

function report(bytes: number): Map<string, StatEntry> {
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
    {
      sid: "camera",
      name: "camera",
      source: LiveKit.Track.sourceToProto(LiveKit.Track.Source.Camera),
      muted: false,
      width: 1280,
      height: 720,
    },
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
