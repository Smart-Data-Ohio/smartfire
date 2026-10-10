import { loadForUpdate } from "../../service-worker/update-required.ts";
import type { HighlightResult } from "./kinds.ts";
import type { HighlightRequest } from "./worker.ts";

/** Colours one block's source, as the lazy module (tokenize.ts) does. */
export type Highlight = (source: string, label: string | undefined) => Promise<HighlightResult>;

/**
 * The entry chunks' side of code colouring: it holds no highlighter, only a way to load one on
 * the first block that needs it, and the results so far, so a row the timeline unmounts and
 * mounts again is coloured before it paints, without asking twice.
 */
export interface Tokenizer {
  /** The finished result for this block, when there is one. */
  readonly cached: (source: string, label: string | undefined) => HighlightResult | undefined;
  readonly tokenize: Highlight;
}

/** Enough for a long timeline's code blocks; the oldest is dropped first. */
const CACHE_LIMIT = 500;

const key = (source: string, label: string | undefined) => `${label ?? ""}\u0000${source}`;

/** A tokenizer that calls `load` once, on its first block (again after a failed load). */
export function createTokenizer(load: () => Promise<Highlight>): Tokenizer {
  let highlight: Promise<Highlight> | undefined;
  const results = new Map<string, HighlightResult>();
  const running = new Map<string, Promise<HighlightResult>>();

  const loaded = () => {
    highlight ??= load().catch((error: Error) => {
      highlight = undefined;

      throw error;
    });

    return highlight;
  };

  return {
    cached: (source, label) => results.get(key(source, label)),
    tokenize: (source, label) => {
      const id = key(source, label);
      const done = results.get(id);

      if (done !== undefined) {
        return Promise.resolve(done);
      }

      const pending =
        running.get(id) ??
        loaded()
          .then((run) => run(source, label))
          .then(
            (result) => {
              running.delete(id);
              results.set(id, result);

              if (results.size > CACHE_LIMIT) {
                results.delete(results.keys().next().value ?? id);
              }

              return result;
            },
            (error: Error) => {
              running.delete(id);

              throw error;
            },
          );

      running.set(id, pending);

      return pending;
    },
  };
}

/** No answer from the worker by then (a grammar that never loads, say): leave the block plain. */
const WORKER_TIMEOUT_MS = 15_000;

interface Waiting {
  readonly resolve: (result: HighlightResult) => void;
  readonly reject: (error: Error) => void;
  readonly timer: number;
}

interface HighlightResponse {
  readonly id: number;
  readonly result: HighlightResult | null;
}

/** Colouring in the code-highlight worker, started on the first block and restarted after a fault. */
function workerHighlight(): Highlight {
  let worker: Worker | null = null;
  let nextId = 0;
  const waiting = new Map<number, Waiting>();

  const stop = () => {
    worker?.terminate();
    worker = null;

    for (const request of waiting.values()) {
      window.clearTimeout(request.timer);
      request.reject(new Error("The code highlighter stopped"));
    }

    waiting.clear();
  };

  const start = () => {
    const started = new Worker(new URL("./worker.ts", import.meta.url), {
      type: "module",
      name: "code-highlight",
    });

    started.addEventListener("message", (event: MessageEvent<HighlightResponse>) => {
      const { id, result } = event.data;
      const request = waiting.get(id);

      if (request === undefined) {
        return;
      }

      window.clearTimeout(request.timer);
      waiting.delete(id);

      if (result === null) {
        request.reject(new Error("The code highlighter failed"));
      } else {
        request.resolve(result);
      }
    });

    started.addEventListener("error", stop);
    started.addEventListener("messageerror", stop);

    return started;
  };

  return (source, label) =>
    new Promise((resolve, reject) => {
      worker ??= start();
      nextId += 1;

      const request: HighlightRequest = { id: nextId, source, label };

      waiting.set(request.id, {
        resolve,
        reject,
        timer: window.setTimeout(stop, WORKER_TIMEOUT_MS),
      });

      worker.postMessage(request);
    });
}

/**
 * The same module on this thread, for tests (jsdom has no Worker). Production builds leave it out,
 * so the highlighter ships once, in the worker's chunks.
 */
const inThread = import.meta.env.PROD ? null : () => import("./tokenize.ts");

/** The worker; without one (a test, an engine too old for module workers) blocks stay plain. */
async function loadHighlight(): Promise<Highlight> {
  if ("Worker" in globalThis) {
    return workerHighlight();
  }

  if (inThread === null) {
    throw new Error("Code colouring needs a worker");
  }

  const { highlight } = await loadForUpdate(inThread);

  return highlight;
}

export const tokenizer: Tokenizer = createTokenizer(loadHighlight);
