/**
 * The live microphone level (the classic `lib/huddle/meter.js`). An `AnalyserNode` over the
 * microphone track in its own `AudioContext`, read the way LiveKit's `createAudioAnalyser`
 * reads it: frequency-domain energy, which sits well below 1 for ordinary speech. The server's
 * `audioLevel` only moves while somebody counts as speaking, so it can't drive a meter at rest.
 * Closing the context frees the microphone tap, so `stop` always runs when the track changes.
 */

/** Maps conversational level onto the top half of a 0–100 meter without pinning it. */
const METER_GAIN = 250;

export function scaleMeterVolume(volume: number): number {
  if (!Number.isFinite(volume) || volume <= 0) {
    return 0;
  }

  return Math.min(100, Math.round(volume * METER_GAIN));
}

export interface LevelMeter {
  /** The track it listens to; a new track needs a new meter. */
  readonly track: MediaStreamTrack;
  /** 0–100; 0 once the track has ended. */
  level(): number;
  stop(): void;
}

/** A meter over `track`, or `null` where Web Audio is missing or the track isn't live. */
export function createLevelMeter(track: MediaStreamTrack): LevelMeter | null {
  if (track.readyState !== "live" || globalThis.AudioContext === undefined) {
    return null;
  }

  let context: AudioContext;

  try {
    context = new AudioContext();
  } catch {
    return null;
  }

  const analyser = context.createAnalyser();

  analyser.minDecibels = -100;
  analyser.maxDecibels = -80;
  analyser.fftSize = 2048;
  analyser.smoothingTimeConstant = 0.8;
  context.createMediaStreamSource(new MediaStream([track])).connect(analyser);

  const data = new Uint8Array(analyser.frequencyBinCount);
  let stopped = false;

  const stop = (): void => {
    if (stopped) {
      return;
    }

    stopped = true;
    void context.close().catch(() => undefined);
  };

  return {
    track,
    level: () => {
      if (stopped || track.readyState !== "live") {
        stop();

        return 0;
      }

      analyser.getByteFrequencyData(data);

      let sum = 0;

      for (const amplitude of data) {
        sum += (amplitude / 255) ** 2;
      }

      return scaleMeterVolume(Math.sqrt(sum / data.length));
    },
    stop,
  };
}
