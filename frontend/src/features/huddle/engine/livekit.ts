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
  LocalVideoTrack,
  Participant,
  RemoteParticipant,
  RemoteTrack,
  RemoteTrackPublication,
  Room,
  RoomOptions,
  ScreenShareCaptureOptions,
  TrackPublication,
  TrackPublishOptions,
} from "livekit-client";
import type { StreamQuality } from "../../../gen/StreamQuality.ts";
import { loadForUpdate } from "../../../service-worker/update-required.ts";
import { ActionError } from "../../../sync/run.ts";
import { microphonePublishOptions } from "./audio-quality.ts";
import { cameraCaptureOptions, cameraPublishOptions } from "./camera-quality.ts";
import { canShareScreen, permissionDenied } from "./devices.ts";
import { createLevelMeter, type LevelMeter } from "./meter.ts";
import {
  NoiseSuppressionUnsupported,
  NoiseSuppressor,
  noiseSuppressionSupported,
} from "./noise.ts";
import { loadCameraQuality, loadDevicePreferences } from "./preferences.ts";
import {
  applyScreenContentHint,
  DEFAULT_SHARE_QUALITY,
  screenCaptureAttempts,
  screenPublishOptions,
} from "./screen-quality.ts";
import {
  type StatEntry,
  type StatsBaseline,
  summarizeConnectionStats,
  summarizeVideoStats,
  type VideoStatsBaseline,
  type VideoStatsInput,
} from "./stats.ts";
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
  type ViewerQuality,
} from "./transport.ts";
import { selectVideoCodec, supportsVideoSimulcast, videoCodecRejected } from "./video-codecs.ts";
import { focusScreenPublication, VIDEO_ADAPTIVE_STREAM } from "./video-quality.ts";

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
  return liveKitTransport(await loadLiveKit());
}

