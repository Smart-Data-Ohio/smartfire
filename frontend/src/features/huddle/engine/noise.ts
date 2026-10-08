import { loadForUpdate } from "../../../service-worker/update-required.ts";
/**
 * RNNoise microphone noise suppression (the classic `lib/huddle_noise_suppressor.js`). LiveKit
 * Cloud's Krisp filter isn't available to a self-hosted deployment, so the call runs RNNoise as
 * a LiveKit audio `TrackProcessor`: the model runs in an AudioWorklet off the main thread, and
 * the WebAssembly ships from this origin rather than a CDN.
 *
 * RNNoise is trained for 48 kHz mono speech: when LiveKit hands over a context at another rate,
 * a dedicated 48 kHz context is opened instead of feeding the model the wrong sample rate.
 */

import wasmUrl from "@sapphi-red/web-noise-suppressor/rnnoise.wasm?url";
import simdWasmUrl from "@sapphi-red/web-noise-suppressor/rnnoise_simd.wasm?url";
import workletUrl from "@sapphi-red/web-noise-suppressor/rnnoiseWorklet.js?url";
import type { AudioProcessorOptions, Track, TrackProcessor } from "livekit-client";

const RNNOISE_SAMPLE_RATE = 48_000;

type Suppressor = typeof import("@sapphi-red/web-noise-suppressor");

let suppressor: Promise<Suppressor> | null = null;

let wasm: Promise<ArrayBuffer> | null = null;

const contextsWithWorklet = new WeakSet<BaseAudioContext>();

function loadSuppressor(): Promise<Suppressor> {
  suppressor ??= loadForUpdate(() => import("@sapphi-red/web-noise-suppressor")).catch(
    (error: Error) => {
      suppressor = null;

      throw error;
    },
  );

  return suppressor;
}

/** The filter needs AudioWorklet, Web Audio and WebAssembly. */
export function noiseSuppressionSupported(): boolean {
  return (
    globalThis.AudioWorkletNode !== undefined &&
    globalThis.AudioContext !== undefined &&
    globalThis.WebAssembly !== undefined &&
    "instantiate" in WebAssembly
  );
}

/** A failure that means this browser can't run the filter at all (latched off for the page). */
export class NoiseSuppressionUnsupported extends Error {
  override readonly name = "NoiseSuppressionUnsupported";
}

function channels(track: MediaStreamTrack): number {
  return Math.min(Math.max(track.getSettings().channelCount ?? 1, 1), 2);
}

export class NoiseSuppressor implements TrackProcessor<Track.Kind.Audio, AudioProcessorOptions> {
  readonly name = "campfire-rnnoise";
  processedTrack?: MediaStreamTrack;
  #source: MediaStreamAudioSourceNode | null = null;
  #node: (AudioWorkletNode & { destroy(): void }) | null = null;
  #destination: MediaStreamAudioDestinationNode | null = null;
  #ownContext: AudioContext | null = null;
  #channelCount = 1;

  constructor() {
    if (!noiseSuppressionSupported()) {
      throw new NoiseSuppressionUnsupported("noise-suppression-unsupported");
    }
  }

  async init(options: AudioProcessorOptions): Promise<void> {
    try {
      await this.#start(options);
    } catch (error) {
      // A half-built graph would leave the microphone silent: unwind before LiveKit hears of it.
      await this.destroy();

      throw error;
    }
  }

  /**
   * LiveKit restarts the microphone track on unmute and device switches and hands the new track
   * over here. The graph survives that: only the source is rewired onto the new track, so the
   * worklet, its model and the context stay alive, and the publisher keeps the processed track
   * with no unfiltered burst. Anything the running graph can't take falls back to a rebuild.
   */
  async restart(options: AudioProcessorOptions): Promise<void> {
    if (await this.#rewire(options.track).catch(() => false)) {
      return;
    }

    await this.destroy();
    await this.init(options);
  }

  async destroy(): Promise<void> {
    try {
      this.#node?.destroy();
    } catch {
      // The worklet is already gone when its context closed first.
    }

    this.#source?.disconnect();
    this.#node?.disconnect();
    // LiveKit stops the track it publishes, not this destination's: stop it here.
    this.processedTrack?.stop();
    this.#source = null;
    this.#node = null;
    this.#destination = null;
    delete this.processedTrack;

    const own = this.#ownContext;

    this.#ownContext = null;

    if (own !== null && own.state !== "closed") {
      await own.close().catch(() => undefined);
    }
  }

  async #start(options: AudioProcessorOptions): Promise<void> {
    const { RnnoiseWorkletNode, loadRnnoise } = await loadSuppressor();
    const context = this.#context(options.audioContext);

    if (context.state === "suspended") {
      await context.resume().catch(() => undefined);
    }

    // A context that never starts would publish a silent track, which is worse than the raw
    // microphone: fail so `init` unwinds to it.
    if (context.state !== "running") {
      throw new Error("noise-suppression-context-not-running");
    }

    if (!contextsWithWorklet.has(context)) {
      await context.audioWorklet.addModule(workletUrl);
      contextsWithWorklet.add(context);
    }

    // The binary is structured-cloned into the worklet, so one fetch serves every restart.
    wasm ??= loadRnnoise({ url: wasmUrl, simdUrl: simdWasmUrl }).catch((error: Error) => {
      wasm = null;

      throw error;
    });

    const binary = await wasm;

    this.#channelCount = channels(options.track);
    this.#source = context.createMediaStreamSource(new MediaStream([options.track]));
    this.#node = new RnnoiseWorkletNode(context, {
      maxChannels: this.#channelCount,
      wasmBinary: binary.slice(0),
    });
    this.#destination = context.createMediaStreamDestination();
    this.#destination.channelCount = this.#channelCount;
    this.#source.connect(this.#node);
    this.#node.connect(this.#destination);

    const [processed] = this.#destination.stream.getAudioTracks();

    if (processed === undefined) {
      throw new Error("noise-suppression-produced-no-track");
    }

    this.processedTrack = processed;
  }

  async #rewire(track: MediaStreamTrack): Promise<boolean> {
    const node = this.#node;
    const source = this.#source;

    if (node === null || source === null || this.#destination === null) {
      return false;
    }

    if (this.processedTrack === undefined || this.processedTrack.readyState === "ended") {
      return false;
    }

    const context = node.context;

    if (context.state === "suspended" && context instanceof AudioContext) {
      await context.resume().catch(() => undefined);
    }

    // The node was built for one channel count; anything else needs a matching graph.
    if (context.state !== "running" || channels(track) !== this.#channelCount) {
      return false;
    }

    if (!(context instanceof AudioContext)) {
      return false;
    }

    source.disconnect();
    this.#source = context.createMediaStreamSource(new MediaStream([track]));
    this.#source.connect(node);

    return true;
  }

  #context(offered: AudioContext): AudioContext {
    if (offered.sampleRate === RNNOISE_SAMPLE_RATE && offered.state !== "closed") {
      return offered;
    }

    this.#ownContext = new AudioContext({
      sampleRate: RNNOISE_SAMPLE_RATE,
      latencyHint: "interactive",
    });

    return this.#ownContext;
  }
}
