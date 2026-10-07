/**
 * The media layer behind a call, as the call controller sees it. `LiveKitTransport` drives a real
 * LiveKit room (livekit-client, loaded on first use); `FakeTransport` simulates one for the mock
 * backend, unit tests and Playwright, so the whole call UI runs without a LiveKit server. Plain
 * TypeScript: nothing here knows about React or the store.
 */
import type { StreamQuality } from "../../../gen/StreamQuality.ts";

/** LiveKit's per-participant connection quality. */
export type ConnectionQuality = "excellent" | "good" | "poor" | "lost" | "unknown";

/** Why the room went away without the viewer leaving (LiveKit's `DisconnectReason`, grouped). */
export type DisconnectReason = "removed" | "ended" | "replaced" | "lost";

export type DeviceKind = "audioinput" | "audiooutput" | "videoinput";

/** Where a video track comes from. */
export type VideoSource = "camera" | "screen";

/** The viewer's choice for a watched stream: let adaptive streaming decide, or pin a layer. */
export type ViewerQuality = "auto" | "low" | "high";

export interface CallParticipant {
  /** The LiveKit identity: one per tab or device, so one person can appear twice. */
  readonly identity: string;
  readonly name: string;
  readonly local: boolean;
  readonly speaking: boolean;
  /** The voice level, 0 (silence) to 1. */
  readonly audioLevel: number;
  readonly microphoneMuted: boolean;
  readonly quality: ConnectionQuality;
  /** The camera track's id, for `attachVideo`; `null` while the camera is off. */
  readonly cameraId: string | null;
  /** The screen share's video track id; `null` while not sharing. */
  readonly screenId: string | null;
}

export interface CallSnapshot {
  /** The viewer first, then everyone else by name. */
  readonly participants: readonly CallParticipant[];
  readonly microphoneEnabled: boolean;
  readonly cameraEnabled: boolean;
  readonly screenSharing: boolean;
  /** False until a gesture lets the page play audio (autoplay policies, or a suspended boost). */
  readonly canPlaybackAudio: boolean;
  /**
   * A boost above 100% can't follow the chosen speaker in this browser (no
   * `AudioContext.setSinkId`), so volumes cap at 100% until the default speaker is chosen.
   */
  readonly boostCapped: boolean;
}

/** A screen captured inside the Go-live gesture, not yet published (the transport holds it). */
export interface CapturedScreen {
  readonly id: number;
}

/** No call: what the dock shows before connecting and after leaving. */
export const EMPTY_SNAPSHOT: CallSnapshot = {
  participants: [],
  microphoneEnabled: false,
  cameraEnabled: false,
  screenSharing: false,
  canPlaybackAudio: true,
  boostCapped: false,
};

/** Device pixels, for asking a screen share's layer. */
export interface VideoSize {
  readonly width: number;
  readonly height: number;
}

/** What the connection details panel shows (sampled on demand, never sent anywhere). */
export interface ConnectionStats {
  readonly rttMs: number | null;
  readonly lossRatio: number | null;
  readonly jitterMs: number | null;
  readonly rxBps: number | null;
  readonly txBps: number | null;
  readonly relayed: boolean | null;
}

export type TransportEvent =
  /** Anything in `snapshot()` changed. */
  | { readonly type: "changed" }
  | { readonly type: "reconnecting" }
  | { readonly type: "reconnected" }
  | { readonly type: "disconnected"; readonly reason: DisconnectReason }
  | { readonly type: "quality"; readonly quality: ConnectionQuality }
  /** The SDK moved a track to another device (the chosen one vanished). */
  | { readonly type: "device"; readonly kind: DeviceKind }
  /** The local screen share was unpublished (the browser's own Stop control, say). */
  | { readonly type: "screen-ended" }
  /** A noise-suppression sync finished (asked for, or after the SDK replaced the microphone). */
  | { readonly type: "noise"; readonly outcome: NoiseOutcome };

