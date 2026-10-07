/**
 * Connection statistics for the call's details panel, sampled from WebRTC `getStats` reports on
 * demand and never sent anywhere (the classic `lib/huddle/stats.js`). The publisher report comes
 * from the microphone's sender, the subscriber report from a subscribed remote track's receiver.
 * Round-trip time, loss and jitter describe the upstream path as the server reports it back;
 * the received bitrate describes the subscriber path.
 */
import type { ConnectionStats } from "./transport.ts";

/** The fields this reads from the report's entries (all optional: entry types differ). */
export interface StatEntry {
  readonly id?: string;
  readonly type?: string;
  readonly kind?: string;
  readonly isRemote?: boolean;
  readonly bytesSent?: number;
  readonly bytesReceived?: number;
  readonly packetsSent?: number;
  readonly packetsLost?: number;
  readonly roundTripTime?: number;
  readonly jitter?: number;
  readonly selectedCandidatePairId?: string;
  readonly localCandidateId?: string;
  readonly remoteCandidateId?: string;
  readonly nominated?: boolean;
  readonly selected?: boolean;
  readonly state?: string;
  readonly candidateType?: string;
}

export type StatsReport = ReadonlyMap<string, StatEntry>;

/** The byte counters a bitrate is measured against. */
export interface StatsBaseline {
  readonly at: number;
  readonly txBytes: number | null;
  readonly rxBytes: number | null;
}

export interface StatsSample {
  readonly stats: ConnectionStats;
  readonly baseline: StatsBaseline;
}

function finite(value: number | null | undefined): value is number {
  return value !== null && value !== undefined && Number.isFinite(value);
}

interface PathReading {
  readonly pair: StatEntry | null;
  readonly candidates: ReadonlyMap<string, StatEntry>;
}

function relayed(reading: PathReading, report: StatsReport | null): boolean | null {
  const { pair, candidates } = reading;

  if (pair === null) {
    return null;
  }

  const lookup = (id: string | undefined): StatEntry | undefined =>
    id === undefined ? undefined : (candidates.get(id) ?? report?.get(id));

  const local = lookup(pair.localCandidateId);
  const remote = lookup(pair.remoteCandidateId);

  if (local === undefined && remote === undefined) {
    return null;
  }

  return local?.candidateType === "relay" || remote?.candidateType === "relay";
}

/** Tracks the selected candidate pair as entries stream past. */
class PathTracker {
  #pair: StatEntry | null = null;
  #nominated: StatEntry | null = null;
  readonly #candidates = new Map<string, StatEntry>();

  see(stat: StatEntry, report: StatsReport): boolean {
    if (stat.type === "transport" && stat.selectedCandidatePairId !== undefined) {
      this.#pair = report.get(stat.selectedCandidatePairId) ?? this.#pair;

      return true;
    }

    if (stat.type === "candidate-pair") {
      if (stat.nominated === true || stat.selected === true) {
        this.#pair ??= stat;
      }

      if (stat.state === "succeeded") {
        this.#nominated ??= stat;
      }

      return true;
    }

    if (
      (stat.type === "local-candidate" || stat.type === "remote-candidate") &&
      stat.id !== undefined
    ) {
      this.#candidates.set(stat.id, stat);

      return true;
    }

    return false;
  }

