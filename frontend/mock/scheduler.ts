/**
 * The timers the mock runs on (simulation, bot streaming, keep-alive pings). Injectable so tests
 * drive time by hand instead of waiting.
 */
export interface Scheduler {
  /** Runs `run` after `delayMs`; the returned id cancels it. */
  schedule(delayMs: number, run: () => void): number;
  cancel(id: number): void;
}

/** The real clock: `setTimeout` underneath. */
export function realScheduler(): Scheduler {
  const handles = new Map<number, ReturnType<typeof setTimeout>>();
  let nextId = 1;

  return {
    schedule(delayMs, run) {
      const id = nextId++;

      handles.set(
        id,
        setTimeout(() => {
          handles.delete(id);
          run();
        }, delayMs),
      );

      return id;
    },
    cancel(id) {
      clearTimeout(handles.get(id));
      handles.delete(id);
    },
  };
}

/** A scheduler and clock that only move when told to, for tests. */
export interface ManualScheduler extends Scheduler {
  /** The manual clock, in ms since the epoch; pass it as the server's `now`. */
  now(): number;
  /** Moves the clock forward, running every timer that falls due, in order. */
  advance(ms: number): void;
  /** Timers still waiting. */
  pending(): number;
}

interface ManualTimer {
  readonly id: number;
  readonly at: number;
  readonly run: () => void;
}

export function manualScheduler(startMs = Date.UTC(2026, 9, 6, 15, 0, 0)): ManualScheduler {
  let clock = startMs;
  let nextId = 1;
  let timers: ManualTimer[] = [];

  const takeDue = (until: number): ManualTimer | undefined => {
    let due: ManualTimer | undefined;

    for (const timer of timers) {
      if (timer.at > until) continue;

      if (due === undefined || timer.at < due.at || (timer.at === due.at && timer.id < due.id)) {
        due = timer;
      }
    }

    if (due !== undefined) {
      const taken = due;

      timers = timers.filter((timer) => timer !== taken);
    }

    return due;
  };

  return {
    now: () => clock,
    schedule(delayMs, run) {
      const id = nextId++;

      timers.push({ id, at: clock + Math.max(0, delayMs), run });

      return id;
    },
    cancel(id) {
      timers = timers.filter((timer) => timer.id !== id);
    },
    advance(ms) {
      const until = clock + ms;

      for (let due = takeDue(until); due !== undefined; due = takeDue(until)) {
        clock = Math.max(clock, due.at);
        due.run();
      }

      clock = until;
    },
    pending: () => timers.length,
  };
}
