/**
 * The real media layer: one livekit-client `Room`, loaded on first use, configured and driven the
 * way the classic `huddle_controller.js` drives it (room options, screen-share retries, the
 * camera unpublish, per-person volume with a Web Audio boost, RNNoise and its capture sync, the
 * level meter and the statistics sample). The call controller owns the flow; this owns the SDK.
 */
import type {
  AudioCaptureOptions,
  LocalAudioTrack,
  LocalTrack,
  Participant,
  RemoteParticipant,
  RemoteTrack,
  RemoteTrackPublication,
  Room,
  RoomOptions,
  ScreenShareCaptureOptions,
  TrackPublication,
  VideoEncoding,
} from "livekit-client";
import type { StreamQuality } from "../../../gen/StreamQuality.ts";
import { loadForUpdate } from "../../../service-worker/update-required.ts";
import { canShareScreen, permissionDenied } from "./devices.ts";
import { createLevelMeter, type LevelMeter } from "./meter.ts";
import {
  NoiseSuppressionUnsupported,
  NoiseSuppressor,
  noiseSuppressionSupported,
} from "./noise.ts";
import { loadDevicePreferences } from "./preferences.ts";
import { type StatsBaseline, summarizeConnectionStats } from "./stats.ts";
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
  type NoiseOutcome,
  type TransportEvent,
  type VideoSize,
  type ViewerQuality,
} from "./transport.ts";

type LiveKit = typeof import("livekit-client");

let liveKit: Promise<LiveKit> | null = null;

/** The SDK, imported once (it is large, and most sessions never join a call). */
export function loadLiveKit(): Promise<LiveKit> {
  liveKit ??= loadForUpdate(() => import("livekit-client")).catch((error: Error) => {
    liveKit = null;

    throw error;
  });

  return liveKit;
}

export async function createLiveKitTransport(): Promise<CallTransport> {
  return new LiveKitTransport(await loadLiveKit());
}

/** getDisplayMedia reports unusable constraints as NotSupportedError or TypeError. */
export function displayMediaRejectedConstraints(error: Error): boolean {
  return (
    !permissionDenied(error) && (error.name === "NotSupportedError" || error.name === "TypeError")
  );
}

/** An AudioContext that can pick its output device (newer than the element version). */
interface SinkableAudioContext extends AudioContext {
  setSinkId(sinkId: string): Promise<void>;
}

function sinkable(context: AudioContext): context is SinkableAudioContext {
  return "setSinkId" in context;
}

/**
 * Capture options for a screen share: tab audio only (capturing system audio while sharing a
 * whole screen feeds the speakers back into the call on Windows), no `resolution` (a preset's
 * resolution carries its frame rate and would cap capture too; the SDK fills in 1080p/30 and
 * skips it where it can't be constrained).
 */
function screenCaptureOptions(withAudio: boolean): ScreenShareCaptureOptions {
  return {
    contentHint: "detail",
    surfaceSwitching: "include",
    systemAudio: "exclude",
    audio: withAudio,
  };
}

/** A local or remote participant's quality, as the transport reports it. */
function quality(participant: Participant): ConnectionQuality {
  switch (participant.connectionQuality) {
    case "excellent":
    case "good":
    case "poor":
    case "lost":
      return participant.connectionQuality;
    default:
      return "unknown";
  }
}

class LiveKitTransport implements CallTransport {
  readonly #lk: LiveKit;
  #room: Room | null = null;
  readonly #listeners = new Set<(event: TransportEvent) => void>();
  readonly #unbind: (() => void)[] = [];
  /** Where remote audio elements live while subscribed. */
  #audioHost: HTMLDivElement | null = null;
  readonly #volumes = new Map<string, number>();
  readonly #locallyMuted = new Set<string>();
  #deafened = false;
  /** Identities whose audio currently routes through the boost gain. */
  readonly #boosted = new Set<string>();
  #boostContext: AudioContext | null = null;
  #meter: LevelMeter | null = null;
  #noiseAvailable = noiseSuppressionSupported();
  #noiseWanted = false;
  #noiseQueue: Promise<void> = Promise.resolve();
  #restartFailed = false;
  #microphoneLost = false;
  /** The microphone track the "ended" restore listens on (the SDK reuses it across restarts). */
  #watchedMicrophone: LocalAudioTrack | null = null;
  readonly #captured = new Map<number, LocalTrack[]>();
  #capturedSequence = 0;
  #statsBaseline: StatsBaseline | null = null;