  reading(): PathReading {
    const pair = this.#pair?.localCandidateId === undefined ? this.#nominated : this.#pair;

    return { pair, candidates: this.#candidates };
  }
}

interface PublisherReading {
  readonly rttMs: number | null;
  readonly lossRatio: number | null;
  readonly jitterMs: number | null;
  readonly bytesSent: number | null;
  readonly relayed: boolean | null;
}

function secondsToMs(seconds: number | undefined): number | null {
  return finite(seconds) && seconds >= 0 ? seconds * 1000 : null;
}

function lossRatio(lost: number | undefined, total: number | null): number | null {
  if (!finite(lost) || !finite(total) || total <= 0) {
    return null;
  }

  return Math.min(1, Math.max(0, lost / total));
}

function readPublisher(report: StatsReport | null): PublisherReading {
  // Audio is the stable track: it is published for the whole call while video comes and goes,
  // so video feedback only stands in when there is no audio feedback.
  let feedback: StatEntry | null = null;
  let fallbackFeedback: StatEntry | null = null;
  // Null until an outbound entry is seen: a failed report must read as "no sample", not as zero
  // bytes, or the next bitrate spikes off a zero base.
  let bytesSent: number | null = null;
  let audioPacketsSent: number | null = null;
  let totalPacketsSent = 0;
  let packetsSentSeen = false;
  const path = new PathTracker();

  for (const stat of report?.values() ?? []) {
    if (stat.type === "outbound-rtp" && stat.isRemote !== true) {
      if (finite(stat.bytesSent)) {
        bytesSent = (bytesSent ?? 0) + stat.bytesSent;
      }

      // Chrome doesn't repeat `packetsSent` on the remote-inbound entry, so the loss
      // denominator comes from the outbound side of the same flow.
      if (finite(stat.packetsSent)) {
        packetsSentSeen = true;
        totalPacketsSent += stat.packetsSent;

        if (stat.kind === "audio") {
          audioPacketsSent = (audioPacketsSent ?? 0) + stat.packetsSent;
        }
      }
    } else if (stat.type === "remote-inbound-rtp") {
      if (stat.kind === "audio") {
        feedback = stat;
      } else {
        fallbackFeedback ??= stat;
      }
    } else if (report !== null) {
      path.see(stat, report);
    }
  }

  const upstream: StatEntry | null = feedback ?? fallbackFeedback;
  const everySent = packetsSentSeen ? totalPacketsSent : null;
  const sent = upstream?.kind === "audio" ? (audioPacketsSent ?? everySent) : everySent;
  const lost = upstream?.packetsLost;
  const total = sent === null || !finite(lost) ? null : sent + lost;

  return {
    rttMs: secondsToMs(upstream?.roundTripTime),
    lossRatio: lossRatio(lost, total),
    jitterMs: secondsToMs(upstream?.jitter),
    bytesSent,
    relayed: relayed(path.reading(), report),
  };
}

interface SubscriberReading {
  readonly bytesReceived: number | null;
  readonly relayed: boolean | null;
}

function readSubscriber(report: StatsReport | null): SubscriberReading {
  // The same "no sample" rule: nothing subscribed reads as unknown, not zero.
  let bytesReceived: number | null = null;
  const path = new PathTracker();

  for (const stat of report?.values() ?? []) {
    if (stat.type === "inbound-rtp" && finite(stat.bytesReceived)) {
      bytesReceived = (bytesReceived ?? 0) + stat.bytesReceived;
    } else if (report !== null) {
      path.see(stat, report);
    }
  }

  return { bytesReceived, relayed: relayed(path.reading(), report) };
}

function bitrate(
  bytes: number | null,
  previousBytes: number | null | undefined,
  now: number,
  previousAt: number | undefined,
): number | null {
  if (!finite(bytes) || !finite(previousBytes) || !finite(previousAt)) {
    return null;
  }

  const elapsed = (now - previousAt) / 1000;

  if (elapsed <= 0 || bytes < previousBytes) {
    return null;
  }

  return ((bytes - previousBytes) * 8) / elapsed;
}

/** One sample from the two reports, with bitrates measured against the previous sample. */
export function summarizeConnectionStats(
  publisherReport: StatsReport | null,
  subscriberReport: StatsReport | null,
  previous: StatsBaseline | null,
  now: number,
): StatsSample {
  const publisher = readPublisher(publisherReport);
  const subscriber = readSubscriber(subscriberReport);

  return {
    stats: {
      rttMs: publisher.rttMs,
      lossRatio: publisher.lossRatio,
      jitterMs: publisher.jitterMs,
      rxBps: bitrate(subscriber.bytesReceived, previous?.rxBytes, now, previous?.at),
      txBps: bitrate(publisher.bytesSent, previous?.txBytes, now, previous?.at),
      relayed: publisher.relayed ?? subscriber.relayed,
    },
    baseline: { at: now, txBytes: publisher.bytesSent, rxBytes: subscriber.bytesReceived },
  };
}

export interface FormattedStats {
  readonly rtt: string;
  readonly loss: string;
  readonly jitter: string;
  readonly received: string;
  readonly sent: string;
  readonly transport: string;
}

function milliseconds(value: number | null): string {
  if (!finite(value)) {
    return "–";
  }

  return value < 10 ? `${value.toFixed(1)} ms` : `${Math.round(value)} ms`;
}

function percent(value: number | null): string {
  return finite(value) ? `${(value * 100).toFixed(1)}%` : "–";
}

function bits(value: number | null): string {
  if (!finite(value)) {
    return "–";
  }

  if (value >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1)} Mbps`;
  }

  return `${Math.max(0, Math.round(value / 1000))} kbps`;
}

/** The panel's text for one sample ("–" where nothing was measured). */
export function formatConnectionStats(stats: ConnectionStats): FormattedStats {
  let transport = "–";

  if (stats.relayed !== null) {
    transport = stats.relayed ? "Relayed (TURN)" : "Direct";
  }

  return {
    rtt: milliseconds(stats.rttMs),
    loss: percent(stats.lossRatio),
    jitter: milliseconds(stats.jitterMs),
    received: bits(stats.rxBps),
    sent: bits(stats.txBps),
    transport,
  };
}
