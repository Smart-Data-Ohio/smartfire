/**
 * A simulated call for the mock backend, unit tests and Playwright: the whole call UI runs
 * without a LiveKit server. The mock's credentials carry `mock://` as the URL and a JSON token
 * (`{identity, userId, canPublish, simulate}`). The other people in the call are whoever the
 * store's presence lists for the room (one LiveKit identity per tab, as in a real call); with
 * `simulate` on they take turns speaking. Tests steer everything else through
 * `window.__smartfireHuddle` (speaking, disconnects, reconnects, quality, remote cameras and
 * screens, refused screen shares).
 */
import type { StreamQuality } from "../../../gen/StreamQuality.ts";
import { store } from "../../../store/store.ts";
import { DEFAULT_SHARE_QUALITY } from "./screen-quality.ts";
import {
  type CallParticipant,
  type CallSnapshot,
  type CallTransport,
  type CapturedScreen,
  type ConnectionQuality,
  type ConnectionStats,
  type DeviceKind,
  type DisconnectReason,
  EMPTY_SNAPSHOT,
  type TransportEvent,
  type VideoSize,
  type ViewerQuality,
} from "./transport.ts";

/** What the mock server puts in a fake token. */
export interface FakeToken {
  readonly identity: string;
  readonly userId: number;
  readonly canPublish: boolean;
  readonly simulate: boolean;
}

export function parseFakeToken(token: string): FakeToken | null {
  try {
    const parsed: unknown = JSON.parse(token);

    if (!(parsed instanceof Object)) {
      return null;
    }

    const fields = new Map(Object.entries(parsed));
    const identity = fields.get("identity");
    const userId = Number(fields.get("userId"));

    if (`${identity}` !== identity || !Number.isInteger(userId)) {
      return null;
    }

    return {
      identity,
      userId,
      canPublish: fields.get("canPublish") !== false,
      simulate: fields.get("simulate") === true,
    };
  } catch {
    return null;
  }
}

/** The steering wheel Playwright holds. */
export interface FakeCallHooks {
  /** Forces someone (by user id) speaking or quiet; `null` returns them to the simulation. */
  speak(userId: number, speaking: boolean | null): void;
  disconnect(reason: DisconnectReason): void;
  reconnecting(): void;
  reconnected(): void;
  /** The viewer's own connection quality. */
  quality(quality: ConnectionQuality): void;
  remoteQuality(userId: number, quality: ConnectionQuality): void;
  remoteCamera(userId: number, on: boolean): void;
  remoteScreen(userId: number, on: boolean): void;
  /** The next screen capture rejects with an error of this name (`NotAllowedError`…). */
  failNextScreen(name: string): void;
  /** Whether the page could play audio (false shows the "Resume audio" control). */
  playback(allowed: boolean): void;
  /** The current snapshot, for assertions. */
  snapshot(): CallSnapshot;
}

declare global {
  interface Window {
    __smartfireHuddle?: FakeCallHooks;
  }
}

const SPEAKING_TURN_MS = 2500;

const PALETTE = ["#4f7cff", "#e5484d", "#30a46c", "#f5a524", "#8e4ec6"];

/** A moving test pattern, so a video element has something to show (no-op where unsupported). */
function patternStream(label: string, seed: number): MediaStream | null {
  const canvas = document.createElement("canvas");

  if (!("captureStream" in canvas)) {
    return null;
  }

  canvas.width = 640;
  canvas.height = 360;

  const context = canvas.getContext("2d");

  if (context === null) {
    return null;
  }

  let frame = 0;

  const draw = (): void => {
    frame += 1;
    context.fillStyle = PALETTE[seed % PALETTE.length] ?? "#4f7cff";
    context.fillRect(0, 0, canvas.width, canvas.height);
    context.fillStyle = "rgba(255, 255, 255, 0.85)";
    context.font = "32px sans-serif";
    context.fillText(label, 32, 64);
    context.fillRect((frame * 4) % canvas.width, canvas.height - 24, 80, 8);
  };

  draw();

  const stream = canvas.captureStream(15);
  const timer = setInterval(draw, 1000 / 15);

  for (const track of stream.getTracks()) {
    track.addEventListener("ended", () => {
      clearInterval(timer);
    });
  }

  return stream;
}