/** Where a noise-suppression sync left the microphone. */
export interface NoiseOutcome {
  /** RNNoise is attached and filtering. */
  readonly processing: boolean;
  /**
   * RNNoise was wanted and didn't start: `unsupported` (this browser can't run it; off for the
   * page) or `failed` (a load or context failure that may work next time).
   */
  readonly failure: "unsupported" | "failed" | null;
  /** Re-acquiring the microphone failed and it is silent. */
  readonly microphoneLost: boolean;
  /** The processor, the capture constraints and the last restart all agree. */
  readonly settled: boolean;
}

export interface CallTransport {
  /**
   * Connects and subscribes; rejects when the room can't be joined. `noiseSuppression` says
   * whether RNNoise is wanted: the microphone then asks the browser for no suppression of its
   * own, so the signal is filtered exactly once.
   */
  connect(url: string, token: string, noiseSuppression: boolean): Promise<void>;
  /** Leaves the room and stops every local track. Never rejects. */
  disconnect(): Promise<void>;
  snapshot(): CallSnapshot;
  /** Calls `listener` for every transport event; returns the unsubscribe. */
  subscribe(listener: (event: TransportEvent) => void): () => void;
  /** Lets audio play (call from a gesture); harmless when it already can. */
  startAudio(): Promise<void>;
  /** Opens or closes the microphone (the remembered device is an "ideal" constraint). */
  setMicrophone(enabled: boolean): Promise<void>;
  /** On publishes a camera track; off unpublishes it, releasing the device. */
  setCamera(enabled: boolean): Promise<void>;
  /** An ordinary screen share (tab audio included where the browser allows). */
  setScreenShare(enabled: boolean): Promise<void>;
  /** Starts a screen capture; call synchronously inside the gesture (Safari requires it). */
  captureScreen(): Promise<CapturedScreen>;
  /** Publishes a captured screen at a stream's quality. */
  publishScreen(captured: CapturedScreen, quality: StreamQuality): Promise<void>;
  /** Stops captured tracks that won't be published. */
  discardScreen(captured: CapturedScreen): void;
  /** Moves a live track (or the speaker) to another device; false when the switch failed. */
  switchDevice(kind: DeviceKind, deviceId: string): Promise<boolean>;
  /** The device a live track is on, as the SDK reports it. */
  activeDevice(kind: DeviceKind): string | undefined;
  /** Plays a video track (`cameraId` or `screenId`) in `element`. */
  attachVideo(videoId: string, element: HTMLVideoElement): void;
  detachVideo(videoId: string, element: HTMLVideoElement): void;
  /** A remote person's volume for this browser only: 0–200 (above 100 boosts). */
  setParticipantVolume(identity: string, volume: number): void;
  /** Stops (or resumes) receiving one person's microphone. */
  setParticipantMuted(identity: string, muted: boolean): void;
  /** Silences (or restores) every remote voice for this browser. */
  setDeafened(deafened: boolean): void;
  /**
   * Turns RNNoise on or off on the live microphone (queued: overlapping calls run in order). The
   * outcome arrives as a `noise` event before the promise settles; nothing happens without a
   * microphone track.
   */
  setNoiseSuppression(enabled: boolean): Promise<void>;
  /** The live microphone's level for the meter, 0–100; 0 while muted. */
  microphoneLevel(): number;
  /** Pins (or frees) the layer a watched stream arrives at. */
  setViewerQuality(videoId: string, quality: ViewerQuality): void;
  /** Asks for an ordinary screen share's top layer, sized to where it shows when expanded. */
  showScreen(videoId: string, size: VideoSize | null): void;
  /**
   * One sample of the connection's statistics, `null` while there is nothing to sample. `fresh`
   * starts a new bitrate baseline (a reopened panel shouldn't average over the closed time).
   */
  sampleStats(fresh: boolean): Promise<ConnectionStats | null>;
}

/** Builds the transport for a set of credentials (`mock:` URLs get the fake). */
export type TransportFactory = (url: string) => Promise<CallTransport>;
