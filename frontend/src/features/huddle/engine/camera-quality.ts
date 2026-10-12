import type {
  TrackPublishOptions,
  VideoCaptureOptions,
  VideoEncoding,
  VideoPreset,
  VideoResolution,
} from "livekit-client";

export const CAMERA_QUALITIES = ["720p30", "1080p30", "1080p60"] as const;

export type CameraQuality = (typeof CAMERA_QUALITIES)[number];

export const DEFAULT_CAMERA_QUALITY: CameraQuality = "720p30";

export type CameraCapabilities = Pick<MediaTrackCapabilities, "width" | "height" | "frameRate">;

type Layer = Readonly<Pick<VideoPreset, "width" | "height" | "encoding">>;

interface Profile {
  readonly capture: Readonly<Required<Pick<VideoResolution, "width" | "height" | "frameRate">>>;
  readonly encoding: Readonly<VideoEncoding>;
  readonly layers: readonly Layer[];
}

// The lower layers keep the pinned SDK's h180, h360, and h720 camera encodings.
const LOW_LAYER: Layer = {
  width: 320,
  height: 180,
  encoding: { maxBitrate: 160_000, maxFramerate: 20 },
};

const MEDIUM_LAYER: Layer = {
  width: 640,
  height: 360,
  encoding: { maxBitrate: 450_000, maxFramerate: 20 },
};

const HD_LAYER: Layer = {
  width: 1280,
  height: 720,
  encoding: { maxBitrate: 1_700_000, maxFramerate: 30 },
};

const PROFILES: Readonly<Record<CameraQuality, Profile>> = {
  "720p30": {
    capture: { width: 1280, height: 720, frameRate: 30 },
    encoding: { maxBitrate: 1_700_000, maxFramerate: 30 },
    layers: [LOW_LAYER, MEDIUM_LAYER],
  },
  "1080p30": {
    capture: { width: 1920, height: 1080, frameRate: 30 },
    encoding: { maxBitrate: 3_000_000, maxFramerate: 30 },
    layers: [MEDIUM_LAYER, HD_LAYER],
  },
  "1080p60": {
    capture: { width: 1920, height: 1080, frameRate: 60 },
    encoding: { maxBitrate: 6_000_000, maxFramerate: 60 },
    layers: [MEDIUM_LAYER, HD_LAYER],
  },
};

export function knownCameraQuality(value: string | null): CameraQuality | null {
  return CAMERA_QUALITIES.find((quality) => quality === value) ?? null;
}

/** Higher modes need all reported bounds; ranges cannot prove a combined capture mode. */
export function supportedCameraQualities(
  capabilities: CameraCapabilities | null,
): readonly CameraQuality[] {
  return CAMERA_QUALITIES.filter((quality) => {
    if (quality === DEFAULT_CAMERA_QUALITY) {
      // Even a smaller camera can satisfy this advisory fallback request at its own size.
      return true;
    }

    const { width, height, frameRate } = PROFILES[quality].capture;

    return (
      (capabilities?.width?.max ?? 0) >= width &&
      (capabilities?.height?.max ?? 0) >= height &&
      (capabilities?.frameRate?.max ?? 0) >= frameRate
    );
  });
}

/** Numeric size constraints and an ideal frame rate let the browser choose a supported mode. */
export function cameraCaptureOptions(
  quality: CameraQuality,
  deviceId?: string,
): VideoCaptureOptions {
  const capture = PROFILES[quality].capture;
  const options: VideoCaptureOptions = {
    resolution: { ...capture },
    frameRate: { ideal: capture.frameRate },
  };

  if (deviceId !== undefined && deviceId !== "") {
    options.deviceId = { ideal: deviceId };
  }

  return options;
}

/** Passed in so choosing a camera quality does not load the media SDK. */
type VideoPresetClass = new (
  width: number,
  height: number,
  maxBitrate: number,
  maxFramerate?: number,
  priority?: RTCPriorityType,
) => VideoPreset;

export function cameraPublishOptions(
  quality: CameraQuality,
  Preset: VideoPresetClass,
): TrackPublishOptions {
  const { encoding, layers } = PROFILES[quality];

  return {
    videoEncoding: { ...encoding },
    simulcast: true,
    videoSimulcastLayers: layers.map(
      ({ width, height, encoding: layer }) =>
        new Preset(width, height, layer.maxBitrate, layer.maxFramerate, layer.priority),
    ),
    degradationPreference: "maintain-framerate",
  };
}
