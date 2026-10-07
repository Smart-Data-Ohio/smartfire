/**
 * The call, as the classic `huddle_controller.js` runs it, minus the DOM: join (with the
 * first-join device check), connect, the 30-second reconnect countdown, leave, the role-change
 * rejoin, mute and push-to-talk, camera, screen share and Go live, device switching, noise
 * suppression, per-person volume, the 45-second access check, and the ends (pagehide, another
 * account, a revoked grant). An operation counter drops the results of anything a newer
 * action overtook. The media itself sits behind a `CallTransport`.
 */
import type { HuddleCredentials } from "../../gen/HuddleCredentials.ts";
import type { StageRole } from "../../gen/StageRole.ts";
import type { StageStream } from "../../gen/StageStream.ts";
import type { StreamQuality } from "../../gen/StreamQuality.ts";
import { store } from "../../store/store.ts";
import { huddles } from "../../sync/huddles.ts";
import { ActionError } from "../../sync/run.ts";
import {
  activePhase,
  type CallBusy,
  type CallPhase,
  type CallState,
  callStore,
  initialCallState,
  livePhase,
  type PrejoinState,
  streamVideoIdOf,
  userIdForIdentity,
} from "./call-store.ts";
import {
  canShareScreen,
  type DeviceLists,
  listDevices,
  type MediaDeviceOption,
  permissionDenied,
  shouldCheckDevices,
} from "./engine/devices.ts";
import { createLevelMeter, type LevelMeter } from "./engine/meter.ts";
import {
  type DevicePreferences,
  loadDevicePreferences,
  loadNoiseSuppression,
  loadParticipantMuted,
  loadParticipantVolume,
  loadStreamQuality,
  storeDevicePreference,
  storeNoiseSuppression,
  storeParticipantMuted,
  storeParticipantVolume,
  storeStreamQuality,
} from "./engine/preferences.ts";
import {
  type CallTransport,
  type DeviceKind,
  type DisconnectReason,
  EMPTY_SNAPSHOT,
  type NoiseOutcome,
  type TransportEvent,
  type ViewerQuality,
} from "./engine/transport.ts";

export const AUTH_CHECK_INTERVAL_MS = 45_000;

export const STATS_INTERVAL_MS = 2_000;

export const RECONNECT_COUNTDOWN_SECONDS = 30;

const METER_INTERVAL_MS = 100;

const SNAPSHOT_INTERVAL_MS = 200;

const STATUS_RESET_MS = 4_000;

export const SERVER_MUTED_NOTICE = "A host muted you";

export const MICROPHONE_RESTART_ERROR =
  "The microphone couldn’t be restarted. Check that it is still connected and try again.";

const SIGN_IN_EXPIRED = "Your sign-in expired. Sign in again to join a huddle.";

/** Everything the controller reaches outside itself, so tests can stand in for it. */
export interface CallEnvironment {
  join(roomId: number): Promise<HuddleCredentials>;
  leave(roomId: number): Promise<void>;
  leaveOnUnload(roomId: number): void;
  /** `GET /rooms/:id/huddle`: rejects with an `ActionError` once access ended. */
  check(roomId: number): Promise<void>;
  startStream(roomId: number, quality: StreamQuality): Promise<StageStream>;
  stopStream(roomId: number, streamId: number | null): Promise<void>;
  stopStreamOnUnload(roomId: number, streamId: number | null): void;
  /** Starts loading the media layer while the credentials are requested. */
  preload(): Promise<void>;
  /** The transport for a set of credentials (`mock:` URLs get the fake). */
  transport(url: string, roomId: number): Promise<CallTransport>;
  shouldCheckDevices(): Promise<boolean>;
  getUserMedia(constraints: MediaStreamConstraints): Promise<MediaStream>;
  listDevices(): Promise<DeviceLists>;
  /** Keeps the room's sync topic subscribed for the call (stage roster, stream, notices). */
  holdRoom(roomId: number): void;
  releaseRoom(roomId: number): void;
}

function isMock(url: string): boolean {
  return url.startsWith("mock:");
}

export const browserEnvironment: CallEnvironment = {
  join: (roomId) => huddles.join(roomId),
  leave: (roomId) => huddles.leave(roomId),
  leaveOnUnload: (roomId) => {
    huddles.leaveOnUnload(roomId);
  },
  check: async (roomId) => {
    await huddles.room(roomId);
  },
  startStream: (roomId, quality) => huddles.startStream(roomId, quality),
  stopStream: (roomId, streamId) => huddles.stopStream(roomId, streamId),
  stopStreamOnUnload: (roomId, streamId) => {
    huddles.stopStreamOnUnload(roomId, streamId);
  },
  preload: async () => {
    // The SDK is large; it loads alongside the credentials request, as the classic join did.
    await import("./engine/livekit.ts")
      .then((module) => module.loadLiveKit())
      .catch(() => undefined);
  },
  transport: async (url, roomId) => {
    if (isMock(url)) {
      const { FakeTransport } = await import("./engine/fake.ts");

      return new FakeTransport(roomId);
    }

    const { createLiveKitTransport } = await import("./engine/livekit.ts");

    return createLiveKitTransport();
  },
  shouldCheckDevices,
  getUserMedia: (constraints) => navigator.mediaDevices.getUserMedia(constraints),
  listDevices,
  holdRoom: (roomId) => {
    huddles.holdRoom(roomId);
  },
  releaseRoom: (roomId) => {
    huddles.releaseRoom(roomId);
  },
};

/** The tag an `ActionError` carries (`NotFound`, `Forbidden`…), or `null` for other errors. */
function errorTag(error: Error): string | null {
  return error instanceof ActionError ? error.tag : null;
}

const DEVICE_KINDS: readonly DeviceKind[] = ["audioinput", "audiooutput", "videoinput"];

function joinErrorMessage(error: Error): string {
  if (permissionDenied(error)) {
    return "Microphone access was denied or cancelled. Allow microphone access and try again. You are not connected.";
  }

  switch (errorTag(error)) {
    case "Unauthorized":
      return SIGN_IN_EXPIRED;
    case "Forbidden":
    case "NotFound":
      return "You no longer have access to this room.";
    case "Unavailable":
      return "Huddles aren’t available on this server right now.";
    default:
      return "The huddle could not connect. Check your connection and try again.";
  }
}

function disconnectMessage(reason: DisconnectReason): string {
  switch (reason) {
    case "removed":
      return "Your access to this huddle ended. Join again if you still have access to the room.";
    case "ended":
      return "This huddle has ended.";
    case "replaced":
      return "This huddle connection was replaced by another connection.";
    default:
      return "The huddle ended because the connection was lost. Try joining again.";
  }
}

const DEVICE_NAMES: Readonly<Record<DeviceKind, string>> = {
  audioinput: "microphone",
  audiooutput: "speaker",
  videoinput: "camera",
};

/** The picker's choice: the preferred device when listed, else the current one, else the first. */
function pick(devices: readonly MediaDeviceOption[], preferred: string, current: string): string {
  const ids = new Set(devices.map((device) => device.deviceId));

  if (preferred !== "" && ids.has(preferred)) {
    return preferred;
  }

  if (current !== "" && ids.has(current)) {
    return current;
  }

  return devices[0]?.deviceId ?? "";
}

