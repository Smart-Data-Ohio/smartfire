/**
 * What each stream quality asks of the browser and the encoder, for the stage and for an ordinary
 * screen share alike. Each profile requests its named capture size and frame rate. The encoder
 * ceilings stay pinned to the SDK presets; 1080p60 adds a 720p/30 layer for smaller views.
 *
 * Every capture constraint is an "ideal": a browser or screen that can't deliver 60 fps (or full
 * HD) shares at what it can rather than failing, and a browser that rejects the constraints
 * outright gets one more try without them.
 */
import type {
  getBrowser,
  ScreenShareCaptureOptions,
  TrackPublishOptions,
  VideoEncoding,
  VideoPreset,
} from "livekit-client";
import type { StreamQuality } from "../../../gen/StreamQuality.ts";

/** Every quality, in picker order. */
export const STREAM_QUALITIES = [
  "720p15",
  "1080p15",
  "1080p30",
  "1080p60",
] as const satisfies readonly StreamQuality[];

/** An ordinary share's quality until the person picks another (the classic room default). */
export const DEFAULT_SHARE_QUALITY: StreamQuality = "1080p15";

interface Profile {
  readonly capture: {
    readonly width: number;
    readonly height: number;
    readonly frameRate: number;
  };
  readonly encoding: VideoEncoding;
  /** The lower simulcast layer; `null` keeps the SDK's half-size layer. */
  readonly layer: {
    readonly width: number;
    readonly height: number;
    readonly encoding: VideoEncoding;
  } | null;
  /** 1080p60 is chosen for motion, so it keeps frames and gives up resolution under pressure. */
  readonly motion: boolean;
}

const PROFILES: Readonly<Record<StreamQuality, Profile>> = {
  // ScreenSharePresets.h720fps15 / h1080fps15 / h1080fps30.
  "720p15": {
    capture: { width: 1280, height: 720, frameRate: 15 },
    encoding: { maxBitrate: 1_500_000, maxFramerate: 15, priority: "medium" },
    layer: null,
    motion: false,
  },
  "1080p15": {
    capture: { width: 1920, height: 1080, frameRate: 15 },
    encoding: { maxBitrate: 2_500_000, maxFramerate: 15, priority: "medium" },
    layer: null,
    motion: false,
  },
  "1080p30": {
    capture: { width: 1920, height: 1080, frameRate: 30 },
    encoding: { maxBitrate: 5_000_000, maxFramerate: 30, priority: "medium" },
    layer: null,
    motion: false,
  },
  // Twice the frames of 1080p30 at about 1.6 times its bitrate; the layer is h720fps30.
  "1080p60": {
    capture: { width: 1920, height: 1080, frameRate: 60 },
    encoding: { maxBitrate: 8_000_000, maxFramerate: 60, priority: "medium" },
    layer: {
      width: 1280,
      height: 720,
      encoding: { maxBitrate: 2_000_000, maxFramerate: 30, priority: "medium" },
    },
    motion: true,
  },
};

/** `value` as a quality this client knows, or `null`. */
export function knownStreamQuality(value: string | null): StreamQuality | null {
  return STREAM_QUALITIES.find((quality) => quality === value) ?? null;
}

/**
 * The getDisplayMedia attempts for a share, in order; the next one runs only after the browser
 * rejects the constraints of the one before. Tab audio comes first (capturing system audio while
 * sharing a whole screen feeds the speakers back into the call on Windows), then video only, and
 * finally video without size or frame-rate constraints. Safari 17 and iOS 17+ start unconstrained,
 * matching the SDK's exception: accepting a size can silently produce a low-resolution capture.
 */
export function screenCaptureAttempts(
  quality: StreamQuality,
  browser?: ReturnType<typeof getBrowser>,
): ScreenShareCaptureOptions[] {
  const { capture, motion } = PROFILES[quality];

  const base = (audio: boolean): ScreenShareCaptureOptions => ({
    contentHint: motion ? "motion" : "text",
    surfaceSwitching: "include",
    systemAudio: "exclude",
    audio,
  });

  if (
    (browser?.name === "Safari" && browser.version.startsWith("17.")) ||
    (browser?.os === "iOS" && Number(browser.osVersion?.split(".")[0]) >= 17)
  ) {
    return [base(true), base(false)];
  }

  return [
    { ...base(true), resolution: capture },
    { ...base(false), resolution: capture },
    // Zero means uncapped to the SDK; omitting resolution would reinstate its 1080p30 default.
    { ...base(false), resolution: { width: 0, height: 0 } },
  ];
}

/** A rejected or ignored text hint falls back on the captured track, without reopening the picker. */
export function applyScreenContentHint(
  track: Pick<MediaStreamTrack, "contentHint">,
  quality: StreamQuality,
): void {
  if (!("contentHint" in track)) {
    return;
  }

  const hint = PROFILES[quality].motion ? "motion" : "text";

  try {
    track.contentHint = hint;

    if (track.contentHint === hint || hint === "motion") {
      return;
    }
  } catch {
    // Some implementations reject hints rather than ignoring them.
  }

  try {
    track.contentHint = "detail";
  } catch {
    // Hints are advisory and must not stop a successful capture.
  }
}

/** The SDK's own `VideoPreset`, passed in so this module doesn't load the SDK. */
type VideoPresetClass = new (
  width: number,
  height: number,
  maxBitrate: number,
  maxFramerate?: number,
  priority?: RTCPriorityType,
) => VideoPreset;

/**
 * Publish options for a share at `quality`. Shared audio is usually music or video rather than
 * speech, and DTX chops it, so a share publishes without DTX. Options left out inherit the room's
 * publish defaults (simulcast on, "maintain-resolution" for readable code and slides).
 */
export function screenPublishOptions(
  quality: StreamQuality,
  Preset: VideoPresetClass,
): TrackPublishOptions {
  const { encoding, layer, motion } = PROFILES[quality];

  const options: TrackPublishOptions = {
    dtx: false,
    screenShareEncoding: { ...encoding },
    degradationPreference: motion ? "maintain-framerate" : "maintain-resolution",
  };

  if (layer !== null) {
    options.screenShareSimulcastLayers = [
      new Preset(
        layer.width,
        layer.height,
        layer.encoding.maxBitrate,
        layer.encoding.maxFramerate,
        layer.encoding.priority,
      ),
    ];
  }

  return options;
}
