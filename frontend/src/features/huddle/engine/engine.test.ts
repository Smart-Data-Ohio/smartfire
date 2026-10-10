import { beforeEach, describe, expect, it } from "vitest";
import { parseFakeToken } from "./fake.ts";
import { scaleMeterVolume } from "./meter.ts";
import {
  DEVICE_STORAGE_KEY,
  loadDevicePreferences,
  loadNoiseSuppression,
  loadParticipantMuted,
  loadParticipantVolume,
  loadShareQuality,
  loadStreamQuality,
  PARTICIPANT_VOLUME_PREFIX,
  SHARE_QUALITY_STORAGE_KEY,
  STREAM_QUALITY_STORAGE_KEY,
  storeDevicePreference,
  storeNoiseSuppression,
  storeParticipantMuted,
  storeParticipantVolume,
  storeShareQuality,
} from "./preferences.ts";
import {
  formatConnectionStats,
  type StatEntry,
  type StatsReport,
  summarizeConnectionStats,
} from "./stats.ts";

function report(entries: readonly StatEntry[]): StatsReport {
  return new Map(entries.map((entry, index) => [entry.id ?? `entry-${index}`, entry]));
}

describe("connection stats", () => {
  const publisher = report([
    { type: "outbound-rtp", kind: "audio", bytesSent: 10_000, packetsSent: 990 },
    { type: "outbound-rtp", kind: "video", bytesSent: 40_000, packetsSent: 3000 },
    {
      type: "remote-inbound-rtp",
      kind: "video",
      roundTripTime: 0.2,
      jitter: 0.05,
      packetsLost: 300,
    },
    {
      type: "remote-inbound-rtp",
      kind: "audio",
      roundTripTime: 0.042,
      jitter: 0.0032,
      packetsLost: 10,
    },
    {
      id: "pair",
      type: "candidate-pair",
      nominated: true,
      localCandidateId: "local",
      remoteCandidateId: "remote",
    },
    { id: "local", type: "local-candidate", candidateType: "host" },
    { id: "remote", type: "remote-candidate", candidateType: "srflx" },
  ]);

  const subscriber = report([{ type: "inbound-rtp", bytesReceived: 8000 }]);

  it("reads the upstream path from audio feedback, against the audio packets sent", () => {
    const { stats } = summarizeConnectionStats(publisher, subscriber, null, 1000);

    expect(stats.rttMs).toBeCloseTo(42);
    expect(stats.jitterMs).toBeCloseTo(3.2);
    expect(stats.lossRatio).toBeCloseTo(0.01);
    expect(stats.relayed).toBe(false);
    // The first sample has nothing to measure a bitrate against.
    expect(stats.rxBps).toBeNull();
    expect(stats.txBps).toBeNull();
  });

  it("measures bitrates against the previous sample", () => {
    const first = summarizeConnectionStats(publisher, subscriber, null, 1000);

    const later = summarizeConnectionStats(
      report([{ type: "outbound-rtp", kind: "audio", bytesSent: 16_000 + 40_000, packetsSent: 1 }]),
      report([{ type: "inbound-rtp", bytesReceived: 16_000 }]),
      first.baseline,
      3000,
    );

    expect(later.stats.rxBps).toBe(32_000);
    expect(later.stats.txBps).toBe(24_000);
  });

  it("reads a missing report as no sample, not zero", () => {
    const first = summarizeConnectionStats(null, null, null, 1000);

    expect(first.baseline.txBytes).toBeNull();
    expect(first.stats.relayed).toBeNull();

    const next = summarizeConnectionStats(null, subscriber, first.baseline, 2000);

    expect(next.stats.rxBps).toBeNull();
  });

  it("notices a relayed path through the transport's selected pair", () => {
    const relayed = report([
      { type: "transport", selectedCandidatePairId: "pair" },
      {
        id: "pair",
        type: "candidate-pair",
        localCandidateId: "local",
        remoteCandidateId: "remote",
      },
      { id: "local", type: "local-candidate", candidateType: "relay" },
      { id: "remote", type: "remote-candidate", candidateType: "host" },
    ]);

    expect(summarizeConnectionStats(relayed, null, null, 0).stats.relayed).toBe(true);
  });

  it("formats each field, with a dash for what wasn't measured", () => {
    expect(
      formatConnectionStats({
        rttMs: 42.4,
        lossRatio: 0.004,
        jitterMs: 3.21,
        rxBps: 64_000,
        txBps: 2_500_000,
        relayed: true,
      }),
    ).toEqual({
      rtt: "42 ms",
      loss: "0.4%",
      jitter: "3.2 ms",
      received: "64 kbps",
      sent: "2.5 Mbps",
      transport: "Relayed (TURN)",
    });

    expect(
      formatConnectionStats({
        rttMs: null,
        lossRatio: null,
        jitterMs: null,
        rxBps: null,
        txBps: null,
        relayed: null,
      }),
    ).toEqual({ rtt: "–", loss: "–", jitter: "–", received: "–", sent: "–", transport: "–" });
  });
});