function stopStream(stream: MediaStream | null): void {
  for (const track of stream?.getTracks() ?? []) {
    try {
      track.stop();
    } catch {
      // A track that is already gone needs no stopping.
    }
  }
}

/** Typing anywhere (an input, a textarea, an editable element) keeps the push-to-talk key. */
function typingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }

  return (
    target.isContentEditable ||
    target.closest("input, textarea, select, [contenteditable='true'], [contenteditable='']") !==
      null
  );
}

/** A cheap fingerprint of what the call view draws, so polling only commits real changes. */
function snapshotKey(state: CallState["snapshot"]): string {
  return state.participants
    .map(
      (participant) =>
        `${participant.identity}:${participant.speaking ? 1 : 0}:${Math.round(participant.audioLevel * 20)}:` +
        `${participant.microphoneMuted ? 1 : 0}:${participant.quality}:${participant.cameraId}:${participant.screenId}:${participant.name}`,
    )
    .join("|")
    .concat(
      `#${state.microphoneEnabled}${state.cameraEnabled}${state.screenSharing}${state.canPlaybackAudio}${state.boostCapped}`,
    );
}

export class CallController {
  readonly #env: CallEnvironment;
  #operation = 0;
  /** The device check's microphone preview; the camera's has its own, so neither drops the other. */
  #previewOperation = 0;
  #previewVideoOperation = 0;
  #transport: CallTransport | null = null;
  #unsubscribeTransport: (() => void) | null = null;
  #canPublishHint: boolean | null = null;
  #pushToTalkActive = false;
  #pushToTalkOpenedMic = false;
  #microphoneBeforeDeafen = false;
  #reconnectTimer: ReturnType<typeof setInterval> | null = null;
  #authTimer: ReturnType<typeof setInterval> | null = null;
  #authCheck: Promise<void> | null = null;
  #meterTimer: ReturnType<typeof setInterval> | null = null;
  #snapshotTimer: ReturnType<typeof setInterval> | null = null;
  #statsTimer: ReturnType<typeof setInterval> | null = null;
  #statsSampling = false;
  #statusRevision = 0;
  #previewAudio: MediaStream | null = null;
  #previewMeter: LevelMeter | null = null;
  #lastSnapshotKey = "";
  #autoExpanded: string | null = null;
  #heldRoom: number | null = null;

  constructor(env: CallEnvironment) {
    this.#env = env;
  }

  // ── State helpers ─────────────────────────────────────────────────────────────────────────

