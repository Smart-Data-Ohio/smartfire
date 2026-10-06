import { useEffect } from "react";
import { store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";

/** Ids asked for already (or on their way), so each person is fetched once. */
const requested = new Set<number>();

let queued: number[] = [];

function flush(): void {
  const ids = queued;

  queued = [];

  if (ids.length === 0) {
    return;
  }

  void actions.ensureUsers(ids).catch(() => {
    for (const id of ids) {
      requested.delete(id);
    }
  });
}

/**
 * Makes sure these people are in the store: a reply indicator names repliers whose messages
 * aren't on the page. Unknown ids from every caller in one tick go out as one request.
 */
export function useEnsureUsers(ids: readonly number[]): void {
  const key = ids.join(",");

  useEffect(() => {
    const known = store.getState().users;

    const missing = key
      .split(",")
      .filter((part) => part !== "")
      .map(Number)
      .filter((id) => known[id] === undefined && !requested.has(id));

    if (missing.length === 0) {
      return;
    }

    for (const id of missing) {
      requested.add(id);
    }

    if (queued.length === 0) {
      queueMicrotask(flush);
    }

    queued.push(...missing);
  }, [key]);
}
