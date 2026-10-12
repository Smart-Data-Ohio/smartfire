import { describe, expect, it } from "vitest";
import {
  formatVideoTrackStats,
  type StatEntry,
  type StatsReport,
  summarizeVideoStats,
  type VideoStatsInput,
} from "./stats.ts";

function report(entries: readonly StatEntry[]): StatsReport {
  return new Map(entries.map((entry, index) => [entry.id ?? `entry-${index}`, entry]));
}

function sending(stats: StatsReport | null): VideoStatsInput {
  return {
    participantId: "alice-tab",
    participantName: "Alice",
    trackSid: "camera-a",
    direction: "sending",
    source: "camera",
    report: stats,
  };
}

function receiving(stats: StatsReport | null): VideoStatsInput {
  return {
    participantId: "bob-tab",
    participantName: "Bob",
    trackSid: "screen-b",
    direction: "receiving",
    source: "screen",
    report: stats,
  };
}

const codec = { id: "codec", type: "codec", mimeType: "video/VP9" };

function outbound(bytes: number, frames: number, id = "high"): StatEntry {
  return { id, type: "outbound-rtp", kind: "video", bytesSent: bytes, framesEncoded: frames };
}

function inbound(bytes: number, frames: number): StatEntry {
  return {
    id: "received",
    type: "inbound-rtp",
    kind: "video",
    bytesReceived: bytes,
    framesDecoded: frames,
  };
}

