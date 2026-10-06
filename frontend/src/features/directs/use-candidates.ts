import { useEffect, useState } from "react";
import type { DirectCandidate } from "../../gen/DirectCandidate.ts";
import { directs } from "../../sync/directs.ts";

/** The candidate list as a picker sees it. */
export interface CandidatesState {
  /** `null` until the first answer. */
  readonly candidates: readonly DirectCandidate[] | null;
  readonly error: string | null;
}

/** The last answer, so a picker reopened in the same session lists people at once. */
let cached: readonly DirectCandidate[] | null = null;

/**
 * Everyone the viewer can message, fetched each time a picker opens (`active`), showing the
 * previous answer meanwhile. Profiles land in the store, so names and avatars resolve there.
 */
export function useCandidates(active: boolean): CandidatesState {
  const [state, setState] = useState<CandidatesState>({ candidates: cached, error: null });

  useEffect(() => {
    if (!active) {
      return;
    }

    let current = true;

    directs.candidates().then(
      (candidates) => {
        cached = candidates;

        if (current) {
          setState({ candidates, error: null });
        }
      },
      (error: Error) => {
        if (current) {
          setState((held) => ({ candidates: held.candidates, error: error.message }));
        }
      },
    );

    return () => {
      current = false;
    };
  }, [active]);

  return state;
}