  get #state(): CallState {
    return callStore.getState();
  }

  #set(partial: Partial<CallState>): void {
    callStore.setState(partial);
  }

  #setBusy(key: keyof CallBusy, value: boolean): void {
    this.#set({ busy: { ...this.#state.busy, [key]: value } });
  }

  #setPrejoin(partial: Partial<PrejoinState>): void {
    const prejoin = this.#state.prejoin;

    if (prejoin !== null) {
      this.#set({ prejoin: { ...prejoin, ...partial } });
    }
  }

  /** Moves the call's topic hold to `roomId` (or drops it). */
  #holdRoom(roomId: number | null): void {
    if (roomId === this.#heldRoom) {
      return;
    }

    if (this.#heldRoom !== null) {
      this.#env.releaseRoom(this.#heldRoom);
    }

    this.#heldRoom = roomId;

    if (roomId !== null) {
      this.#env.holdRoom(roomId);
    }
  }

  #pushToTalk(): boolean {
    return store.getState().me?.preferences.voiceMode === "push_to_talk";
  }

  #pushToTalkKey(): string {
    const key = store.getState().me?.preferences.pushToTalkKey ?? "";

    return key === "" ? "`" : key;
  }

  /**
   * Moves to a phase. A "host muted you" notice survives the rejoin it triggers (otherwise the
   * connecting step would clear it before it's read); failures replace it, and leaving or
   * unmuting clears it.
   */
  #setPhase(phase: CallPhase, message: string, failureTitle: string | null = null): void {
    const failed = failureTitle !== null;
    const keepMuted = !failed && this.#state.notice === SERVER_MUTED_NOTICE;
    let notice: string | null = null;

    if (failed) {
      notice = message;
    } else if (keepMuted) {
      notice = SERVER_MUTED_NOTICE;
    }

    this.#statusRevision += 1;

    const live = livePhase(phase);

    this.#set({
      phase,
      status: failed ? failureTitle : message,
      failureTitle,
      notice,
      devicesOpen: live ? this.#state.devicesOpen : false,
      statsOpen: live ? this.#state.statsOpen : false,
      prejoin: phase === "prejoin" ? this.#state.prejoin : null,
      startedAt: live && this.#state.startedAt === null ? Date.now() : this.#state.startedAt,
    });

    if (phase !== "reconnecting") {
      this.#stopReconnectCountdown();
    }

    if (!live) {
      this.#stopStats();
    }
  }

  /** A four-second status line that falls back to "Huddle active" while connected. */
  #flash(message: string): void {
    const phase = this.#state.phase;

    this.#statusRevision += 1;

    const revision = this.#statusRevision;

    this.#set({ status: message });
    setTimeout(() => {
      if (
        revision === this.#statusRevision &&
        this.#state.phase === phase &&
        phase === "connected"
      ) {
        this.#set({ status: "Huddle active" });
      }
    }, STATUS_RESET_MS);
  }

  #showNotice(message: string): void {
    this.#set({ notice: message });
  }

  #clearNotice(message: string | null = null): void {
    if (message === null || this.#state.notice === message) {
      this.#set({ notice: null });
    }
  }

  #refreshSnapshot(): void {
    const snapshot = this.#transport?.snapshot() ?? EMPTY_SNAPSHOT;
    const key = snapshotKey(snapshot);

    if (key === this.#lastSnapshotKey) {
      return;
    }

    this.#lastSnapshotKey = key;
    this.#set({ snapshot });
    this.#autoExpandStream();
  }

  // ── Joining ───────────────────────────────────────────────────────────────────────────────

  /**
   * Joins a room's call. A browser that has never granted microphone access stops at the device
   * check first; returning people skip it, and so does a denied permission (the denial surfaces
   * through the usual failure). A stage listener needs no device, so a false hint connects
   * straight away; the token stays the authority on publishing.
   */
  async join(roomId: number, roomName: string, canPublishHint: boolean | null): Promise<void> {
    if (!Number.isInteger(roomId) || roomId <= 0) {
      return;
    }

    const state = this.#state;

    if (state.roomId === roomId && activePhase(state.phase)) {
      this.#set({ viewOpen: true });

      return;
    }

    this.#operation += 1;

    const operation = this.#operation;
    // Switching calls leaves the old one, reported as `leave()` reports it.
    const previous = state.roomId !== null && state.phase !== "idle" ? state.roomId : null;

    await this.#disconnectCurrent();

    if (previous !== null) {
      void this.#env.leave(previous);
    }

    if (operation !== this.#operation) {
      return;
    }

    this.#canPublishHint = canPublishHint;
    this.#pushToTalkActive = false;
    this.#pushToTalkOpenedMic = false;

    // Storage is the preference: a transient processor failure only turned it off in memory,
    // so a fresh join gets a fresh attempt.
    const available = state.noise.available || (await this.#noiseSupported());

    this.#set({
      roomId,
      roomName,
      identity: null,
      canPublish: true,
      deafened: false,
      expandedVideoId: null,
      viewOpen: true,
      // A retry in the same room keeps its timer, as a rejoin does.
      startedAt: state.roomId === roomId ? state.startedAt : null,
      localMutes: [],
      noise: { available, enabled: available && loadNoiseSuppression(), busy: false },
    });
    this.#holdRoom(roomId);

    if (canPublishHint !== false && (await this.#env.shouldCheckDevices())) {
      if (operation !== this.#operation) {
        return;
      }

      await this.#enterPrejoin(operation);

      return;
    }

    if (operation !== this.#operation) {
      return;
    }

    await this.#connect(operation);
  }

  async #noiseSupported(): Promise<boolean> {
    const { noiseSuppressionSupported } = await import("./engine/noise.ts");

    return noiseSuppressionSupported();
  }

  async #connect(operation: number): Promise<void> {
    const roomId = this.#state.roomId;

    if (roomId === null) {
      return;
    }

    this.#setPhase("connecting", `Connecting to ${this.#state.roomName}…`);

    let transport: CallTransport | null = null;

    try {
      const [credentials] = await Promise.all([this.#env.join(roomId), this.#env.preload()]);

      if (operation !== this.#operation) {
        return;
      }

      transport = await this.#env.transport(credentials.url, roomId);

      if (operation !== this.#operation) {
        await transport.disconnect();

        return;
      }

      this.#set({
        roomName: credentials.roomName === "" ? this.#state.roomName : credentials.roomName,
        identity: credentials.identity,
        canPublish: credentials.canPublish,
      });
      this.#adopt(transport);

      const noise = this.#state.noise;

      await transport.connect(credentials.url, credentials.token, noise.available && noise.enabled);

      if (operation !== this.#operation || transport !== this.#transport) {
        await this.#drop(transport);

        return;
      }

      await transport.startAudio().catch(() => undefined);

      // A listener's token can't publish, so LiveKit would reject the microphone outright:
      // they join subscribe-only. Push-to-talk joins with the microphone closed.
      if (credentials.canPublish) {
        await transport.setMicrophone(!this.#pushToTalk());
      }

      if (operation !== this.#operation || transport !== this.#transport) {
        await this.#drop(transport);

        return;
      }

      this.#setPhase("connected", "Huddle active");
      this.#set({ quality: "unknown" });
      this.#refreshSnapshot();
      this.#startAuthChecks();
      this.#startMeters();
      void this.#applyStoredOutput(transport);
      void this.#refreshDevices();
      void this.#refreshIdentities();

      // Noise suppression applies once the call is usable: a processor that can't start must
      // never keep somebody out of the conversation.
      if (credentials.canPublish) {
        void transport.setNoiseSuppression(noise.available && noise.enabled);
      }
    } catch (caught) {
      if (operation !== this.#operation) {
        if (transport !== null) {
          await this.#drop(transport);
        }

        return;
      }

      await this.#disconnectCurrent();
      const error = caught instanceof Error ? caught : new Error(String(caught));

      this.#setPhase("failed", joinErrorMessage(error), "Couldn’t join huddle");
    }
  }

  /** Retry after a failure, with the same hint. */
  retry(): void {
    const { roomId, roomName } = this.#state;

    if (roomId !== null) {
      void this.join(roomId, roomName, this.#canPublishHint);
    }
  }

  /** Drops a struggling connection and joins fresh: the same path a role change takes. */
  async reconnectNow(): Promise<void> {
    const roomId = this.#state.roomId;

    if (roomId === null) {
      return;
    }

    this.#stopReconnectCountdown();
    await this.#rejoin(roomId);
  }

  /**
   * A role change or a mute revokes the old grant, so the call rejoins the same room for a fresh
   * token instead of updating LiveKit permissions in place, skipping the device check. A failed
   * dock rejoins too, so a demotion the gateway enforced first still lands the member back in
   * the call as a listener. The hint follows the new role (hosts and speakers publish, listeners
   * don't, the server-muted don't), and a mute says why.
   */
  async roleChanged(
    roomId: number,
    stageRole: StageRole | null,
    serverMuted: boolean,
  ): Promise<void> {
    if (roomId === this.#state.roomId) {
      this.#canPublishHint = stageRole !== "listener" && !serverMuted;

      if (serverMuted) {
        this.#showNotice(SERVER_MUTED_NOTICE);
      } else {
        this.#clearNotice(SERVER_MUTED_NOTICE);
      }
    }

    await this.#rejoin(roomId);
  }

  async #rejoin(roomId: number): Promise<void> {
    const state = this.#state;

    if (state.roomId !== roomId || !(activePhase(state.phase) || state.phase === "failed")) {
      return;
    }

    this.#operation += 1;

    const operation = this.#operation;

    await this.#disconnectCurrent();

    if (operation !== this.#operation) {
      return;
    }

    await this.#connect(operation);
  }

  // ── The device check ──────────────────────────────────────────────────────────────────────

  /** Runs on local getUserMedia streams only: nothing is requested or published until Join. */
  async #enterPrejoin(operation: number): Promise<void> {
    this.#set({
      prejoin: {
        selected: loadDevicePreferences(),
        error: null,
        canJoin: false,
        retry: false,
        preview: null,
      },
    });
    this.#setPhase("prejoin", "Check your devices");
    await this.#refreshDevices();

    if (operation !== this.#operation || this.#state.phase !== "prejoin") {
      return;
    }

    this.#startMeters();
    await this.#startPreview();
  }

  async confirmPrejoin(): Promise<void> {
    const state = this.#state;

    if (state.phase !== "prejoin" || state.prejoin?.canJoin !== true) {
      return;
    }

    const operation = this.#operation;
    const selected = state.prejoin.selected;

    for (const kind of DEVICE_KINDS) {
      storeDevicePreference(kind, selected[kind]);
    }

    this.#stopPreview();
    await this.#connect(operation);
  }

  async retryPrejoin(): Promise<void> {
    if (this.#state.phase === "prejoin") {
      await this.#startPreview();
    }
  }

  async #startPreview(): Promise<void> {
    this.#previewOperation += 1;
    // The camera preview restarts after the microphone's: one still opening is stale.
    this.#previewVideoOperation += 1;

    const preview = this.#previewOperation;

    this.#stopPreviewStreams();
    this.#setPrejoin({ error: null, canJoin: false, retry: false });

    const track = await this.#acquirePreviewAudio(preview);

    if (preview !== this.#previewOperation || this.#state.phase !== "prejoin") {
      track?.stop();

      return;
    }

    if (track === null) {
      return;
    }

    this.#previewMeter = createLevelMeter(track);
    this.#setPrejoin({ canJoin: true });
    await this.#startPreviewVideo();
  }

  async #acquirePreviewAudio(preview: number): Promise<MediaStreamTrack | null> {
    // The pickers are authoritative here, not storage. The preview keeps the browser's own
    // suppression: no processor runs before the call.
    const selected = this.#state.prejoin?.selected.audioinput ?? "";

    const audio: MediaTrackConstraints = {
      autoGainControl: true,
      echoCancellation: true,
      noiseSuppression: true,
    };

    if (selected !== "") {
      audio.deviceId = { exact: selected };
    }

    try {
      const stream = await this.#env.getUserMedia({ audio, video: false });
      const track = stream.getAudioTracks()[0];

      if (
        preview !== this.#previewOperation ||
        this.#state.phase !== "prejoin" ||
        track === undefined
      ) {
        stopStream(stream);

        return null;
      }

      this.#previewAudio = stream;

      return track;
    } catch (caught) {
      if (preview !== this.#previewOperation || this.#state.phase !== "prejoin") {
        return null;
      }

      // A refresh covers a device that vanished between listing and capture.
      await this.#refreshDevices();
      this.#setPrejoin({
        error:
          caught instanceof Error && permissionDenied(caught)
            ? "Microphone access was denied. Allow microphone access and try again. You are not connected."
            : "The microphone could not be started. Check your device and try again.",
        retry: true,
      });

      return null;
    }
  }

  async #startPreviewVideo(): Promise<void> {
    this.#previewVideoOperation += 1;

    const preview = this.#previewVideoOperation;

    this.#stopPreviewVideo();

    const deviceId = this.#state.prejoin?.selected.videoinput ?? "";

    if (deviceId === "") {
      return;
    }

    try {
      const stream = await this.#env.getUserMedia({
        video: { deviceId: { exact: deviceId } },
        audio: false,
      });

      if (preview !== this.#previewVideoOperation || this.#state.phase !== "prejoin") {
        stopStream(stream);

        return;
      }

      this.#setPrejoin({ preview: stream });
    } catch {
      if (preview !== this.#previewVideoOperation || this.#state.phase !== "prejoin") {
        return;
      }

      // A camera that won't preview must not block joining audio-only.
      await this.#refreshDevices();
      this.#setPrejoin({
        error: "The camera preview could not be started. You can still join with your microphone.",
      });
    }
  }

  #stopPreview(): void {
    this.#previewOperation += 1;
    this.#previewVideoOperation += 1;
    this.#stopPreviewStreams();
    this.#set({ meter: 0 });
  }

  #stopPreviewStreams(): void {
    this.#previewMeter?.stop();
    this.#previewMeter = null;
    stopStream(this.#previewAudio);
    this.#previewAudio = null;
    this.#stopPreviewVideo();
  }

  #stopPreviewVideo(): void {
    const preview = this.#state.prejoin?.preview ?? null;

    if (preview !== null) {
      stopStream(preview);
      this.#setPrejoin({ preview: null });
    }
  }

  // ── Devices ───────────────────────────────────────────────────────────────────────────────

  async #refreshDevices(): Promise<void> {
    let devices: DeviceLists;

    try {
      devices = await this.#env.listDevices();
    } catch {
      return;
    }

    const preferences = loadDevicePreferences();
    const prejoin = this.#state.prejoin;
    const current: DevicePreferences = prejoin?.selected ?? this.#state.selectedDevices;

    const choose = (kind: DeviceKind): string =>
      pick(devices[kind], this.#transport?.activeDevice(kind) ?? preferences[kind], current[kind]);

    const selected: DevicePreferences = {
      audioinput: choose("audioinput"),
      audiooutput: choose("audiooutput"),
      videoinput: choose("videoinput"),
    };

    this.#set({ devices, selectedDevices: selected });

    if (prejoin !== null && this.#state.prejoin !== null) {
      this.#setPrejoin({ selected });
    }
  }

  /** The OS device list changed: keep the pickers truthful, and re-preview a vanished choice. */
  devicesChanged(): void {
    const state = this.#state;

    if (state.phase !== "prejoin" && !state.devicesOpen) {
      return;
    }

    const before = state.prejoin?.selected ?? null;

    void this.#refreshDevices().then(() => {
      const after = this.#state.prejoin?.selected ?? null;

      if (
        this.#state.phase === "prejoin" &&
        before !== null &&
        after !== null &&
        (before.audioinput !== after.audioinput || before.videoinput !== after.videoinput)
      ) {
        void this.#startPreview();
      }
    });
  }

  toggleDevices(): void {
    if (!livePhase(this.#state.phase)) {
      return;
    }

    const open = !this.#state.devicesOpen;

    this.#set({ devicesOpen: open });

    if (open) {
      void this.#refreshDevices();
    }
  }

  /** A picker changed. The choice is remembered (only ever from the person's own choice). */
  async selectDevice(kind: DeviceKind, deviceId: string): Promise<void> {
    storeDevicePreference(kind, deviceId);

    const state = this.#state;

    if (state.phase === "prejoin" && state.prejoin !== null) {
      this.#setPrejoin({ selected: { ...state.prejoin.selected, [kind]: deviceId } });

      if (kind === "audioinput") {
        await this.#startPreview();
      } else if (kind === "videoinput") {
        await this.#startPreviewVideo();
      }

      return;
    }

    const transport = this.#transport;

    if (transport === null || state.phase !== "connected") {
      return;
    }

    if (deviceId === "") {
      await this.#refreshDevices();

      return;
    }

    this.#set({ selectedDevices: { ...state.selectedDevices, [kind]: deviceId } });

    const switched = await transport.switchDevice(kind, deviceId);

    if (transport !== this.#transport) {
      return;
    }

    if (!switched) {
      this.#flash(`The ${DEVICE_NAMES[kind]} could not be switched. Try again.`);
      await this.#refreshDevices();

      return;
    }

    this.#refreshSnapshot();
  }

  async #applyStoredOutput(transport: CallTransport): Promise<void> {
    if (!("setSinkId" in HTMLMediaElement.prototype)) {
      return;
    }

    const deviceId = loadDevicePreferences().audiooutput;

    if (deviceId === "") {
      return;
    }

    try {
      const { audiooutput } = await this.#env.listDevices();

      if (audiooutput.some((device) => device.deviceId === deviceId)) {
        await transport.switchDevice("audiooutput", deviceId);
      }
    } catch {
      // Output selection is a preference, never a reason to fail a join.
    }
  }

  // ── Leaving and ending ────────────────────────────────────────────────────────────────────

  /**
   * Leaves, then reports it: the gateway checks connected participants about once a second, and
   * a check landing between an early report and the disconnect would mark the grant seen again.
   * The report is fire-and-forget, so leaving works offline.
   */
  async leave(): Promise<void> {
    const roomId = this.#state.roomId;

    this.#operation += 1;
    await this.#disconnectCurrent();

    if (roomId !== null) {
      void this.#env.leave(roomId);
    }

    this.#pushToTalkActive = false;
    this.#pushToTalkOpenedMic = false;
    this.#holdRoom(null);
    callStore.setState({
      ...initialCallState,
      noise: this.#state.noise,
      devices: this.#state.devices,
      selectedDevices: this.#state.selectedDevices,
    });
  }

  /** The page is going away, or another account signed in: end without waiting. */
  endForPageChange(): void {
    const roomId = this.#state.roomId;
    // Mid-connect the grant may already exist: report the leave then too.
    const live = this.#transport !== null || this.#state.phase === "connecting";

    this.#operation += 1;
    this.#endStreamOnDisconnect(true);
    this.#stopTimers();
    this.#stopPreview();

    const transport = this.#transport;

    this.#transport = null;
    this.#unsubscribeTransport?.();
    this.#unsubscribeTransport = null;
    void transport?.disconnect();

    if (roomId !== null && live) {
      this.#env.leaveOnUnload(roomId);
    }

    this.#pushToTalkActive = false;
    this.#pushToTalkOpenedMic = false;
    this.#holdRoom(null);
    callStore.setState({ ...initialCallState, noise: this.#state.noise });
  }

  async #endForAccess(tag: string | null): Promise<void> {
    this.#operation += 1;
    await this.#disconnectCurrent();
    this.#setPhase(
      "failed",
      tag === "Forbidden" || tag === "NotFound"
        ? "Your access to this room ended."
        : SIGN_IN_EXPIRED,
      "Huddle ended",
    );
  }

  #adopt(transport: CallTransport): void {
    this.#transport = transport;
    this.#lastSnapshotKey = "";
    this.#unsubscribeTransport = transport.subscribe((event) => {
      this.#transportEvent(transport, event);
    });
  }

  async #drop(transport: CallTransport): Promise<void> {
    if (transport === this.#transport) {
      this.#transport = null;
      this.#unsubscribeTransport?.();
      this.#unsubscribeTransport = null;
    }

    await transport.disconnect();
  }

  async #disconnectCurrent(): Promise<void> {
    const transport = this.#transport;

    this.#transport = null;
    this.#unsubscribeTransport?.();
    this.#unsubscribeTransport = null;
    this.#pushToTalkActive = false;
    this.#pushToTalkOpenedMic = false;
    this.#endStreamOnDisconnect(false);
    this.#stopTimers();
    this.#stopPreview();

    if (transport !== null) {
      await transport.disconnect();
    }

    this.#lastSnapshotKey = "";
    this.#autoExpanded = null;
    // A toggle still running on the old transport skips its own cleanup (it checks the
    // transport), so its busy flag ends here, or the control would stay dead on the new one.
    this.#set({
      snapshot: EMPTY_SNAPSHOT,
      expandedVideoId: null,
      meter: 0,
      stats: null,
      busy: initialCallState.busy,
    });
  }

  #stopTimers(): void {
    this.#stopAuthChecks();
    this.#stopMeters();
    this.#stopStats();
    this.#stopReconnectCountdown();
  }

  // ── Transport events ──────────────────────────────────────────────────────────────────────

  #transportEvent(transport: CallTransport, event: TransportEvent): void {
    if (transport !== this.#transport) {
      return;
    }

    switch (event.type) {
      case "changed":
        this.#refreshSnapshot();

        return;
      case "reconnecting":
        this.#setPhase("reconnecting", "Connection interrupted. Reconnecting…");
        this.#startReconnectCountdown();
        void this.#checkAccess();

        return;
      case "reconnected":
        this.#setPhase("connected", "Huddle active");
        this.#refreshSnapshot();

        return;
      case "disconnected":
        void this.#unexpectedDisconnect(transport, event.reason);

        return;
      case "quality":
        this.#set({ quality: event.quality });

        return;
      case "device":
        // The SDK retargeted a track (the chosen device vanished): the pickers follow it.
        void this.#refreshDevices();

        return;
      case "screen-ended":
        this.#streamShareEnded();
        this.#refreshSnapshot();

        return;
      case "noise":
        this.#noiseOutcome(event.outcome);

        return;
    }
  }

  async #unexpectedDisconnect(transport: CallTransport, reason: DisconnectReason): Promise<void> {
    this.#operation += 1;
    this.#transport = null;
    this.#unsubscribeTransport?.();
    this.#unsubscribeTransport = null;
    this.#pushToTalkActive = false;
    this.#pushToTalkOpenedMic = false;
    this.#stopTimers();
    this.#stopPreview();
    void transport.disconnect();
    this.#lastSnapshotKey = "";
    this.#set({ snapshot: EMPTY_SNAPSHOT, expandedVideoId: null, meter: 0 });
    this.#setPhase("failed", disconnectMessage(reason), "Huddle ended");
  }

  // ── The reconnect countdown ───────────────────────────────────────────────────────────────

  /** The SDK owns reconnection and may still recover, so expiry never forces a failure. */
  #startReconnectCountdown(): void {
    this.#stopReconnectCountdown();

    let remaining = RECONNECT_COUNTDOWN_SECONDS;

    this.#set({ reconnectSeconds: remaining });
    this.#reconnectTimer = setInterval(() => {
      remaining -= 1;

      if (remaining <= 0 || this.#state.phase !== "reconnecting") {
        this.#clearReconnectTimer();
        this.#set({ reconnectSeconds: this.#state.phase === "reconnecting" ? 0 : null });

        return;
      }

      this.#set({ reconnectSeconds: remaining });
    }, 1000);
  }

  #clearReconnectTimer(): void {
    if (this.#reconnectTimer !== null) {
      clearInterval(this.#reconnectTimer);
      this.#reconnectTimer = null;
    }
  }

  #stopReconnectCountdown(): void {
    this.#clearReconnectTimer();

    if (this.#state.reconnectSeconds !== null) {
      this.#set({ reconnectSeconds: null });
    }
  }

  // ── The access check ──────────────────────────────────────────────────────────────────────

  #startAuthChecks(): void {
    if (this.#transport === null || this.#authTimer !== null) {
      return;
    }

    this.#authTimer = setInterval(() => {
      void this.#checkAccess();
    }, AUTH_CHECK_INTERVAL_MS);
  }

  #stopAuthChecks(): void {
    if (this.#authTimer !== null) {
      clearInterval(this.#authTimer);
      this.#authTimer = null;
    }

    this.#authCheck = null;
  }

  /**
   * `GET /rooms/:id/huddle`: a 401, 403 or 404 ends the call at once. Inside a call the check
   * runs hidden too, so a revoked background tab ends instead of lingering. A failed request
   * alone isn't proof of anything (LiveKit owns reconnection).
   */
  #checkAccess(): Promise<void> | null {
    const transport = this.#transport;
    const roomId = this.#state.roomId;

    if (transport === null || roomId === null || this.#authCheck !== null) {
      return this.#authCheck;
    }

    if (document.visibilityState === "hidden" && !livePhase(this.#state.phase)) {
      return null;
    }

    const check = this.#env
      .check(roomId)
      .then(() => {
        this.#applyStoredAudio();
      })
      .catch((caught: Error) => {
        const tag = errorTag(caught);

        if (
          transport === this.#transport &&
          (tag === "Unauthorized" || tag === "Forbidden" || tag === "NotFound")
        ) {
          void this.#endForAccess(tag);
        }
      })
      .finally(() => {
        if (this.#authCheck === check) {
          this.#authCheck = null;
        }
      });

    this.#authCheck = check;

    return check;
  }

  /** The tab became visible again: check access straight away. */
  visible(): void {
    void this.#checkAccess();
  }

  /** The tab was hidden: a hidden page drops keyups, so a held push-to-talk key is released. */
  hidden(): void {
    this.#releasePushToTalk();
  }

  // ── Who's who: LiveKit identities to people ───────────────────────────────────────────────

  /** The presence stacks map identities to user ids; per-person audio follows the person. */
  async #refreshIdentities(): Promise<void> {
    await this.#checkAccess();
    this.#applyStoredAudio();
  }

  /** The user id behind a LiveKit identity, from the room's presence. */
  userIdFor(identity: string): number | null {
    const roomId = this.#state.roomId;

    return roomId === null ? null : userIdForIdentity(store.getState().huddles[roomId], identity);
  }

  #identitiesOf(userId: number): string[] {
    const roomId = this.#state.roomId;
    const own = this.#state.identity;
    const presence = roomId === null ? undefined : store.getState().huddles[roomId];
    const participant = presence?.participants.find((entry) => entry.userId === userId);

    return (participant?.identities ?? []).filter((identity) => identity !== own);
  }

  /** Remembered volumes and local mutes; untouched people keep the SDK's own audio path. */
  #applyStoredAudio(): void {
    const transport = this.#transport;
    const roomId = this.#state.roomId;

    if (transport === null || roomId === null) {
      return;
    }

    const participants = store.getState().huddles[roomId]?.participants ?? [];

    this.#setLocalMutes(
      participants
        .filter((participant) => loadParticipantMuted(participant.userId))
        .map((participant) => participant.userId),
    );

    for (const participant of participants) {
      const volume = loadParticipantVolume(participant.userId);
      const muted = loadParticipantMuted(participant.userId);

      if (volume !== 100 || muted) {
        for (const identity of this.#identitiesOf(participant.userId)) {
          transport.setParticipantVolume(identity, volume);
          transport.setParticipantMuted(identity, muted);
        }
      }
    }
  }

  /** The room's presence changed (someone joined, or a tab of theirs did). */
  presenceChanged(): void {
    this.#applyStoredAudio();
    this.#refreshSnapshot();
  }

  /** Someone's volume for this browser only, 0–200, remembered per person. */
  setParticipantVolume(userId: number, volume: number): void {
    const value = Math.max(0, Math.min(200, Math.round(volume)));
    const transport = this.#transport;

    storeParticipantVolume(userId, value);

    if (transport === null) {
      return;
    }

    for (const identity of this.#identitiesOf(userId)) {
      transport.setParticipantVolume(identity, value);
    }

    this.#refreshSnapshot();
  }

  /** Stops (or resumes) hearing someone, for this browser only. */
  toggleParticipantMute(userId: number): void {
    const muted = !loadParticipantMuted(userId);
    const transport = this.#transport;

    storeParticipantMuted(userId, muted);

    for (const identity of this.#identitiesOf(userId)) {
      transport?.setParticipantMuted(identity, muted);
    }

    const others = this.#state.localMutes.filter((id) => id !== userId);

    this.#setLocalMutes(muted ? [...others, userId] : others);
  }

  /** The "muted for me" marks, kept in the store so tiles and menus follow them. */
  #setLocalMutes(userIds: readonly number[]): void {
    const next = userIds.toSorted((a, b) => a - b);
    const current = this.#state.localMutes;

    if (next.length !== current.length || next.some((id, index) => id !== current[index])) {
      this.#set({ localMutes: next });
    }
  }

  // ── Microphone, push-to-talk and deafen ───────────────────────────────────────────────────

  async toggleMute(): Promise<void> {
    const transport = this.#transport;
    const state = this.#state;

    if (
      transport === null ||
      state.phase !== "connected" ||
      !state.canPublish ||
      state.busy.microphone
    ) {
      return;
    }

    const enabling = !state.snapshot.microphoneEnabled;

    this.#setBusy("microphone", true);

    try {
      // Speaking again while deafened undeafens, as in Slack and Discord.
      if (enabling && state.deafened) {
        this.#microphoneBeforeDeafen = false;
        this.#set({ deafened: false });
        transport.setDeafened(false);
      }

      await transport.setMicrophone(enabling);

      if (transport === this.#transport) {
        this.#refreshSnapshot();

        const noise = this.#state.noise;

        void transport.setNoiseSuppression(noise.available && noise.enabled);
      }
    } catch {
      if (transport === this.#transport) {
        this.#flash("The microphone could not be changed.");
      }
    } finally {
      if (transport === this.#transport) {
        this.#setBusy("microphone", false);
      }
    }
  }

  /** Silences everyone for this browser and closes the microphone; undeafening reopens it. */
  async toggleDeafen(): Promise<void> {
    const transport = this.#transport;
    const state = this.#state;

    if (transport === null || !livePhase(state.phase)) {
      return;
    }

    const deafened = !state.deafened;

    this.#set({ deafened });
    transport.setDeafened(deafened);

    if (!state.canPublish) {
      return;
    }

    try {
      if (deafened) {
        this.#microphoneBeforeDeafen = state.snapshot.microphoneEnabled;

        if (state.snapshot.microphoneEnabled) {
          await transport.setMicrophone(false);
        }
      } else if (this.#microphoneBeforeDeafen) {
        this.#microphoneBeforeDeafen = false;
        await transport.setMicrophone(true);
      }
    } catch {
      this.#flash("The microphone could not be changed.");
    }

    this.#refreshSnapshot();
  }

  /**
   * Call shortcuts, active anywhere while in a call: Ctrl/Cmd+Shift+M toggles the microphone,
   * and in push-to-talk mode holding the key opens it. The push-to-talk key never fires while
   * typing or composing, or with Ctrl, Meta or Alt held; the mute chord is global because it
   * can't be typed by accident. The default backtick is a dead key on international layouts, so
   * it matches by physical position (`Backquote`); custom keys match the typed character.
   */
  keyDown(event: KeyboardEvent): void {
    if (event.repeat || event.defaultPrevented) {
      return;
    }

    const state = this.#state;
    const ready = state.phase === "connected" && this.#transport !== null && state.canPublish;

    if (
      (event.ctrlKey || event.metaKey) &&
      event.shiftKey &&
      !event.altKey &&
      event.key.toLowerCase() === "m"
    ) {
      if (ready) {
        event.preventDefault();
        void this.toggleMute();
      }

      return;
    }

    if (
      !this.#pushToTalkMatches(event) ||
      event.isComposing ||
      event.ctrlKey ||
      event.metaKey ||
      event.altKey
    ) {
      return;
    }

    if (
      !this.#pushToTalk() ||
      this.#pushToTalkActive ||
      !ready ||
      state.deafened ||
      typingTarget(event.target)
    ) {
      return;
    }

    event.preventDefault();
    this.#pushToTalkActive = true;
    this.#pushToTalkOpenedMic = !state.snapshot.microphoneEnabled;

    const transport = this.#transport;

    transport
      ?.setMicrophone(true)
      .then(() => {
        if (transport === this.#transport) {
          this.#refreshSnapshot();
        }
      })
      .catch(() => {
        this.#pushToTalkActive = false;
        this.#pushToTalkOpenedMic = false;
      });
  }

  keyUp(event: KeyboardEvent): void {
    if (this.#pushToTalkMatches(event)) {
      this.#releasePushToTalk();
    }
  }

  /** Losing the window releases a held key without its keyup (no mic wedged open). */
  blurred(): void {
    this.#releasePushToTalk();
  }

  #pushToTalkMatches(event: KeyboardEvent): boolean {
    const key = this.#pushToTalkKey();

    return event.key === key || (key === "`" && event.code === "Backquote");
  }

  /** A microphone opened by hand stays open; only a hold that opened it closes it again. */
  #releasePushToTalk(): void {
    if (!this.#pushToTalkActive) {
      return;
    }

    this.#pushToTalkActive = false;

    const opened = this.#pushToTalkOpenedMic;
    const transport = this.#transport;

    this.#pushToTalkOpenedMic = false;

    if (transport === null || !opened) {
      return;
    }

    transport
      .setMicrophone(false)
      .then(() => {
        if (transport === this.#transport) {
          this.#refreshSnapshot();
        }
      })
      .catch(() => undefined);
  }

  // ── Noise suppression ─────────────────────────────────────────────────────────────────────

  async toggleNoise(): Promise<void> {
    const state = this.#state;

    if (!state.noise.available || state.noise.busy) {
      return;
    }

    const requested = !state.noise.enabled;

    storeNoiseSuppression(requested);
    this.#set({ noise: { ...state.noise, enabled: requested } });

    const transport = this.#transport;

    if (transport === null || state.phase !== "connected") {
      return;
    }

    this.#set({ noise: { ...this.#state.noise, busy: true } });
    await transport.setNoiseSuppression(requested);
    this.#set({ noise: { ...this.#state.noise, busy: false } });

    // A failure has already explained itself; don't talk over it.
    const noise = this.#state.noise;

    if (transport === this.#transport && noise.available && noise.enabled === requested) {
      this.#flash(
        requested
          ? "Noise suppression on"
          : "Noise suppression off. Your browser’s basic filtering stays on.",
      );
    }
  }

  #noiseOutcome(outcome: NoiseOutcome): void {
    const noise = this.#state.noise;

    if (outcome.failure === "unsupported") {
      this.#set({ noise: { ...noise, available: false } });
      this.#flash(
        "Extra noise suppression isn’t available in this browser. Basic filtering is still on.",
      );
    } else if (outcome.failure === "failed") {
      // A worklet or model that failed to load may well load next time: nothing is stored.
      this.#set({ noise: { ...noise, enabled: false } });
      this.#flash("Noise suppression couldn’t start. Basic filtering is still on — try again.");
    }

    if (outcome.microphoneLost) {
      this.#showNotice(MICROPHONE_RESTART_ERROR);
    } else if (outcome.settled) {
      this.#clearNotice(MICROPHONE_RESTART_ERROR);
    }
  }

  // ── Camera and screen share ───────────────────────────────────────────────────────────────

  async toggleCamera(): Promise<void> {
    const transport = this.#transport;
    const state = this.#state;

    if (
      transport === null ||
      state.phase !== "connected" ||
      !state.canPublish ||
      state.busy.camera
    ) {
      return;
    }

    const enabling = !state.snapshot.cameraEnabled;

    this.#setBusy("camera", true);

    try {
      await transport.setCamera(enabling);

      if (transport !== this.#transport) {
        await transport.setCamera(false).catch(() => undefined);

        return;
      }

      this.#refreshSnapshot();

      if (enabling) {
        this.#clearNotice();
      }

      this.#flash(enabling ? "Your camera is on" : "Camera off");
    } catch (caught) {
      if (transport === this.#transport) {
        const message =
          caught instanceof Error && permissionDenied(caught)
            ? "Camera wasn’t started. Allow camera access to try again."
            : "Camera could not be changed. Try again.";

        // A camera that fails to start leaves the call connected, so the failure stays up
        // instead of fading with the status line.
        if (enabling) {
          this.#showNotice(message);
        } else {
          this.#flash(message);
        }

        this.#refreshSnapshot();
      }
    } finally {
      if (transport === this.#transport) {
        this.#setBusy("camera", false);
      }
    }
  }

  async toggleScreenShare(): Promise<void> {
    const transport = this.#transport;
    const state = this.#state;

    if (
      transport === null ||
      state.phase !== "connected" ||
      !state.canPublish ||
      state.busy.screen
    ) {
      return;
    }

    const enabling = !state.snapshot.screenSharing;

    this.#setBusy("screen", true);

    try {
      await transport.setScreenShare(enabling);

      if (transport !== this.#transport) {
        await transport.setScreenShare(false).catch(() => undefined);

        return;
      }

      this.#refreshSnapshot();
      this.#flash(enabling ? "You’re sharing your screen" : "Screen sharing stopped");
    } catch (caught) {
      if (transport === this.#transport) {
        this.#flash(
          caught instanceof Error && permissionDenied(caught)
            ? "Screen sharing wasn’t started. Choose a screen and allow sharing to try again."
            : "Screen sharing could not be changed. Try again.",
        );
        this.#refreshSnapshot();
      }
    } finally {
      if (transport === this.#transport) {
        this.#setBusy("screen", false);
      }
    }
  }

  /**
   * Go live. Call this synchronously from the click: everything before the capture runs inside
   * the gesture (Safari denies a getDisplayMedia that starts after the POST round-trip). Then
   * the stream is posted and the captured tracks publish at its quality. Any failure stops the
   * tracks, and one after the POST also ends the posted stream.
   */
  async goLive(roomId: number, quality: StreamQuality): Promise<void> {
    const transport = this.#transport;
    const state = this.#state;

    // Presenting needs the call: nothing posts and no picker opens without it.
    if (
      roomId !== state.roomId ||
      transport === null ||
      state.phase !== "connected" ||
      !state.canPublish
    ) {
      this.#flash("Join the stage before going live.");

      return;
    }

    if (!canShareScreen()) {
      this.#flash("Screen sharing isn’t available in this browser.");

      return;
    }

    const operation = this.#operation;
    const capture = transport.captureScreen();
    let captured: Awaited<typeof capture>;

    try {
      captured = await capture;
    } catch (caught) {
      // Cancelled or denied before anything posted: nothing to unwind.
      if (transport === this.#transport) {
        this.#flash(
          caught instanceof Error && permissionDenied(caught)
            ? "Screen sharing wasn’t started. Choose a screen and allow sharing to try again."
            : "Screen sharing could not be started. Try again.",
        );
      }

      return;
    }

    if (operation !== this.#operation || transport !== this.#transport) {
      transport.discardScreen(captured);

      return;
    }

    let stream: StageStream;

    try {
      stream = await this.#env.startStream(roomId, quality);
    } catch (caught) {
      transport.discardScreen(captured);

      if (transport === this.#transport) {
        this.#flash(
          caught instanceof Error && caught.message !== ""
            ? caught.message
            : "Going live failed. Try again.",
        );
      }

      return;
    }

    if (operation !== this.#operation || transport !== this.#transport) {
      transport.discardScreen(captured);
      void this.#env.stopStream(roomId, stream.id).catch(() => undefined);

      return;
    }

    try {
      await transport.publishScreen(captured, quality);
    } catch {
      transport.discardScreen(captured);
      void this.#env.stopStream(roomId, stream.id).catch(() => undefined);

      if (transport === this.#transport) {
        this.#flash("Screen sharing could not be started. Try again.");
      }

      return;
    }

    this.#set({ streaming: { roomId, quality, streamId: stream.id } });
    this.#refreshSnapshot();
    this.#flash("You’re live");
  }

  /**
   * A host ended this tab's stream: the server state is already over, so this only stops the
   * share. The flag clears first, so the unpublish doesn't end a stream that's already gone.
   */
  async streamStopped(roomId: number): Promise<void> {
    const transport = this.#transport;

    if (this.#state.streaming?.roomId !== roomId || transport === null) {
      return;
    }

    this.#set({ streaming: null });
    await transport.setScreenShare(false).catch(() => undefined);
    this.#refreshSnapshot();
  }

  /**
   * The browser's own Stop control (or leaving) unpublished the share without touching the
   * stream: end it too, with its id, so a late end never kills someone's newer stream.
   */
  #streamShareEnded(): void {
    const streaming = this.#state.streaming;

    if (streaming === null || streaming.roomId !== this.#state.roomId) {
      return;
    }

    this.#set({ streaming: null });
    void this.#env.stopStream(streaming.roomId, streaming.streamId).catch(() => undefined);
  }

  /** Leaving, switching rooms and rejoining end the stream with the old connection. */
  #endStreamOnDisconnect(unloading: boolean): void {
    const streaming = this.#state.streaming;

    if (streaming === null) {
      return;
    }

    this.#set({ streaming: null });

    if (unloading) {
      this.#env.stopStreamOnUnload(streaming.roomId, streaming.streamId);
    } else {
      void this.#env.stopStream(streaming.roomId, streaming.streamId).catch(() => undefined);
    }
  }

  // ── Watching shares ───────────────────────────────────────────────────────────────────────

  /** The stage presenter's screen share, if they're in this call and sharing. */
  streamVideoId(): string | null {
    const state = this.#state;
    const roomId = state.roomId;

    if (roomId === null) {
      return null;
    }

    return streamVideoIdOf(
      state.snapshot.participants,
      state.streaming?.roomId === roomId,
      store.getState().stages[roomId]?.live?.identity ?? null,
    );
  }

  /**
   * A live stream expands itself for viewers when it arrives, unless something else is already
   * expanded (joining late takes the same path).
   */
  #autoExpandStream(): void {
    const videoId = this.streamVideoId();
    const state = this.#state;

    if (videoId === null || state.streaming !== null || videoId === this.#autoExpanded) {
      return;
    }

    this.#autoExpanded = videoId;
    this.#transport?.setViewerQuality(videoId, loadStreamQuality());

    if (state.expandedVideoId === null) {
      this.#set({ expandedVideoId: videoId, viewOpen: true });
    }
  }

  /** Expands a share (theater mode), or collapses with `null`. */
  expand(videoId: string | null): void {
    this.#set({ expandedVideoId: videoId });
  }

  /** The shown size of a share changed: ask for a matching layer (streams follow the viewer's choice). */
  screenShown(videoId: string, width: number, height: number, expanded: boolean): void {
    const transport = this.#transport;

    if (transport === null) {
      return;
    }

    if (videoId === this.streamVideoId()) {
      transport.setViewerQuality(videoId, loadStreamQuality());

      return;
    }

    transport.showScreen(videoId, expanded ? { width, height } : null);
  }

  /** The viewer's stream quality, remembered like the other call preferences. */
  setViewerQuality(quality: ViewerQuality): void {
    storeStreamQuality(quality);

    const videoId = this.streamVideoId();

    if (videoId !== null) {
      this.#transport?.setViewerQuality(videoId, quality);
    }
  }

  attachVideo(videoId: string, element: HTMLVideoElement): void {
    this.#transport?.attachVideo(videoId, element);
  }

  detachVideo(videoId: string, element: HTMLVideoElement): void {
    this.#transport?.detachVideo(videoId, element);
  }

  setViewOpen(open: boolean): void {
    this.#set({ viewOpen: open });
  }

  /** Lets audio play after an autoplay block (from a gesture). */
  async resumeAudio(): Promise<void> {
    await this.#transport?.startAudio().catch(() => undefined);
    this.#refreshSnapshot();
  }

  // ── Meters and statistics ─────────────────────────────────────────────────────────────────

  /** The meter (10 Hz) and the speaking levels (5 Hz), skipped while the tab is hidden. */
  #startMeters(): void {
    this.#stopMeters();
    this.#meterTimer = setInterval(() => {
      if (document.visibilityState === "hidden") {
        return;
      }

      const level =
        this.#state.phase === "prejoin"
          ? (this.#previewMeter?.level() ?? 0)
          : (this.#transport?.microphoneLevel() ?? 0);

      if (level !== this.#state.meter) {
        this.#set({ meter: level });
      }
    }, METER_INTERVAL_MS);
    this.#snapshotTimer = setInterval(() => {
      if (document.visibilityState !== "hidden" && this.#transport !== null) {
        this.#refreshSnapshot();
      }
    }, SNAPSHOT_INTERVAL_MS);
  }

  #stopMeters(): void {
    for (const timer of [this.#meterTimer, this.#snapshotTimer]) {
      if (timer !== null) {
        clearInterval(timer);
      }
    }

    this.#meterTimer = null;
    this.#snapshotTimer = null;
  }

  /** Statistics are sampled only while the panel is open, never in the background. */
  toggleStats(): void {
    if (!livePhase(this.#state.phase)) {
      return;
    }

    const open = !this.#state.statsOpen;

    this.#set({ statsOpen: open });

    if (!open) {
      this.#stopStats();

      return;
    }

    void this.#sampleStats(true);
    this.#statsTimer = setInterval(() => {
      void this.#sampleStats(false);
    }, STATS_INTERVAL_MS);
  }

  #stopStats(): void {
    if (this.#statsTimer !== null) {
      clearInterval(this.#statsTimer);
      this.#statsTimer = null;
    }

    this.#statsSampling = false;
  }

  async #sampleStats(fresh: boolean): Promise<void> {
    const transport = this.#transport;

    if (transport === null || this.#statsSampling || !this.#state.statsOpen) {
      return;
    }

    if (document.visibilityState === "hidden" || !livePhase(this.#state.phase)) {
      return;
    }

    this.#statsSampling = true;

    try {
      const stats = await transport.sampleStats(fresh);

      if (transport === this.#transport && this.#state.statsOpen && stats !== null) {
        this.#set({ stats });
      }
    } finally {
      this.#statsSampling = false;
    }
  }
}

export const callController = new CallController(browserEnvironment);