describe("video track stats", () => {
  it("reports actual dimensions and rates for each sending layer and receiving track", () => {
    const sample = summarizeVideoStats(
      [
        sending(
          report([
            codec,
            {
              ...outbound(10_000, 30, "low"),
              ssrc: 11,
              frameWidth: 320,
              frameHeight: 180,
              framesPerSecond: 15,
              codecId: "codec",
              rid: "q",
              qualityLimitationReason: "bandwidth",
            },
            {
              ...outbound(80_000, 60),
              ssrc: 12,
              frameWidth: 1920,
              frameHeight: 1080,
              framesPerSecond: 30,
              codecId: "codec",
              rid: "f",
              scalabilityMode: "L1T3",
              qualityLimitationReason: "none",
            },
          ]),
        ),
        receiving(
          report([
            codec,
            {
              ...inbound(8_000, 30),
              ssrc: 21,
              frameWidth: 640,
              frameHeight: 360,
              framesPerSecond: 24,
              codecId: "codec",
              scalabilityMode: "L3T3_KEY",
              freezeCount: 2,
            },
          ]),
        ),
      ],
      null,
      1000,
    );

    expect(sample.videoTracks).toEqual([
      {
        participantId: "alice-tab",
        participantName: "Alice",
        trackSid: "camera-a",
        direction: "sending",
        source: "camera",
        layers: [
          {
            id: "low",
            ssrc: 11,
            frameWidth: 320,
            frameHeight: 180,
            framesPerSecond: 15,
            codec: "video/VP9",
            rid: "q",
            scalabilityMode: null,
            bitrateBps: null,
            freezeCount: null,
            qualityLimitationReason: "bandwidth",
          },
          {
            id: "high",
            ssrc: 12,
            frameWidth: 1920,
            frameHeight: 1080,
            framesPerSecond: 30,
            codec: "video/VP9",
            rid: "f",
            scalabilityMode: "L1T3",
            bitrateBps: null,
            freezeCount: null,
            qualityLimitationReason: "none",
          },
        ],
      },
      {
        participantId: "bob-tab",
        participantName: "Bob",
        trackSid: "screen-b",
        direction: "receiving",
        source: "screen",
        layers: [
          {
            id: "received",
            ssrc: 21,
            frameWidth: 640,
            frameHeight: 360,
            framesPerSecond: 24,
            codec: "video/VP9",
            rid: null,
            scalabilityMode: "L3T3_KEY",
            bitrateBps: null,
            freezeCount: 2,
            qualityLimitationReason: null,
          },
        ],
      },
    ]);
    expect(sample.baseline.size).toBe(3);
  });

  it("measures byte and frame deltas independently for every RTP layer", () => {
    const first = summarizeVideoStats(
      [
        sending(report([outbound(10_000, 30, "low"), outbound(80_000, 60)])),
        receiving(report([inbound(8_000, 30)])),
      ],
      null,
      1000,
    );
    const next = summarizeVideoStats(
      [
        sending(report([outbound(12_000, 50, "low"), outbound(100_000, 120)])),
        receiving(report([inbound(12_000, 78)])),
      ],
      first.baseline,
      3000,
    );

    expect(
      next.videoTracks.flatMap((track) =>
        track.layers.map((layer) => ({
          id: layer.id,
          bitrateBps: layer.bitrateBps,
          framesPerSecond: layer.framesPerSecond,
        })),
      ),
    ).toEqual([
      { id: "low", bitrateBps: 8_000, framesPerSecond: 10 },
      { id: "high", bitrateBps: 80_000, framesPerSecond: 30 },
      { id: "received", bitrateBps: 16_000, framesPerSecond: 24 },
    ]);
  });

  it("prefers the reported FPS, including zero, over a frame-counter fallback", () => {
    const first = summarizeVideoStats([sending(report([outbound(1_000, 30)]))], null, 1000);
    const next = summarizeVideoStats(
      [sending(report([{ ...outbound(2_000, 90), framesPerSecond: 0 }]))],
      first.baseline,
      2000,
    );

    expect(next.videoTracks[0]?.layers[0]?.framesPerSecond).toBe(0);
  });

  it("isolates the same RTP id by participant, direction, track and SSRC", () => {
    const first = summarizeVideoStats(
      [sending(report([{ ...outbound(1_000, 30), ssrc: 10 }]))],
      null,
      1000,
    );
    const changed = summarizeVideoStats(
      [
        { ...sending(report([outbound(10_000, 60)])), participantId: "another-tab" },
        { ...sending(report([outbound(10_000, 60)])), trackSid: "new-camera" },
        sending(report([{ ...outbound(10_000, 60), ssrc: 11 }])),
        {
          ...sending(report([{ ...inbound(10_000, 60), id: "high", ssrc: 10 }])),
          direction: "receiving",
        },
      ],
      first.baseline,
      2000,
    );

    expect(
      changed.videoTracks.flatMap((track) => track.layers.map((layer) => layer.bitrateBps)),
    ).toEqual([null, null, null, null]);
  });

  it("drops failed and empty reports so their recovery starts with unknown rates", () => {
    const first = summarizeVideoStats([sending(report([outbound(1_000, 30)]))], null, 1000);
    const failed = summarizeVideoStats([sending(null)], first.baseline, 2000);
    const recovered = summarizeVideoStats(
      [sending(report([outbound(9_000, 100)]))],
      failed.baseline,
      3000,
    );
    const empty = summarizeVideoStats([sending(report([]))], recovered.baseline, 4000);

    expect(failed.videoTracks[0]?.layers).toEqual([]);
    expect(failed.baseline.size).toBe(0);
    expect(recovered.videoTracks[0]?.layers[0]?.bitrateBps).toBeNull();
    expect(recovered.videoTracks[0]?.layers[0]?.framesPerSecond).toBeNull();
    expect(empty.videoTracks[0]?.layers).toEqual([]);
    expect(empty.baseline.size).toBe(0);
  });

  it("prunes removed tracks and layers and does not spike when they return", () => {
    const first = summarizeVideoStats(
      [
        sending(report([outbound(1_000, 30, "low"), outbound(10_000, 60)])),
        receiving(report([inbound(1_000, 30)])),
      ],
      null,
      1000,
    );
    const reduced = summarizeVideoStats(
      [sending(report([outbound(2_000, 60, "low")]))],
      first.baseline,
      2000,
    );
    const returned = summarizeVideoStats(
      [
        sending(report([outbound(3_000, 90, "low"), outbound(50_000, 200)])),
        receiving(report([inbound(100_000, 300)])),
      ],
      reduced.baseline,
      3000,
    );
    const removed = summarizeVideoStats([], returned.baseline, 4000);

    expect(reduced.baseline.size).toBe(1);
    expect(
      returned.videoTracks.flatMap((track) => track.layers.map((layer) => layer.bitrateBps)),
    ).toEqual([8_000, null, null]);
    expect(removed.videoTracks).toEqual([]);
    expect(removed.baseline.size).toBe(0);
  });

  it("treats counter resets, repeated times and time reversal as unknown rates", () => {
    const first = summarizeVideoStats([sending(report([outbound(10_000, 60)]))], null, 2000);
    const reset = summarizeVideoStats([sending(report([outbound(100, 1)]))], first.baseline, 3000);
    const later = summarizeVideoStats(
      [sending(report([outbound(1_100, 31)]))],
      reset.baseline,
      4000,
    );
    const repeated = summarizeVideoStats(
      [sending(report([outbound(2_100, 61)]))],
      later.baseline,
      4000,
    );
    const reversed = summarizeVideoStats(
      [sending(report([outbound(3_100, 91)]))],
      repeated.baseline,
      3500,
    );

    expect(reset.videoTracks[0]?.layers[0]?.bitrateBps).toBeNull();
    expect(reset.videoTracks[0]?.layers[0]?.framesPerSecond).toBeNull();
    expect(later.videoTracks[0]?.layers[0]?.bitrateBps).toBe(8_000);
    expect(later.videoTracks[0]?.layers[0]?.framesPerSecond).toBe(30);
    expect(repeated.videoTracks[0]?.layers[0]?.bitrateBps).toBeNull();
    expect(repeated.videoTracks[0]?.layers[0]?.framesPerSecond).toBeNull();
    expect(reversed.videoTracks[0]?.layers[0]?.bitrateBps).toBeNull();
    expect(reversed.videoTracks[0]?.layers[0]?.framesPerSecond).toBeNull();
  });

  it("does not expose audio, remote feedback, codec or repair entries as video layers", () => {
    const sample = summarizeVideoStats(
      [
        sending(
          report([
            codec,
            { id: "repair-codec", type: "codec", mimeType: "video/RTX" },
            { id: "red-codec", type: "codec", mimeType: "video/red" },
            { ...outbound(1_000, 30), codecId: "codec" },
            { ...outbound(1_000, 30, "repair"), codecId: "repair-codec" },
            { ...outbound(1_000, 30, "red"), codecId: "red-codec" },
            { id: "audio", type: "outbound-rtp", kind: "audio", bytesSent: 1_000 },
            { id: "remote", type: "outbound-rtp", kind: "video", isRemote: true },
            { id: "feedback", type: "remote-inbound-rtp", kind: "video", bytesReceived: 1_000 },
            inbound(1_000, 30),
          ]),
        ),
      ],
      null,
      1000,
    );

    expect(sample.videoTracks[0]?.layers.map((layer) => layer.id)).toEqual(["high"]);
    expect(sample.baseline.size).toBe(1);
  });

  it("supports codec-based video classification and an SSRC when RTP id is absent", () => {
    const first = summarizeVideoStats(
      [
        receiving(
          report([
            codec,
            { type: "inbound-rtp", ssrc: 42, codecId: "codec", bytesReceived: 1_000 },
          ]),
        ),
      ],
      null,
      1000,
    );
    const next = summarizeVideoStats(
      [
        receiving(
          report([
            codec,
            { type: "inbound-rtp", ssrc: 42, codecId: "codec", bytesReceived: 2_000 },
          ]),
        ),
      ],
      first.baseline,
      2000,
    );

    expect(next.videoTracks[0]?.layers[0]?.id).toBe("ssrc-42");
    expect(next.videoTracks[0]?.layers[0]?.bitrateBps).toBe(8_000);
  });

  it("keeps unsupported and invalid measurements unknown", () => {
    const sample = summarizeVideoStats(
      [
        receiving(
          report([
            { id: "unknown", type: "inbound-rtp", kind: "video" },
            {
              id: "invalid",
              type: "inbound-rtp",
              kind: "video",
              frameWidth: 0,
              frameHeight: -1,
              framesPerSecond: Number.NaN,
              bytesReceived: -10,
              framesDecoded: Number.POSITIVE_INFINITY,
              freezeCount: -1,
            },
          ]),
        ),
      ],
      null,
      1000,
    );

    expect(sample.videoTracks[0]?.layers).toEqual([
      {
        id: "unknown",
        ssrc: null,
        frameWidth: null,
        frameHeight: null,
        framesPerSecond: null,
        codec: null,
        rid: null,
        scalabilityMode: null,
        bitrateBps: null,
        freezeCount: null,
        qualityLimitationReason: null,
      },
      {
        id: "invalid",
        ssrc: null,
        frameWidth: null,
        frameHeight: null,
        framesPerSecond: null,
        codec: null,
        rid: null,
        scalabilityMode: null,
        bitrateBps: null,
        freezeCount: null,
        qualityLimitationReason: null,
      },
    ]);
  });
});