  constructor(lk: LiveKit) {
    this.#lk = lk;
  }

  async connect(url: string, token: string, noiseSuppression: boolean): Promise<void> {
    this.#noiseWanted = noiseSuppression;

    const room = new this.#lk.Room(this.#roomOptions());

    this.#room = room;
    this.#bind(room);
    await room.connect(url, token, { autoSubscribe: true });
  }

  async disconnect(): Promise<void> {
    const room = this.#room;

    this.#room = null;

    for (const unbind of this.#unbind.splice(0)) {
      unbind();
    }

    this.#meter?.stop();
    this.#meter = null;

    for (const tracks of this.#captured.values()) {
      for (const track of tracks) {
        track.stop();
      }
    }

    this.#captured.clear();

    if (room !== null) {
      for (const publication of room.localParticipant.trackPublications.values()) {
        publication.track?.stop();
      }
    }

    const context = this.#boostContext;

    this.#boostContext = null;
    this.#boosted.clear();
    void context?.close().catch(() => undefined);

    try {
      await room?.disconnect(true);
    } catch {
      // The media tracks are already stopped; nothing else to recover.
    }

    this.#audioHost?.remove();
    this.#audioHost = null;
  }

  snapshot(): CallSnapshot {
    const room = this.#room;

    if (room === null) {
      return EMPTY_SNAPSHOT;
    }

    const local = room.localParticipant;

    const remotes = [...room.remoteParticipants.values()].sort((left, right) =>
      this.#name(left).localeCompare(this.#name(right)),
    );

    const boostSuspended = this.#boosted.size > 0 && this.#boostContext?.state === "suspended";

    return {
      participants: [
        this.#describe(local, true),
        ...remotes.map((remote) => this.#describe(remote, false)),
      ],
      microphoneEnabled: local.isMicrophoneEnabled,
      cameraEnabled: local.isCameraEnabled,
      screenSharing: local.isScreenShareEnabled,
      canPlaybackAudio: room.canPlaybackAudio && !boostSuspended,
      boostCapped: this.#boostCapped(),
    };
  }

  subscribe(listener: (event: TransportEvent) => void): () => void {
    this.#listeners.add(listener);

    return () => {
      this.#listeners.delete(listener);
    };
  }

  async startAudio(): Promise<void> {
    try {
      await this.#room?.startAudio();
    } finally {
      // A remembered boost engages outside any gesture, so its context may still be suspended:
      // the same control resumes it.
      await this.#boostContext?.resume().catch(() => undefined);
      this.#emit({ type: "changed" });
    }
  }

  async setMicrophone(enabled: boolean): Promise<void> {
    await this.#room?.localParticipant.setMicrophoneEnabled(enabled, this.#audioCapture());
    this.#watchMicrophone();
  }

  /**
   * `setCameraEnabled(false)` only mutes: the track stays published and keeps the device. Off
   * unpublishes instead, releasing the camera and removing the tile on both sides. On keeps
   * frame rate under constrained bandwidth (the SDK's camera default, spelled out so the camera
   * doesn't inherit the room's screen-share "maintain-resolution").
   */
  async setCamera(enabled: boolean): Promise<void> {
    const local = this.#room?.localParticipant;

    if (local === undefined) {
      return;
    }

    if (enabled) {
      await local.setCameraEnabled(true, undefined, {
        degradationPreference: "maintain-framerate",
      });

      return;
    }

    const track = local.getTrackPublication(this.#lk.Track.Source.Camera)?.track;

    if (track === undefined) {
      await local.setCameraEnabled(false);
    } else {
      await local.unpublishTrack(track);
    }
  }

  /**
   * Shared audio is usually music or video rather than speech, and DTX chops it, so a share
   * publishes without DTX. Only a browser that refused to capture with these constraints is
   * retried (video only); a publishing failure would just show a second picker.
   */
  async setScreenShare(enabled: boolean): Promise<void> {
    const local = this.#room?.localParticipant;

    if (local === undefined) {
      return;
    }

    if (!enabled) {
      await local.setScreenShareEnabled(false);

      return;
    }

    try {
      await local.setScreenShareEnabled(true, screenCaptureOptions(true), { dtx: false });
    } catch (error) {
      if (!(error instanceof Error) || !displayMediaRejectedConstraints(error)) {
        throw error;
      }

      await local.setScreenShareEnabled(true, screenCaptureOptions(false), { dtx: false });
    }
  }

  /**
   * Starts the capture: `createScreenTracks` runs before this method's first await, so it stays
   * inside the gesture (Safari denies a getDisplayMedia that starts after a round-trip). The
   * video-only retry only runs after a constraint rejection.
   */
  captureScreen(): Promise<CapturedScreen> {
    const local = this.#room?.localParticipant;

    if (local === undefined || !canShareScreen()) {
      return Promise.reject(new Error("screen-share-unavailable"));
    }

    return local
      .createScreenTracks(screenCaptureOptions(true))
      .catch((error: Error) => {
        if (displayMediaRejectedConstraints(error)) {
          return local.createScreenTracks(screenCaptureOptions(false));
        }

        throw error;
      })
      .then((tracks) => {
        this.#capturedSequence += 1;
        this.#captured.set(this.#capturedSequence, tracks);

        return { id: this.#capturedSequence };
      });
  }

  /** The same publish options as an ordinary share, at the stream's encoding. */
  async publishScreen(captured: CapturedScreen, streamQuality: StreamQuality): Promise<void> {
    const local = this.#room?.localParticipant;
    const tracks = this.#captured.get(captured.id);

    if (local === undefined || tracks === undefined) {
      throw new Error("screen-capture-gone");
    }

    for (const track of tracks) {
      await local.publishTrack(track, {
        dtx: false,
        screenShareEncoding: this.#encoding(streamQuality),
      });
    }

    this.#captured.delete(captured.id);
  }

  discardScreen(captured: CapturedScreen): void {
    for (const track of this.#captured.get(captured.id) ?? []) {
      track.stop();
    }

    this.#captured.delete(captured.id);
  }

  async switchDevice(kind: DeviceKind, deviceId: string): Promise<boolean> {
    const room = this.#room;

    if (room === null) {
      return false;
    }

    try {
      await room.switchActiveDevice(kind, deviceId);
    } catch {
      return false;
    }

    // `switchActiveDevice` records the choice as an `exact` constraint, which turns a later
    // re-acquire (unplug after a switch, then turn the camera on again) into an
    // OverconstrainedError. The stored preference is `ideal`, so the live default is relaxed to
    // match: a missing device falls back to the default instead of failing.
    if (kind === "audioinput" && room.options.audioCaptureDefaults !== undefined) {
      room.options.audioCaptureDefaults.deviceId = { ideal: deviceId };
    }

    if (kind === "videoinput" && room.options.videoCaptureDefaults !== undefined) {
      room.options.videoCaptureDefaults.deviceId = { ideal: deviceId };
    }

    if (kind === "audioinput") {
      void this.#queueNoiseSync();
    }

    // The boost gain sits outside the SDK's path, so a speaker switch retargets it and
    // re-applies every volume: a boost capped on the old speaker may engage on the new one.
    if (kind === "audiooutput") {
      this.#routeBoost();
      this.#applyEveryone();
    }

    this.#emit({ type: "changed" });

    return true;
  }

  activeDevice(kind: DeviceKind): string | undefined {
    try {
      const active = this.#room?.getActiveDevice(kind);

      return active === "default" ? undefined : active;
    } catch {
      return undefined;
    }
  }

  attachVideo(videoId: string, element: HTMLVideoElement): void {
    const found = this.#publication(videoId);

    if (found?.publication.track === undefined) {
      return;
    }

    element.muted = true;
    element.playsInline = true;
    found.publication.track.attach(element);
  }

  detachVideo(videoId: string, element: HTMLVideoElement): void {
    try {
      this.#publication(videoId)?.publication.track?.detach(element);
    } catch {
      // A disconnect can detach the SDK track before this cleanup runs.
    }
  }

  setParticipantVolume(identity: string, volume: number): void {
    this.#volumes.set(identity, Math.max(0, Math.min(200, volume)));
    this.#applyIdentity(identity);
  }

  setParticipantMuted(identity: string, muted: boolean): void {
    if (muted) {
      this.#locallyMuted.add(identity);
    } else {
      this.#locallyMuted.delete(identity);
    }

    this.#applyIdentity(identity);
  }

  setDeafened(deafened: boolean): void {
    this.#deafened = deafened;
    this.#applyEveryone();
  }

  setNoiseSuppression(enabled: boolean): Promise<void> {
    this.#noiseWanted = enabled;

    return this.#queueNoiseSync();
  }

  microphoneLevel(): number {
    const local = this.#room?.localParticipant;

    const track = local?.isMicrophoneEnabled
      ? local.getTrackPublication(this.#lk.Track.Source.Microphone)?.track?.mediaStreamTrack
      : undefined;

    if (track === undefined || track.readyState !== "live") {
      this.#meter?.stop();
      this.#meter = null;

      return 0;
    }

    // Switching, restarting or reconnecting replaces the track: the meter follows it.
    if (this.#meter?.track !== track) {
      this.#meter?.stop();
      this.#meter = createLevelMeter(track);
    }

    return this.#meter?.level() ?? 0;
  }

  /**
   * The viewer's quality for a watched stream: Low and High pin the layer; Auto drops the pin so
   * adaptive streaming sizes it from the element again. The SDK has no public way to drop a pin,
   * so Auto clears the two request fields `setVideoQuality`/`setVideoDimensions` set (as the
   * classic controller does) and re-sends the track settings.
   */
  setViewerQuality(videoId: string, viewerQuality: ViewerQuality): void {
    const publication = this.#remotePublication(videoId);

    if (publication === null) {
      return;
    }

    try {
      if (viewerQuality === "low") {
        publication.setVideoQuality(this.#lk.VideoQuality.LOW);
      } else if (viewerQuality === "high") {
        publication.setVideoQuality(this.#lk.VideoQuality.HIGH);
      } else {
        Object.assign(publication, {
          requestedMaxQuality: undefined,
          requestedVideoDimensions: undefined,
        });
        publication.emitTrackUpdate();
      }
    } catch {
      // Quality is a hint; a rejected hint must not break the view.
    }
  }

  /**
   * Adaptive streaming sizes a subscription from the rendered element and takes the smaller of
   * that and any manual request, so this only matters once the element has actually grown: it
   * asks for the full layer at once instead of waiting for the next resize observation.
   */
  showScreen(videoId: string, size: VideoSize | null): void {
    const publication = this.#remotePublication(videoId);

    if (publication === null) {
      return;
    }

    try {
      publication.setVideoQuality(this.#lk.VideoQuality.HIGH);

      if (size !== null) {
        publication.setVideoDimensions(size);
      }
    } catch {
      // A rejected hint leaves the current layer in place.
    }
  }

  async sampleStats(fresh: boolean): Promise<ConnectionStats | null> {
    const room = this.#room;

    if (room === null) {
      return null;
    }

    if (fresh) {
      this.#statsBaseline = null;
    }

    let publisherReport: RTCStatsReport | null = null;
    let subscriberReport: RTCStatsReport | null = null;

    try {
      const microphone = room.localParticipant.getTrackPublication(
        this.#lk.Track.Source.Microphone,
      )?.track;

      publisherReport = (await microphone?.getRTCStatsReport()) ?? null;
    } catch {
      // A missing sender report reads as unknown.
    }

    try {
      subscriberReport = (await this.#subscriberTrack(room)?.getRTCStatsReport()) ?? null;
    } catch {
      // Alone in a room there is nothing subscribed to.
    }

    if (room !== this.#room) {
      return null;
    }

    const sample = summarizeConnectionStats(
      publisherReport,
      subscriberReport,
      this.#statsBaseline,
      Date.now(),
    );

    this.#statsBaseline = sample.baseline;

    return sample.stats;
  }

  #emit(event: TransportEvent): void {
    for (const listener of this.#listeners) {
      listener(event);
    }
  }

  #roomOptions(): RoomOptions {
    const { AudioPresets, ScreenSharePresets, VideoPresets } = this.#lk;
    const camera = loadDevicePreferences().videoinput;

    return {
      adaptiveStream: true,
      dynacast: true,
      // Spelled out rather than inherited so an SDK upgrade can't quietly change what the call
      // asks the browser to do with a microphone.
      audioCaptureDefaults: this.#audioCapture(),
      // The 720p camera target, the SDK's own default written out to pin it against upgrades.
      videoCaptureDefaults: {
        deviceId: { ideal: camera === "" ? "default" : camera },
        resolution: VideoPresets.h720.resolution,
      },
      publishDefaults: {
        audioPreset: AudioPresets.music,
        dtx: true,
        red: true,
        // Shared code and slides must stay readable: keep resolution, drop frames.
        screenShareEncoding: ScreenSharePresets.h1080fps15.encoding,
        degradationPreference: "maintain-resolution",
        // Camera top layer 1280×720; simulcast adds 640×360 and 320×180 layers so adaptive
        // streaming can size each subscription from its element.
        videoEncoding: VideoPresets.h720.encoding,
        simulcast: true,
        // Stops the microphone's track while muted so the OS indicator clears; unmuting
        // re-acquires from the track's stored constraints, which the noise sync keeps current.
        stopMicTrackOnMute: true,
      },
    };
  }

  /**
   * The microphone capture: RNNoise replaces the browser's suppressor when it is on, so the
   * capture asks for none and the signal is filtered once. Voice isolation is an "ideal"
   * constraint (Chrome's stronger speech isolation; ignored elsewhere). The remembered
   * microphone is "ideal" too, so a vanished device falls back to the default silently.
   */
  #audioCapture(): AudioCaptureOptions {
    const microphone = loadDevicePreferences().audioinput;

    const options: AudioCaptureOptions = {
      autoGainControl: true,
      echoCancellation: true,
      noiseSuppression: !(this.#noiseAvailable && this.#noiseWanted),
      voiceIsolation: true,
    };

    if (microphone !== "") {
      options.deviceId = { ideal: microphone };
    }

    return options;
  }

  #encoding(streamQuality: StreamQuality): VideoEncoding {
    const presets = this.#lk.ScreenSharePresets;

    switch (streamQuality) {
      case "720p15":
        return presets.h720fps15.encoding;
      case "1080p30":
        return presets.h1080fps30.encoding;
      default:
        return presets.h1080fps15.encoding;
    }
  }

  #bind(room: Room): void {
    const { RoomEvent } = this.#lk;

    const changed = (): void => {
      if (room === this.#room) {
        this.#emit({ type: "changed" });
      }
    };

    const on = (event: Parameters<Room["on"]>[0], handler: Parameters<Room["on"]>[1]): void => {
      room.on(event, handler);
      this.#unbind.push(() => {
        room.off(event, handler);
      });
    };

    on(RoomEvent.Reconnecting, () => {
      this.#emit({ type: "reconnecting" });
    });
    on(RoomEvent.Reconnected, () => {
      this.#emit({ type: "reconnected" });
      changed();
    });
    on(RoomEvent.Disconnected, (reason?: number) => {
      if (room === this.#room) {
        this.#emit({ type: "disconnected", reason: this.#disconnectReason(reason) });
      }
    });

    for (const event of [
      RoomEvent.ParticipantConnected,
      RoomEvent.ParticipantNameChanged,
      RoomEvent.ActiveSpeakersChanged,
      RoomEvent.TrackMuted,
      RoomEvent.TrackUnmuted,
      RoomEvent.TrackPublished,
      RoomEvent.TrackUnpublished,
      RoomEvent.LocalTrackPublished,
      RoomEvent.AudioPlaybackStatusChanged,
    ]) {
      on(event, changed);
    }

    on(RoomEvent.ParticipantDisconnected, (participant: RemoteParticipant) => {
      this.#boosted.delete(participant.identity);
      changed();
    });
    on(
      RoomEvent.TrackSubscribed,
      (
        track: RemoteTrack,
        _publication: RemoteTrackPublication,
        participant: RemoteParticipant,
      ) => {
        if (track.kind === this.#lk.Track.Kind.Audio) {
          const element = track.attach();

          element.autoplay = true;
          element.hidden = true;
          this.#host().append(element);
        }

        this.#applyParticipant(participant);
        changed();
      },
    );
    on(
      RoomEvent.TrackUnsubscribed,
      (
        track: RemoteTrack,
        _publication: RemoteTrackPublication,
        participant: RemoteParticipant,
      ) => {
        // A resubscribe may hand back a new track without the gain, so a boost is forgotten
        // here and re-engaged on the next subscribe rather than silently dropping.
        this.#boosted.delete(participant.identity);

        for (const element of track.detach()) {
          element.remove();
        }

        changed();
      },
    );
    on(RoomEvent.LocalTrackUnpublished, (publication: TrackPublication) => {
      changed();

      if (publication.source === this.#lk.Track.Source.ScreenShare) {
        this.#emit({ type: "screen-ended" });
      }
    });
    on(RoomEvent.ConnectionQualityChanged, (_quality: string, participant: Participant) => {
      if (room !== this.#room) {
        return;
      }

      if (participant === room.localParticipant) {
        this.#emit({ type: "quality", quality: quality(participant) });
      } else {
        changed();
      }
    });
    // The SDK retargets tracks itself when a device vanishes. The stored preference keeps the
    // person's own choice; a retargeted microphone gets suppression restored onto its new
    // track, and a retargeted speaker carries the boost with it.
    on(RoomEvent.ActiveDeviceChanged, (kind: MediaDeviceKind) => {
      if (room !== this.#room) {
        return;
      }

      if (kind === "audioinput") {
        void this.#queueNoiseSync();
      }

      if (kind === "audiooutput") {
        this.#routeBoost();
      }

      this.#emit({ type: "device", kind });
    });
  }

  #disconnectReason(reason: number | undefined): DisconnectReason {
    const { DisconnectReason: Reason } = this.#lk;

    switch (reason) {
      case Reason.PARTICIPANT_REMOVED:
        return "removed";
      case Reason.ROOM_DELETED:
      case Reason.ROOM_CLOSED:
        return "ended";
      case Reason.DUPLICATE_IDENTITY:
        return "replaced";
      default:
        return "lost";
    }
  }

  #host(): HTMLDivElement {
    if (this.#audioHost === null) {
      this.#audioHost = document.createElement("div");
      this.#audioHost.hidden = true;
      this.#audioHost.dataset.huddleAudio = "";
      document.body.append(this.#audioHost);
    }

    return this.#audioHost;
  }

  #name(participant: Participant): string {
    return participant.name === undefined || participant.name === ""
      ? participant.identity
      : participant.name;
  }

