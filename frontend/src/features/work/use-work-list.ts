import { useEffect, useRef, useState } from "react";
import type { WorkFilter } from "../../gen/WorkFilter.ts";
import type { WorkList } from "../../gen/WorkList.ts";
import { actions } from "../../sync/runtime.ts";
import { toast } from "../../ui/toast-store.ts";
import type { PagedState } from "../destinations/paged-list.tsx";

interface Loaded {
  readonly filter: WorkFilter;
  readonly status: "loading" | "ready" | "error";
  readonly list: WorkList | null;
  readonly error: string | null;
}

export interface WorkListView extends PagedState {
  readonly list: WorkList | null;
}

const ignore = () => undefined;

/**
 * One filter of the work list. It isn't paged and has no live updates (contract S4): it loads
 * whenever the page shows it, the tab is switched to it, or the browser tab comes back into view.
 * A filter seen before shows its last rows at once while the fresh ones load; a failed refresh
 * keeps them and says so in a toast, and a failed first load shows the error with Retry.
 */
export function useWorkList(filter: WorkFilter): WorkListView {
  const cache = useRef(new Map<WorkFilter, WorkList>());
  const [attempt, setAttempt] = useState(0);

  const [loaded, setLoaded] = useState<Loaded>(() => ({
    filter,
    status: "loading",
    list: null,
    error: null,
  }));

  // biome-ignore lint/correctness/useExhaustiveDependencies: each Retry (`attempt`) loads again
  useEffect(() => {
    let current = true;

    const load = () => {
      const cached = cache.current.get(filter) ?? null;

      setLoaded({
        filter,
        status: cached === null ? "loading" : "ready",
        list: cached,
        error: null,
      });
      actions.work.list(filter).then(
        (list) => {
          if (current) {
            cache.current.set(filter, list);
            setLoaded({ filter, status: "ready", list, error: null });
          }
        },
        (error: Error) => {
          if (!current) {
            return;
          }

          if (cache.current.has(filter)) {
            toast({
              title: "Couldn't refresh your work",
              description: error.message,
              tone: "danger",
            });
          } else {
            setLoaded({ filter, status: "error", list: null, error: error.message });
          }
        },
      );
    };

    const onVisible = () => {
      if (document.visibilityState === "visible") {
        load();
      }
    };

    load();
    document.addEventListener("visibilitychange", onVisible);

    return () => {
      current = false;
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [filter, attempt]);

  // The first render after a tab switch, before the effect: the new filter's cache, or loading.
  const view: Loaded =
    loaded.filter === filter
      ? loaded
      : {
          filter,
          status: cache.current.has(filter) ? "ready" : "loading",
          list: cache.current.get(filter) ?? null,
          error: null,
        };

  return {
    status: view.status,
    list: view.list,
    error: view.error,
    loadingMore: false,
    hasMore: false,
    loadMore: ignore,
    reload: () => setAttempt((count) => count + 1),
  };
}