interface FakeVideo {
  readonly id: string;
  readonly stream: MediaStream | null;
}

function stopVideo(video: FakeVideo | null): void {
  for (const track of video?.stream?.getTracks() ?? []) {
    track.stop();
  }
}

export class FakeTransport implements CallTransport {
  #token: FakeToken | null = null;
  #roomId: number;
  #connected = false;
  readonly #listeners = new Set<(event: TransportEvent) => void>();
  #unsubscribeStore: (() => void) | null = null;
  #microphone = false;
  #camera: FakeVideo | null = null;
  #screen: FakeVideo | null = null;
  #canPlayback = true;
  #deafened = false;
  #localQuality: ConnectionQuality = "excellent";
  readonly #remoteQuality = new Map<number, ConnectionQuality>();
  readonly #forcedSpeaking = new Map<number, boolean>();
  readonly #remoteCameras = new Map<number, FakeVideo>();
  readonly #remoteScreens = new Map<number, FakeVideo>();
  readonly #volumes = new Map<string, number>();
  readonly #muted = new Set<string>();
  readonly #captured = new Map<number, FakeVideo>();
  #captureSequence = 0;
  #videoSequence = 0;
  #failScreen: string | null = null;
  /** The quality the latest share or stream capture asked for (what the real engine encodes). */
  screenQuality: StreamQuality | null = null;
  #noise = false;
  readonly #startedAt = Date.now();

  constructor(roomId: number) {
    this.#roomId = roomId;
  }

  async connect(_url: string, token: string, noiseSuppression: boolean): Promise<void> {
    const parsed = parseFakeToken(token);

    if (parsed === null) {
      throw new Error("invalid-fake-token");
    }

    await Promise.resolve();
    this.#token = parsed;
    this.#noise = noiseSuppression;
    this.#connected = true;
    this.#unsubscribeStore = store.subscribe((state, previous) => {
      if (
        state.huddles[this.#roomId] !== previous.huddles[this.#roomId] ||
        state.stages[this.#roomId] !== previous.stages[this.#roomId]
      ) {
        this.#emit({ type: "changed" });
      }
    });
    window.__smartfireHuddle = this.#hooks();
  }

  async disconnect(): Promise<void> {
    this.#teardown();
  }

