/**
 * Whether a read's reply is still fresh enough to land. Each record (by key) has a mark that moves
 * whenever anything changes it: a write starting or settling, or any store writer changing the
 * record (a sync event, a resync, another page's reply). A read notes the clock when it starts and
 * lands only if its record's mark hasn't moved since and no write on it is in flight. Otherwise
 * its reply may predate a change, whatever order the replies arrive in, so the caller refetches
 * once the writes settle. Writes land only the fields they changed, never a whole snapshot.
 */
export interface RecordFreshness {
  /** The ticket for a read starting now. */
  readonly startRead: () => number;
  /** Whether a read that started at `ticket` may land record `key`. */
  readonly fresh: (ticket: number, key: number) => boolean;
  /** Notes that something changed record `key`. */
  readonly touch: (key: number) => void;
  /** Starts a write on record `key`; call the result once, when it settles either way. */
  readonly startWrite: (key: number) => () => void;
  /** Resolves when no write on record `key` is in flight. */
  readonly settled: (key: number) => Promise<void>;
}

/** Fresh bookkeeping, with nothing changed and nothing in flight. */
export function recordFreshness(): RecordFreshness {
  let clock = 0;
  const changedAt = new Map<number, number>();
  const writing = new Map<number, number>();
  const waiting = new Map<number, (() => void)[]>();

  const touch = (key: number) => {
    clock += 1;
    changedAt.set(key, clock);
  };

  const settled = (key: number) =>
    (writing.get(key) ?? 0) === 0
      ? Promise.resolve()
      : new Promise<void>((resolve) => {
          waiting.set(key, [...(waiting.get(key) ?? []), resolve]);
        });

  const startWrite = (key: number) => {
    let done = false;

    writing.set(key, (writing.get(key) ?? 0) + 1);
    touch(key);

    return () => {
      if (done) return;

      done = true;

      const left = (writing.get(key) ?? 1) - 1;

      touch(key);

      if (left > 0) {
        writing.set(key, left);

        return;
      }

      writing.delete(key);

      const resolvers = waiting.get(key) ?? [];

      waiting.delete(key);

      for (const resolve of resolvers) {
        resolve();
      }
    };
  };

  return {
    startRead: () => clock,
    fresh: (ticket, key) => (changedAt.get(key) ?? 0) <= ticket && !writing.has(key),
    touch,
    startWrite,
    settled,
  };
}

/** Whether two values carry the same JSON, so an identical refresh doesn't count as a change. */
function sameJson<T>(left: T | undefined, right: T | undefined): boolean {
  return left === right || JSON.stringify(left) === JSON.stringify(right);
}

/** Touches every record whose value changed between two versions of a by-id table. */
export function touchChanged<T>(
  freshness: RecordFreshness,
  before: Readonly<Record<number, T>>,
  after: Readonly<Record<number, T>>,
): void {
  if (before === after) return;

  for (const key of Object.keys(after)) {
    const id = Number(key);

    if (!sameJson(before[id], after[id])) freshness.touch(id);
  }
}