  #describe(participant: Participant, local: boolean): CallParticipant {
    const { Source } = this.#lk.Track;
    const microphone = participant.getTrackPublication(Source.Microphone);
    const camera = participant.getTrackPublication(Source.Camera);
    const screen = participant.getTrackPublication(Source.ScreenShare);

    const shown = (publication: TrackPublication | undefined): string | null =>
      publication?.track === undefined || publication.isMuted ? null : publication.trackSid;

    return {
      identity: participant.identity,
      name: this.#name(participant),
      local,
      speaking: participant.isSpeaking,
      audioLevel: participant.audioLevel,
      microphoneMuted: local ? !participant.isMicrophoneEnabled : microphone?.isMuted === true,
      quality: quality(participant),
      cameraId: shown(camera),
      screenId: shown(screen),
    };
  }

  #publication(
    videoId: string,
  ): { readonly publication: TrackPublication; readonly local: boolean } | null {
    const room = this.#room;

    if (room === null) {
      return null;
    }

    const own = room.localParticipant.trackPublications.get(videoId);

    if (own !== undefined) {
      return { publication: own, local: true };
    }

    for (const participant of room.remoteParticipants.values()) {
      const publication = participant.trackPublications.get(videoId);

      if (publication !== undefined) {
        return { publication, local: false };
      }
    }

    return null;
  }

  #remotePublication(videoId: string): RemoteTrackPublication | null {
    for (const participant of this.#room?.remoteParticipants.values() ?? []) {
      const publication = participant.trackPublications.get(videoId);

      if (publication !== undefined) {
        return publication;
      }
    }

    return null;
  }

  /** Every subscribed track shares the subscriber connection: the first audio one stands for it. */
  #subscriberTrack(room: Room): RemoteTrack | null {
    let fallback: RemoteTrack | null = null;

    for (const participant of room.remoteParticipants.values()) {
      for (const publication of participant.trackPublications.values()) {
        if (publication.track === undefined || !publication.isSubscribed) {
          continue;
        }

        if (publication.kind === this.#lk.Track.Kind.Audio) {
          return publication.track;
        }

        fallback ??= publication.track;
      }
    }

    return fallback;
  }

  #applyIdentity(identity: string): void {
    const participant = this.#room?.remoteParticipants.get(identity);

    if (participant !== undefined) {
      this.#applyParticipant(participant);
    }
  }

  #applyEveryone(): void {
    for (const participant of this.#room?.remoteParticipants.values() ?? []) {
      this.#applyParticipant(participant);
    }

    this.#emit({ type: "changed" });
  }

  /**
   * One person's local mute (their microphone is unsubscribed, so the server stops sending it)
   * or volume. Untouched people keep the SDK's own audio path.
   */
  #applyParticipant(participant: RemoteParticipant): void {
    const identity = participant.identity;
    const microphone = participant.getTrackPublication(this.#lk.Track.Source.Microphone);

    if (this.#locallyMuted.has(identity)) {
      microphone?.setSubscribed(false);

      return;
    }

    if (microphone !== undefined && !microphone.isDesired) {
      microphone.setSubscribed(true);
    }

    const volume = this.#volumes.get(identity) ?? 100;

    if (!this.#deafened && volume === 100 && !this.#boosted.has(identity)) {
      participant.setVolume(1);

      return;
    }

    this.#applyVolume(participant, microphone, this.#deafened ? 0 : volume);
  }

  /**
   * Up to 100% rides the audio element; above needs a Web Audio gain, which the SDK wires when a
   * context is set on the track. The elements are muted while the gain carries them (the SDK
   * only quiets elements attached after the context is set, so audio would otherwise play
   * twice). Where the boost can't follow the chosen speaker it caps at 100%.
   */
  #applyVolume(
    participant: RemoteParticipant,
    microphone: RemoteTrackPublication | undefined,
    wanted: number,
  ): void {
    const volume = wanted > 100 && this.#boostCapped() ? 100 : wanted;
    const track = microphone?.audioTrack;

    if (volume > 100) {
      const context = this.#boostAudioContext();

      if (context === null) {
        participant.setVolume(1);

        return;
      }

      try {
        if (track !== undefined && !this.#boosted.has(participant.identity)) {
          track.setAudioContext(context);

          for (const element of track.attachedElements) {
            element.volume = 0;
            element.muted = true;
          }

          this.#boosted.add(participant.identity);
        }

        participant.setVolume(volume / 100);
      } catch {
        participant.setVolume(1);
      }

      return;
    }

    if (this.#boosted.delete(participant.identity)) {
      try {
        track?.setAudioContext(undefined);
      } catch {
        // The element volume below still applies.
      }

      for (const element of track?.attachedElements ?? []) {
        element.muted = false;
      }
    }

    participant.setVolume(volume / 100);
  }

  #boostAudioContext(): AudioContext | null {
    if (this.#boostContext !== null) {
      return this.#boostContext;
    }

    if (globalThis.AudioContext === undefined) {
      return null;
    }

    try {
      this.#boostContext = new AudioContext();
    } catch {
      return null;
    }

    this.#routeBoost();

    return this.#boostContext;
  }

  /** The active speaker, or the remembered one; "" is the default output. */
  #selectedOutput(): string {
    if (!("setSinkId" in HTMLMediaElement.prototype)) {
      return "";
    }

    const active = this.activeDevice("audiooutput");

    if (active !== undefined && active !== "") {
      return active;
    }

    const stored = loadDevicePreferences().audiooutput;

    return stored === "default" ? "" : stored;
  }

  /** Without `AudioContext.setSinkId` boosted audio would escape to the default speaker. */
  #boostCapped(): boolean {
    return (
      (globalThis.AudioContext === undefined || !("setSinkId" in AudioContext.prototype)) &&
      this.#selectedOutput() !== ""
    );
  }

  /** The boost gain lives outside the SDK's path, so it needs its own output routing. */
  #routeBoost(): void {
    const context = this.#boostContext;

    if (context === null || !sinkable(context)) {
      return;
    }

    context.setSinkId(this.#selectedOutput()).catch(() => {
      // The default output stands in when the selected one rejects.
    });
  }

  #microphoneTrack(): LocalAudioTrack | undefined {
    return this.#room?.localParticipant.getTrackPublication(this.#lk.Track.Source.Microphone)
      ?.audioTrack;
  }

  /**
   * The SDK restarts the microphone itself when an unplug ends its track; once the new track
   * lands, suppression is restored onto it. One listener per track object.
   */
  #watchMicrophone(): void {
    const track = this.#microphoneTrack();

    if (track === undefined || track === this.#watchedMicrophone) {
      return;
    }

    this.#watchedMicrophone = track;

    const ended = (): void => {
      setTimeout(() => {
        if (this.#room !== null) {
          void this.#queueNoiseSync();
        }
      }, 0);
    };

    track.on(this.#lk.TrackEvent.Ended, ended);
    this.#unbind.push(() => {
      track.off(this.#lk.TrackEvent.Ended, ended);
    });
  }

  /** Every caller goes through one queue: overlapping runs could leave the processor attached. */
  #queueNoiseSync(): Promise<void> {
    this.#noiseQueue = this.#noiseQueue.catch(() => undefined).then(() => this.#syncNoise());

    return this.#noiseQueue;
  }

  /**
   * The processor stays attached across mute: the stopped track it feeds goes silent, and
   * unmuting hands the live track back to the same worklet with no unfiltered burst.
   */
  async #syncNoise(): Promise<void> {
    const room = this.#room;
    const track = this.#microphoneTrack();

    if (room === null || track === undefined) {
      return;
    }

    this.#watchMicrophone();
    // Each sync judges afresh: a failed restart below blocks the healthy verdict at the end.
    this.#restartFailed = false;
    this.#microphoneLost = false;

    const wanted = this.#noiseAvailable && this.#noiseWanted;
    let failure: NoiseOutcome["failure"] = null;

    if (wanted !== (track.getProcessor() !== undefined)) {
      try {
        if (wanted) {
          await track.setProcessor(new NoiseSuppressor());
        } else {
          // Off re-acquires with browser suppression first while RNNoise still filters, then
          // stops the processor: double-filtered for a moment, never unfiltered.
          await this.#syncCapture(room, track, false);
          await track.stopProcessor();
        }
      } catch (error) {
        if (wanted) {
          // The browser's own suppression beats dropping the microphone out of the call.
          await track.stopProcessor().catch(() => undefined);

          if (error instanceof NoiseSuppressionUnsupported || !noiseSuppressionSupported()) {
            this.#noiseAvailable = false;
            failure = "unsupported";
          } else if (error instanceof Error && error.name === "NotSupportedError") {
            this.#noiseAvailable = false;
            failure = "unsupported";
          } else {
            // A worklet or model that failed to load may well load next time.
            this.#noiseWanted = false;
            failure = "failed";
          }
        }
      }
    }

    const processing = track.getProcessor() !== undefined;

    // Browser suppression is on exactly when RNNoise is off. This reads the attached processor,
    // not the flags, so a failed stop still describes the microphone truthfully.
    await this.#syncCapture(room, track, processing);

    const settledWanted = this.#noiseAvailable && this.#noiseWanted;

    this.#emit({
      type: "noise",
      outcome: {
        processing,
        failure,
        microphoneLost: this.#microphoneLost,
        settled:
          !this.#restartFailed &&
          settledWanted === processing &&
          track.constraints.noiseSuppression === !settledWanted,
      },
    });
  }

  /**
   * Chromium rebuilds its audio processing only at capture time (`applyConstraints` updates the
   * stored copy without touching the graph), so a change while live re-acquires the microphone.
   * While muted the source is stopped, so only the stored constraints are refreshed and the next
   * unmute re-acquires from them. The full capture set is written because the SDK replaces the
   * stored constraints wholesale on restart. Never throws.
   */
  async #syncCapture(room: Room, track: LocalAudioTrack, rnnoiseOn: boolean): Promise<void> {
    const wanted = !rnnoiseOn;
    const constraints: AudioCaptureOptions = { ...this.#audioCapture(), noiseSuppression: wanted };
    const active = track.constraints.deviceId;

    if (active !== undefined) {
      constraints.deviceId = active;
    } else if (constraints.deviceId === undefined) {
      const live = track.getSourceTrackSettings().deviceId;

      if (live !== undefined) {
        constraints.deviceId = { ideal: live };
      }
    }

    const storedMatches = track.constraints.noiseSuppression === wanted;

    // The SDK merges stored constraints only after a live apply succeeds, so a stopped track
    // would keep stale processing: the stored copy (the getter's own object) is written here.
    Object.assign(track.constraints, constraints);

    const live = room.localParticipant.isMicrophoneEnabled && !track.isMuted;

    if (!live || storedMatches) {
      return;
    }

    await this.#restartCapture(track, constraints);
  }

  /**
   * A failed re-acquire retries twice (the classic third try used the stored constraints, which
   * the write above made these). A track still live keeps its old filtering; a silent one is
   * reported so the dock can say so.
   */
  async #restartCapture(track: LocalAudioTrack, constraints: AudioCaptureOptions): Promise<void> {
    for (let attempt = 0; attempt < 3; attempt += 1) {
      try {
        await track.restartTrack(constraints);
        this.#restartFailed = false;

        return;
      } catch {
        // Next attempt.
      }
    }

    this.#restartFailed = true;
    this.#microphoneLost = track.isMuted || track.mediaStreamTrack.readyState === "ended";
  }
}
