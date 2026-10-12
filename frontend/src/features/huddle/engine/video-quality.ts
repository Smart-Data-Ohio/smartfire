import type { AdaptiveStreamSettings, RemoteTrackPublication } from "livekit-client";

export const VIDEO_ADAPTIVE_STREAM = {
  pixelDensity: "screen",
} satisfies AdaptiveStreamSettings;

/**
 * LiveKit 2.22.3 caps HIGH by its adaptive dimensions. Ignore that cap while focused, without
 * disabling the track's observers. Keep their latest dimensions for the return to Auto.
 * These protected fields and the internal emitter are pinned by the real SDK tests.
 */
export function focusScreenPublication(publication: RemoteTrackPublication): () => void {
  const emit = publication.emitTrackUpdate;

  publication.emitTrackUpdate = () => {
    const dimensions: unknown = Reflect.get(publication, "videoDimensionsAdaptiveStream");

    Object.assign(publication, { videoDimensionsAdaptiveStream: undefined });

    try {
      emit.call(publication);
    } finally {
      Object.assign(publication, { videoDimensionsAdaptiveStream: dimensions });
    }
  };

  return () => {
    publication.emitTrackUpdate = emit;
  };
}