describe("preferences", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("remembers devices per kind under the classic key", () => {
    expect(loadDevicePreferences()).toEqual({ audioinput: "", audiooutput: "", videoinput: "" });

    storeDevicePreference("audioinput", "mic-2");
    storeDevicePreference("videoinput", "cam-1");

    expect(loadDevicePreferences()).toEqual({
      audioinput: "mic-2",
      audiooutput: "",
      videoinput: "cam-1",
    });
    expect(JSON.parse(localStorage.getItem(DEVICE_STORAGE_KEY) ?? "{}")).toEqual({
      audioinput: "mic-2",
      audiooutput: "",
      videoinput: "cam-1",
    });
  });

  it("reads a corrupt or mistyped device entry as no preference", () => {
    localStorage.setItem(DEVICE_STORAGE_KEY, "{not json");
    expect(loadDevicePreferences().audioinput).toBe("");

    localStorage.setItem(DEVICE_STORAGE_KEY, JSON.stringify({ audioinput: 4, videoinput: "cam" }));
    expect(loadDevicePreferences()).toEqual({ audioinput: "", audiooutput: "", videoinput: "cam" });
  });

  it("keeps noise suppression on unless turned off", () => {
    expect(loadNoiseSuppression()).toBe(true);
    storeNoiseSuppression(false);
    expect(loadNoiseSuppression()).toBe(false);
    storeNoiseSuppression(true);
    expect(loadNoiseSuppression()).toBe(true);
  });

  it("falls back to auto stream quality", () => {
    expect(loadStreamQuality()).toBe("auto");
    localStorage.setItem(STREAM_QUALITY_STORAGE_KEY, "ultra");
    expect(loadStreamQuality()).toBe("auto");
    localStorage.setItem(STREAM_QUALITY_STORAGE_KEY, "low");
    expect(loadStreamQuality()).toBe("low");
  });

  it("shares at 1080p15 until another known quality is picked", () => {
    expect(loadShareQuality()).toBe("1080p15");
    localStorage.setItem(SHARE_QUALITY_STORAGE_KEY, "4k60");
    expect(loadShareQuality()).toBe("1080p15");
    storeShareQuality("1080p60");
    expect(loadShareQuality()).toBe("1080p60");
  });

  it("clamps volumes to 0–200 and reads an empty entry as 100", () => {
    expect(loadParticipantVolume(5)).toBe(100);
    storeParticipantVolume(5, 150);
    expect(loadParticipantVolume(5)).toBe(150);
    localStorage.setItem(`${PARTICIPANT_VOLUME_PREFIX}5`, "900");
    expect(loadParticipantVolume(5)).toBe(200);
    localStorage.setItem(`${PARTICIPANT_VOLUME_PREFIX}5`, "");
    expect(loadParticipantVolume(5)).toBe(100);
    localStorage.setItem(`${PARTICIPANT_VOLUME_PREFIX}5`, "loud");
    expect(loadParticipantVolume(5)).toBe(100);
  });

  it("remembers a local mute and forgets it on unmute", () => {
    storeParticipantMuted(5, true);
    expect(loadParticipantMuted(5)).toBe(true);
    storeParticipantMuted(5, false);
    expect(loadParticipantMuted(5)).toBe(false);
    expect(localStorage.length).toBe(0);
  });
});

describe("the meter", () => {
  it("scales conversational levels into the top half without pinning", () => {
    expect(scaleMeterVolume(0)).toBe(0);
    expect(scaleMeterVolume(-1)).toBe(0);
    expect(scaleMeterVolume(Number.NaN)).toBe(0);
    expect(scaleMeterVolume(0.2)).toBe(50);
    expect(scaleMeterVolume(1)).toBe(100);
  });
});

describe("fake tokens", () => {
  it("parses what the mock server issues", () => {
    expect(
      parseFakeToken(
        JSON.stringify({ identity: "a", userId: 3, canPublish: false, simulate: true }),
      ),
    ).toEqual({ identity: "a", userId: 3, canPublish: false, simulate: true });
  });

  it("rejects anything else", () => {
    expect(parseFakeToken("eyJhbGciOi.jwt")).toBeNull();
    expect(parseFakeToken(JSON.stringify({ identity: 3, userId: 3 }))).toBeNull();
    expect(parseFakeToken(JSON.stringify({ identity: "a", userId: "x" }))).toBeNull();
    expect(parseFakeToken("null")).toBeNull();
  });
});
