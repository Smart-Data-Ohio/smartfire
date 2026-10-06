/**
 * S2's share of the simulation: now and then someone reacts to a recent message or replies in
 * an open thread (typing on `thread:<id>` first). It runs on its own PRNG and timer loop, so
 * the S1 simulation's sequence is unchanged.
 */
import type { Random } from "../random.ts";
import type { Scheduler } from "../scheduler.ts";

const THREAD_LINES = [
  "Agreed, let's go with that.",
  "I can pick this up tomorrow morning.",
  "Added a couple of notes to the doc.",
  "Same result on my side.",
  "Makes sense to me 👍",
  "Let's check again after the next deploy.",
];

const AMBIENT_REACTIONS = ["👍", "🎉", "🔥", "👀", "❤️", "😂", "💯"];

/** What the ambient loop needs from the server. */
export interface AmbientHost {
  /** Recent root messages people might react to, with who could react. */
  reactable(): readonly { readonly messageId: number; readonly people: readonly number[] }[];
  /** Threads open to replies, with who could reply. */
  threads(): readonly { readonly threadId: number; readonly people: readonly number[] }[];
  react(messageId: number, userId: number, content: string): void;
  typing(threadId: number, userId: number, on: boolean): void;
  reply(threadId: number, userId: number, markdown: string): void;
  paused(): boolean;
}

/** The ambient loop. */
export interface Ambient {
  start(): void;
  stop(): void;
}

/** Creates the ambient loop. */
export function createAmbient(host: AmbientHost, scheduler: Scheduler, random: Random): Ambient {
  const timers = new Set<number>();
  let running = false;

  const later = (delayMs: number, run: () => void) => {
    const id = scheduler.schedule(delayMs, () => {
      timers.delete(id);
      run();
    });

    timers.add(id);
  };

  const react = () => {
    const targets = host.reactable().filter((target) => target.people.length > 0);

    if (targets.length === 0) return;

    const target = random.pick(targets);

    host.react(target.messageId, random.pick(target.people), random.pick(AMBIENT_REACTIONS));
  };

  const reply = () => {
    const targets = host.threads().filter((target) => target.people.length > 0);

    if (targets.length === 0) return;

    const target = random.pick(targets);
    const userId = random.pick(target.people);

    host.typing(target.threadId, userId, true);
    later(random.int(2000, 4000), () => {
      host.typing(target.threadId, userId, false);
      host.reply(target.threadId, userId, random.pick(THREAD_LINES));
    });
  };

  const tick = () => {
    if (!running) return;

    if (!host.paused()) {
      if (random.chance(0.6)) {
        react();
      } else {
        reply();
      }
    }

    later(random.int(15_000, 40_000), tick);
  };

  return {
    start() {
      if (running) return;

      running = true;
      later(random.int(15_000, 40_000), tick);
    },
    stop() {
      running = false;

      for (const id of timers) scheduler.cancel(id);

      timers.clear();
    },
  };
}
