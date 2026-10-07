/**
 * One change at a time per key: an optimistic change waits for the one before it on the same
 * item to settle, so each starts from (and rolls back to) a copy no other change is still
 * holding. Changes to different keys run side by side.
 */
import { Effect, Semaphore } from "effect";

export interface KeyedSerial<K> {
  /** Runs `effect` once every earlier `run` with the same key has finished. */
  readonly run: <A, E, R>(key: K, effect: Effect.Effect<A, E, R>) => Effect.Effect<A, E, R>;
}

interface Lock {
  readonly semaphore: Semaphore.Semaphore;
  waiting: number;
}

export function keyedSerial<K>(): KeyedSerial<K> {
  const locks = new Map<K, Lock>();

  return {
    run: (key, effect) =>
      Effect.suspend(() => {
        const lock = locks.get(key) ?? { semaphore: Semaphore.makeUnsafe(1), waiting: 0 };

        locks.set(key, lock);
        lock.waiting += 1;

        return lock.semaphore.withPermit(effect).pipe(
          Effect.ensuring(
            Effect.sync(() => {
              lock.waiting -= 1;

              if (lock.waiting === 0) {
                locks.delete(key);
              }
            }),
          ),
        );
      }),
  };
}