describe("video track stats text", () => {
  it("labels actual measurements, RID and scalability mode separately", () => {
    const first = summarizeVideoStats([receiving(report([inbound(1_000, 30)]))], null, 1000);
    const next = summarizeVideoStats(
      [
        receiving(
          report([
            codec,
            {
              ...inbound(11_000, 60),
              frameWidth: 640,
              frameHeight: 360,
              codecId: "codec",
              scalabilityMode: "L3T3_KEY",
              freezeCount: 0,
            },
          ]),
        ),
      ],
      first.baseline,
      2000,
    );

    expect(formatVideoTrackStats(next.videoTracks)).toEqual([
      {
        key: JSON.stringify([
          JSON.stringify(["bob-tab", "receiving", "screen-b"]),
          "received",
          null,
        ]),
        label: "Bob (bob-tab) · receiving screen · screen-b · received",
        value:
          "Resolution 640×360 · FPS 30 · Codec video/VP9 · RID – · Mode L3T3_KEY · Bitrate 80 kbps · Freezes 0 · Limit –",
      },
    ]);
  });

  it("keeps a failed track visible with dashes", () => {
    const sample = summarizeVideoStats([sending(null)], null, 1000);

    expect(formatVideoTrackStats(sample.videoTracks)).toEqual([
      {
        key: JSON.stringify(["alice-tab", "sending", "camera-a"]),
        label: "Alice (alice-tab) · sending camera · camera-a",
        value: "Resolution – · FPS – · Codec – · RID – · Mode – · Bitrate – · Freezes – · Limit –",
      },
    ]);
    expect(formatVideoTrackStats([])).toEqual([]);
  });
});