/** A transport over an SDK already loaded (a test's, with its `Room` kept off the network). */
export function liveKitTransport(lk: LiveKit): CallTransport {
  return new LiveKitTransport(lk);
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

/** A failed AddTrack cannot be cancelled on the server without leaving its old session. */
function restartVideoPublisher(room: Room) {
  let cancel: () => void = () => undefined;
  const ready = new Promise<void>((resolve, reject) => {
    const finish = (error?: Error) => {
      clearTimeout(timeout);
      room.engine.off("restarted", restarted);
      room.engine.off("closing", closing);
      if (error === undefined) resolve();
      else reject(error);
    };
    const restarted = () => finish();
    const closing = () => finish(new Error("call-ended"));
    const timeout = setTimeout(() => finish(new Error("video-reconnect-timeout")), 20_000);
    cancel = closing;
    room.engine.on("restarted", restarted);
    room.engine.on("closing", closing);
    try {
      room.engine.reconnect();
    } catch {
      finish(new Error("video-reconnect-failed"));
    }
  });
  return { ready, cancel };
}

/**
 * Runs `capture` with each of the quality's capture attempts in turn, moving on only when the
 * browser rejected the constraints (a cancelled picker or a publishing failure would just show a
 * second picker). The first call runs synchronously, so a capture started in a gesture stays in it.
 */
function withCaptureFallback<T>(
  quality: StreamQuality,
  capture: (options: ScreenShareCaptureOptions) => Promise<T>,
  browser: ReturnType<LiveKit["getBrowser"]>,
): Promise<T> {
  const [first, ...rest] = screenCaptureAttempts(quality, browser);
  // The SDK assigns the hint during capture. Start with its compatible detail hint, then try
  // text on the native track with read-back, so a rejected text hint cannot reopen the picker.
  const attempt = (options: ScreenShareCaptureOptions): Promise<T> =>
    capture({ ...options, contentHint: options.contentHint === "text" ? "detail" : "motion" });

  return rest.reduce<Promise<T>>(
    (pending, options) =>
      pending.catch((error: Error) => {
        if (!displayMediaRejectedConstraints(error)) {
          throw error;
        }

        return attempt(options);
      }),
    attempt(first ?? {}),
  );
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
  #videoStatsBaseline: VideoStatsBaseline | null = null;
  #statsGeneration = 0;
  #cameraQuality = loadCameraQuality();
  readonly #focusedScreens = new Map<RemoteTrackPublication, () => void>();
  /** The microphone's encoding target, from the grant this connection joined with. */
  #microphonePublish: TrackPublishOptions = microphonePublishOptions(undefined);
  /** Soundboard clips started or playing now, each with its stop: run on leaving or deafening. */
  readonly #clips = new Map<HTMLAudioElement, () => void>();

  constructor(lk: LiveKit) {
    this.#lk = lk;
  }

  async connect(
    url: string,
    token: string,
    noiseSuppression: boolean,
    audioBitrate: number,
  ): Promise<void> {
    this.#noiseWanted = noiseSuppression;
    this.#microphonePublish = microphonePublishOptions(audioBitrate);
    this.#cameraQuality = loadCameraQuality();

    const room = new this.#lk.Room(this.#roomOptions());

    this.#room = room;
    this.#bind(room);

    try {
      await room.connect(url, token, { autoSubscribe: true });
    } catch (error) {
      if (error instanceof this.#lk.ConnectionError && error.status === 403) {
        // The SDK prefixes the gateway's reason ("could not establish signal connection: …").
        const message = error.message.trim();

        if (message.endsWith("This stage is full")) {
          throw new ActionError("StageAudienceFull", "This stage is full");
        }

        if (message.endsWith("This voice room is full")) {
          throw new ActionError("HuddleRoomFull", "This voice room is full");
        }
      }

      throw error;
    }
  }

  async disconnect(): Promise<void> {
    const room = this.#room;

    this.#room = null;
    this.#statsGeneration += 1;
    this.#statsBaseline = null;
    this.#videoStatsBaseline = null;

    for (const restore of this.#focusedScreens.values()) {
      restore();
    }

    this.#focusedScreens.clear();

    for (const unbind of this.#unbind.splice(0)) {
      unbind();
    }

    this.#meter?.stop();
    this.#meter = null;

    this.#stopClips();

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

  /**
   * The SDK publishes the microphone the first time it's enabled; the room's voice bitrate rides
   * along as that publication's options (the room defaults fill in the rest). Later toggles
   * mute and unmute the same publication, so its encoding stays.
   */
  async setMicrophone(enabled: boolean): Promise<void> {
    await this.#room?.localParticipant.setMicrophoneEnabled(
      enabled,
      this.#audioCapture(),
      this.#microphonePublish,
    );
    this.#watchMicrophone();
  }

  /**
   * `setCameraEnabled(false)` only mutes: the track stays published and keeps the device. Off
   * unpublishes instead, releasing the camera and removing the tile on both sides. On keeps
   * frame rate under constrained bandwidth (the SDK's camera default, spelled out so the camera
   * doesn't inherit the room's screen-share "maintain-resolution").
   */
  async setCamera(enabled: boolean): Promise<void> {
    const room = this.#room;
    const local = room?.localParticipant;

    if (local === undefined) {
      return;
    }

    if (enabled) {
      const existing = local.getTrackPublication(this.#lk.Track.Source.Camera)?.videoTrack;
      if (existing !== undefined) {
        await existing.unmute();
        return;
      }
      const track = await this.#lk.createLocalVideoTrack(
        cameraCaptureOptions(this.#cameraQuality, loadDevicePreferences().videoinput),
      );
      track.source = this.#lk.Track.Source.Camera;
      if (room !== this.#room) {
        track.stop();
        return;
      }
      try {
        await this.#publishVideo(
          track,
          cameraPublishOptions(this.#cameraQuality, this.#lk.VideoPreset),
          null,
        );
      } catch (error) {
        track.stop();
        throw error;
      }

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
   * An ordinary share at `quality` (see `screen-quality.ts`). Only a browser that refused to
   * capture with the quality's constraints is retried, with fewer of them.
   */
  async setScreenShare(
    enabled: boolean,
    quality: StreamQuality = DEFAULT_SHARE_QUALITY,
  ): Promise<void> {
    const local = this.#room?.localParticipant;

    if (local === undefined) {
      return;
    }

    if (!enabled) {
      await local.setScreenShareEnabled(false);

      return;
    }

    const captured = await this.captureScreen(quality);
    try {
      await this.publishScreen(captured, quality);
    } catch (error) {
      this.discardScreen(captured);
      throw error;
    }
  }

  /**
   * Starts the capture: `createScreenTracks` runs before this method's first await, so it stays
   * inside the gesture (Safari denies a getDisplayMedia that starts after a round-trip). The
   * video-only retry only runs after a constraint rejection.
   */
  captureScreen(quality: StreamQuality): Promise<CapturedScreen> {
    const local = this.#room?.localParticipant;

    if (local === undefined || !canShareScreen()) {
      return Promise.reject(new Error("screen-share-unavailable"));
    }

    return withCaptureFallback(
      quality,
      (options) => local.createScreenTracks(options),
      this.#lk.getBrowser(),
    ).then((tracks) => {
      for (const track of tracks) {
        if (track.kind === this.#lk.Track.Kind.Video) {
          applyScreenContentHint(track.mediaStreamTrack, quality);
        }
      }

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

    const publish = screenPublishOptions(streamQuality, this.#lk.VideoPreset);

    const published = [];
    try {
      for (const track of tracks) {
        published.push(
          track instanceof this.#lk.LocalVideoTrack
            ? await this.#publishVideo(track, publish, streamQuality)
            : await local.publishTrack(track, publish),
        );
      }
    } catch (error) {
      for (const publication of published) {
        if (publication.track !== undefined)
          await local.unpublishTrack(publication.track).catch(() => undefined);
      }
      throw error;
    }

    this.#captured.delete(captured.id);
  }

  async #publishVideo(
    track: LocalVideoTrack,
    options: TrackPublishOptions,
    screenQuality: StreamQuality | null,
  ) {
    const room = this.#room;
    if (room === null) throw new Error("call-ended");
    let active: LocalVideoTrack | MediaStreamTrack = track;
    const stopPending = () => active.stop();
    this.#unbind.push(stopPending);
    try {
      const publishTrack = async (video: LocalVideoTrack, publishOptions: TrackPublishOptions) => {
        const publication = await room.localParticipant.publishTrack(video, publishOptions);
        if (room !== this.#room) {
          await room.localParticipant.unpublishTrack(video).catch(() => undefined);
          video.stop();
          throw new Error("call-ended");
        }
        return publication;
      };
      const simulcast = supportsVideoSimulcast(this.#lk.getBrowser(), room.engine.serverVersion);
      const textScreen = screenQuality !== null && screenQuality !== "1080p60";
      const encoding = screenQuality === null ? options.videoEncoding : options.screenShareEncoding;
      const settings = track.mediaStreamTrack.getSettings();
      const codec = await selectVideoCodec({
        width: settings.width ?? 1280,
        height: settings.height ?? 720,
        framerate: Math.min(settings.frameRate ?? 30, encoding?.maxFramerate ?? 30),
        bitrate: encoding?.maxBitrate ?? 1_700_000,
        scalabilityMode: screenQuality !== null || simulcast ? "L1T3" : "L3T3_KEY",
        advancedAllowed: !textScreen || simulcast,
        supportsAV1: this.#lk.supportsAV1,
        supportsVP9: this.#lk.supportsVP9,
      });
      if (room !== this.#room) throw new Error("call-ended");
      const publish: TrackPublishOptions = {
        ...options,
        videoCodec: codec,
        backupCodec: codec === "vp8" ? false : { codec: "vp8" },
        backupCodecPolicy: this.#lk.BackupCodecPolicy.SIMULCAST,
      };
      if (codec !== "vp8")
        publish.scalabilityMode = screenQuality !== null || simulcast ? "L1T3" : "L3T3_KEY";
      try {
        return await publishTrack(track, publish);
      } catch (error) {
        if (codec === "vp8" || !videoCodecRejected(error) || room !== this.#room) throw error;
        const muted = track.isMuted;
        const media = track.mediaStreamTrack.clone();
        active = media;
        track.stop();
        try {
          // A failed publish may have sent AddTrack before creating any sender. A full SDK
          // restart clears both the server reservation and local partial senders/resolvers.
          const restart = restartVideoPublisher(room);
          this.#unbind.push(restart.cancel);
          try {
            await restart.ready;
          } finally {
            const index = this.#unbind.indexOf(restart.cancel);
            if (index >= 0) this.#unbind.splice(index, 1);
          }
        } catch (cleanupError) {
          media.stop();
          throw cleanupError;
        }
        if (room !== this.#room) {
          media.stop();
          throw error;
        }
        const fallback = new this.#lk.LocalVideoTrack(media, undefined, false);
        active = fallback;
        fallback.source = track.source;
        if (screenQuality !== null) applyScreenContentHint(media, screenQuality);
        if (muted) await fallback.mute();
        try {
          return await publishTrack(fallback, {
            ...options,
            videoCodec: "vp8",
            backupCodec: false,
          });
        } catch (fallbackError) {
          fallback.stop();
          throw fallbackError;
        }
      }
    } finally {
      const index = this.#unbind.indexOf(stopPending);
      if (index >= 0) this.#unbind.splice(index, 1);
    }
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

    // Deafened hears nothing, soundboard clips included; undeafening doesn't bring them back.
    if (deafened) {
      this.#stopClips();
    }
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
      this.#focusScreen(publication, viewerQuality === "high");

      if (viewerQuality === "low") {
        publication.setVideoQuality(this.#lk.VideoQuality.LOW);
      } else if (viewerQuality === "high") {
        publication.setVideoQuality(this.#lk.VideoQuality.HIGH);
      } else {
        Object.assign(publication, {
          requestedMaxQuality: undefined,
          requestedVideoDimensions: undefined,
        });
      }

      publication.emitTrackUpdate();
    } catch {
      // Quality is a hint; a rejected hint must not break the view.
    }
  }

  /** A focused share requests HIGH; collapse returns to the current adaptive dimensions. */
  showScreen(videoId: string, focused: boolean): void {
    const publication = this.#remotePublication(videoId);

    if (publication === null) {
      return;
    }

    try {
      this.#focusScreen(publication, focused);
      publication.setVideoQuality(this.#lk.VideoQuality.HIGH);
      // The setter skips identical quality values, including a collapse from focused HIGH.
      publication.emitTrackUpdate();
    } catch {
      // A rejected hint leaves the current layer in place.
    }
  }

  #focusScreen(publication: RemoteTrackPublication, focused: boolean): void {
    if (focused && !this.#focusedScreens.has(publication)) {
      this.#focusedScreens.set(publication, focusScreenPublication(publication));
    } else if (!focused) {
      this.#focusedScreens.get(publication)?.();
      this.#focusedScreens.delete(publication);
    }
  }

  async sampleStats(fresh: boolean): Promise<ConnectionStats | null> {
    const room = this.#room;

    if (room === null) {
      return null;
    }

    const generation = ++this.#statsGeneration;
    if (fresh) {
      this.#statsBaseline = null;
      this.#videoStatsBaseline = null;
    }

    const reports = new Map<LocalTrack | RemoteTrack, Promise<RTCStatsReport | null>>();
    const read = (
      track: LocalTrack | RemoteTrack | undefined | null,
    ): Promise<RTCStatsReport | null> => {
      if (track == null) {
        return Promise.resolve(null);
      }

      const cached = reports.get(track);
      if (cached !== undefined) return cached;
      const sources = [
        track
          .getRTCStatsReport()
          .then((report) => report ?? null)
          .catch(() => null),
      ];
      if (track instanceof this.#lk.LocalVideoTrack) {
        for (const codec of track.simulcastCodecs.values()) {
          if (codec.sender !== undefined && codec.sender !== track.sender) {
            sources.push(codec.sender.getStats().catch(() => null));
          }
        }
      }
      const pending = Promise.all(sources).then((samples) => {
        const combined = new Map<string, StatEntry>();
        let available = false;
        for (const sample of samples) {
          if (sample === null) continue;
          available = true;
          for (const [id, entry] of sample) combined.set(id, entry);
        }
        return available ? combined : null;
      });
      reports.set(track, pending);

      return pending;
    };

    const videoInputs: Promise<VideoStatsInput>[] = [];
    for (const participant of [room.localParticipant, ...room.remoteParticipants.values()]) {
      for (const publication of participant.videoTrackPublications.values()) {
        if (publication.track === undefined) continue;
        const input: Omit<VideoStatsInput, "report"> = {
          participantId: participant.identity,
          participantName: participant.name || participant.identity,
          trackSid: publication.trackSid,
          direction: participant === room.localParticipant ? "sending" : "receiving",
          source: publication.source === this.#lk.Track.Source.ScreenShare ? "screen" : "camera",
        };
        videoInputs.push(read(publication.track).then((report) => ({ ...input, report })));
      }
    }

    const microphone = room.localParticipant.getTrackPublication(
      this.#lk.Track.Source.Microphone,
    )?.track;
    const [publisherReport, subscriberReport, videos] = await Promise.all([
      read(microphone),
      read(this.#subscriberTrack(room)),
      Promise.all(videoInputs),
    ]);

    if (room !== this.#room || generation !== this.#statsGeneration) {
      return null;
    }

    const publisher = new Map<string, StatEntry>(publisherReport ?? []);
    const subscriber = new Map<string, StatEntry>(subscriberReport ?? []);
    for (const input of videos) {
      const combined = input.direction === "sending" ? publisher : subscriber;
      for (const [id, entry] of input.report ?? []) combined.set(id, entry);
    }
    const now = Date.now();
    const sample = summarizeConnectionStats(
      publisher.size === 0 ? null : publisher,
      subscriber.size === 0 ? null : subscriber,
      this.#statsBaseline,
      now,
    );
    const videoSample = summarizeVideoStats(videos, this.#videoStatsBaseline, now);

    this.#statsBaseline = sample.baseline;
    this.#videoStatsBaseline = videoSample.baseline;

    return { ...sample.stats, videoTracks: videoSample.videoTracks };
  }

  playClip(url: string, volume: number): Promise<void> {
    const room = this.#room;

    if (room === null || this.#deafened) {
      return Promise.resolve();
    }

    // A plain element on the call's speaker: never routed through the room or the microphone.
    const clip = new Audio(url);
    const sink = this.activeDevice("audiooutput");

    clip.volume = Math.max(0, Math.min(1, volume));

    return new Promise<void>((resolve) => {
      const done = (): void => {
        this.#clips.delete(clip);
        clip.removeEventListener("ended", done);
        clip.removeEventListener("error", done);
        clip.removeEventListener("pause", done);
        resolve();
      };

      // Stopping one that hasn't started yet fires no `pause`, so it settles here either way.
      const stop = (): void => {
        clip.pause();
        done();
      };

      this.#clips.set(clip, stop);
      clip.addEventListener("ended", done);
      clip.addEventListener("error", done);
      clip.addEventListener("pause", done);

      const start = async (): Promise<void> => {
        if (sink !== undefined && "setSinkId" in clip) {
          await clip.setSinkId(sink).catch(() => undefined);
        }

        // Left, deafened or rejoined while the speaker was being set: the clip was stopped.
        if (!this.#clips.has(clip) || this.#room !== room) {
          return;
        }

        await clip.play();
      };

      start().catch(done);
    });
  }

  /** Stops every soundboard clip, started or still starting. */
  #stopClips(): void {
    for (const stop of [...this.#clips.values()]) {
      stop();
    }

    this.#clips.clear();
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
      adaptiveStream: VIDEO_ADAPTIVE_STREAM,
      dynacast: true,
      // Spelled out rather than inherited so an SDK upgrade can't quietly change what the call
      // asks the browser to do with a microphone.
      audioCaptureDefaults: this.#audioCapture(),
      // The 720p camera target, the SDK's own default written out to pin it against upgrades.
      videoCaptureDefaults: cameraCaptureOptions(
        this.#cameraQuality,
        camera === "" ? "default" : camera,
      ),
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
      RoomEvent.LocalTrackPublished,
      RoomEvent.AudioPlaybackStatusChanged,
    ]) {
      on(event, changed);
    }

    on(RoomEvent.TrackUnpublished, (publication: RemoteTrackPublication) => {
      this.#focusScreen(publication, false);
      changed();
    });

    on(RoomEvent.ParticipantDisconnected, (participant: RemoteParticipant) => {
      for (const publication of participant.trackPublications.values()) {
        this.#focusScreen(publication, false);
      }

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
