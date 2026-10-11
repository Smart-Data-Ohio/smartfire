/**
 * What the microphone encodes at. A room's voice bitrate arrives with each huddle grant
 * (`audioBitrateTarget`, in bits per second, already the server's effective value). A grant
 * without a usable target keeps the SDK's `AudioPresets.music`, 48 kbps, which is what every call
 * published at before rooms had the setting. Only the microphone takes the room's target: a
 * screen share's audio keeps the room defaults.
 */
import type { AudioPreset, TrackPublishOptions } from "livekit-client";

/** `AudioPresets.music`'s target, written out so an SDK upgrade can't quietly change it. */
export const DEFAULT_AUDIO_BITRATE = 48_000;

/** The microphone's encoding for a grant's `audioBitrateTarget`. */
export function microphonePreset(target: number | null | undefined): AudioPreset {
  const bitrate = target ?? DEFAULT_AUDIO_BITRATE;

  return {
    maxBitrate: Number.isSafeInteger(bitrate) && bitrate > 0 ? bitrate : DEFAULT_AUDIO_BITRATE,
  };
}

/**
 * The microphone's publish options. They only set the encoding target: DTX, RED and the rest
 * still come from the room's publish defaults, which the SDK merges underneath.
 */
export function microphonePublishOptions(target: number | null | undefined): TrackPublishOptions {
  return { audioPreset: microphonePreset(target) };
}