  snapshot(): CallSnapshot {
    const token = this.#token;

    if (!this.#connected || token === null) {
      return EMPTY_SNAPSHOT;
    }

    const local: CallParticipant = {
      identity: token.identity,
      name: store.getState().users[token.userId]?.name ?? "You",
      local: true,
      speaking: false,
      audioLevel: 0,
      microphoneMuted: !this.#microphone,
      quality: this.#localQuality,
      cameraId: this.#camera?.id ?? null,
      screenId: this.#screen?.id ?? null,
    };

    return {
      participants: [local, ...this.#remotes(token)],
      microphoneEnabled: this.#microphone,
      cameraEnabled: this.#camera !== null,
      screenSharing: this.#screen !== null,
      canPlaybackAudio: this.#canPlayback,
      boostCapped: false,
    };
  }

  subscribe(listener: (event: TransportEvent) => void): () => void {
    this.#listeners.add(listener);

    return () => {
      this.#listeners.delete(listener);
    };
  }

  async startAudio(): Promise<void> {
    this.#canPlayback = true;
    this.#emit({ type: "changed" });
  }

  async setMicrophone(enabled: boolean): Promise<void> {
    if (enabled && this.#token?.canPublish === false) {
      throw new Error("insufficient permissions to publish");
    }

    this.#microphone = enabled;
    this.#emit({ type: "changed" });
  }

  async setCamera(enabled: boolean): Promise<void> {
    stopVideo(this.#camera);
    this.#camera = enabled ? this.#video("Your camera") : null;
    this.#emit({ type: "changed" });
  }

  async setScreenShare(
    enabled: boolean,
    quality: StreamQuality = DEFAULT_SHARE_QUALITY,
  ): Promise<void> {
    if (!enabled) {
      this.#endScreen();

      return;
    }

    this.#refuseScreenIfAsked();
    this.screenQuality = quality;
    stopVideo(this.#screen);
    this.#screen = this.#video("Your screen");
    this.#emit({ type: "changed" });
  }

  captureScreen(quality: StreamQuality): Promise<CapturedScreen> {
    try {
      this.#refuseScreenIfAsked();
    } catch (error) {
      return Promise.reject(error);
    }

    this.screenQuality = quality;
    this.#captureSequence += 1;
    this.#captured.set(this.#captureSequence, this.#video("Your stream"));

    return Promise.resolve({ id: this.#captureSequence });
  }

  async publishScreen(captured: CapturedScreen, quality: StreamQuality): Promise<void> {
    const video = this.#captured.get(captured.id);

    if (video === undefined || !this.#connected) {
      throw new Error("screen-capture-gone");
    }

    this.screenQuality = quality;
    this.#captured.delete(captured.id);
    stopVideo(this.#screen);
    this.#screen = video;
    this.#emit({ type: "changed" });
  }

  discardScreen(captured: CapturedScreen): void {
    stopVideo(this.#captured.get(captured.id) ?? null);
    this.#captured.delete(captured.id);
  }

  async switchDevice(_kind: DeviceKind, _deviceId: string): Promise<boolean> {
    return this.#connected;
  }

  activeDevice(_kind: DeviceKind): string | undefined {
    return undefined;
  }

  attachVideo(videoId: string, element: HTMLVideoElement): void {
    const stream = this.#findVideo(videoId)?.stream ?? null;

    if (stream !== null) {
      element.muted = true;
      element.srcObject = stream;
      void element.play().catch(() => undefined);
    }
  }

  detachVideo(videoId: string, element: HTMLVideoElement): void {
    if (element.srcObject === (this.#findVideo(videoId)?.stream ?? null)) {
      element.srcObject = null;
    }
  }

  setParticipantVolume(identity: string, volume: number): void {
    this.#volumes.set(identity, volume);
  }

  setParticipantMuted(identity: string, muted: boolean): void {
    if (muted) {
      this.#muted.add(identity);
    } else {
      this.#muted.delete(identity);
    }
  }

  setDeafened(deafened: boolean): void {
    this.#deafened = deafened;
  }

  /** What the fake would play someone at (tests read it). */
  playbackVolume(identity: string): number {
    if (this.#deafened || this.#muted.has(identity)) {
      return 0;
    }

    return this.#volumes.get(identity) ?? 100;
  }

  async setNoiseSuppression(enabled: boolean): Promise<void> {
    if (!this.#connected || !this.#microphone) {
      this.#noise = enabled;

      return;
    }

    this.#noise = enabled;
    this.#emit({
      type: "noise",
      outcome: { processing: enabled, failure: null, microphoneLost: false, settled: true },
    });
  }

  /** Whether RNNoise would be filtering (tests read it). */
  get noiseSuppression(): boolean {
    return this.#noise;
  }

  microphoneLevel(): number {
    if (!this.#microphone || this.#token?.simulate !== true) {
      return 0;
    }

    return Math.round(40 + 30 * Math.sin((Date.now() - this.#startedAt) / 180));
  }

  setViewerQuality(_videoId: string, _quality: ViewerQuality): void {}

  showScreen(_videoId: string, _size: VideoSize | null): void {}

  async sampleStats(_fresh: boolean): Promise<ConnectionStats | null> {
    if (!this.#connected) {
      return null;
    }

    return {
      rttMs: 42,
      lossRatio: 0.004,
      jitterMs: 3.2,
      rxBps: 64_000,
      txBps: 48_000,
      relayed: false,
    };
  }

  #emit(event: TransportEvent): void {
    for (const listener of this.#listeners) {
      listener(event);
    }
  }

  #teardown(): void {
    this.#connected = false;
    this.#unsubscribeStore?.();
    this.#unsubscribeStore = null;
    stopVideo(this.#camera);
    stopVideo(this.#screen);

    for (const video of [
      ...this.#captured.values(),
      ...this.#remoteCameras.values(),
      ...this.#remoteScreens.values(),
    ]) {
      stopVideo(video);
    }

    this.#camera = null;
    this.#screen = null;
    this.#captured.clear();
    this.#remoteCameras.clear();
    this.#remoteScreens.clear();
    this.#microphone = false;

    if (window.__smartfireHuddle !== undefined) {
      delete window.__smartfireHuddle;
    }
  }

  #video(label: string): FakeVideo {
    this.#videoSequence += 1;

    return {
      id: `fake-video-${this.#videoSequence}`,
      stream: patternStream(label, this.#videoSequence),
    };
  }

  #findVideo(videoId: string): FakeVideo | null {
    for (const video of [
      this.#camera,
      this.#screen,
      ...this.#remoteCameras.values(),
      ...this.#remoteScreens.values(),
    ]) {
      if (video?.id === videoId) {
        return video;
      }
    }

    return null;
  }

  #refuseScreenIfAsked(): void {
    const name = this.#failScreen;

    if (name === null) {
      return;
    }

    this.#failScreen = null;

    const error = new Error(
      name === "NotAllowedError" ? "Permission denied" : "Screen capture failed",
    );

    error.name = name;

    throw error;
  }

  #endScreen(): void {
    if (this.#screen === null) {
      return;
    }

    stopVideo(this.#screen);
    this.#screen = null;
    this.#emit({ type: "changed" });
    this.#emit({ type: "screen-ended" });
  }

  #remotes(token: FakeToken): CallParticipant[] {
    const state = store.getState();
    const presence = state.huddles[this.#roomId];
    const presenter = state.stages[this.#roomId]?.live?.identity ?? null;
    const turn = Math.floor((Date.now() - this.#startedAt) / SPEAKING_TURN_MS);
    const remotes: CallParticipant[] = [];

    for (const participant of presence?.participants ?? []) {
      for (const identity of participant.identities) {
        if (identity !== token.identity) {
          const forced = this.#forcedSpeaking.get(participant.userId);
          const index = remotes.length;
          const simulated = token.simulate && !participant.serverMuted && turn % 4 === index % 4;
          const speaking = forced ?? simulated;
          const presenting = identity === presenter;

          if (presenting && !this.#remoteScreens.has(participant.userId)) {
            this.#remoteScreens.set(
              participant.userId,
              this.#video(`${participant.userId}'s stream`),
            );
          }

          remotes.push({
            identity,
            name: state.users[participant.userId]?.name ?? identity,
            local: false,
            speaking,
            audioLevel: speaking ? 0.35 + 0.25 * Math.abs(Math.sin(Date.now() / 160 + index)) : 0,
            microphoneMuted: participant.serverMuted,
            quality: this.#remoteQuality.get(participant.userId) ?? "good",
            cameraId: this.#remoteCameras.get(participant.userId)?.id ?? null,
            screenId: this.#remoteScreens.get(participant.userId)?.id ?? null,
          });
        }
      }
    }

    return remotes.sort((left, right) => left.name.localeCompare(right.name));
  }

  #toggleRemote(videos: Map<number, FakeVideo>, userId: number, on: boolean, label: string): void {
    stopVideo(videos.get(userId) ?? null);
    videos.delete(userId);

    if (on) {
      videos.set(userId, this.#video(label));
    }

    this.#emit({ type: "changed" });
  }

  #hooks(): FakeCallHooks {
    return {
      speak: (userId, speaking) => {
        if (speaking === null) {
          this.#forcedSpeaking.delete(userId);
        } else {
          this.#forcedSpeaking.set(userId, speaking);
        }

        this.#emit({ type: "changed" });
      },
      disconnect: (reason) => {
        this.#teardown();
        this.#emit({ type: "disconnected", reason });
      },
      reconnecting: () => {
        this.#emit({ type: "reconnecting" });
      },
      reconnected: () => {
        this.#emit({ type: "reconnected" });
        this.#emit({ type: "changed" });
      },
      quality: (quality) => {
        this.#localQuality = quality;
        this.#emit({ type: "quality", quality });
      },
      remoteQuality: (userId, quality) => {
        this.#remoteQuality.set(userId, quality);
        this.#emit({ type: "changed" });
      },
      remoteCamera: (userId, on) => {
        this.#toggleRemote(this.#remoteCameras, userId, on, `${userId}'s camera`);
      },
      remoteScreen: (userId, on) => {
        this.#toggleRemote(this.#remoteScreens, userId, on, `${userId}'s screen`);
      },
      failNextScreen: (name) => {
        this.#failScreen = name;
      },
      playback: (allowed) => {
        this.#canPlayback = allowed;
        this.#emit({ type: "changed" });
      },
      snapshot: () => this.snapshot(),
    };
  }
}
