import { useEffect, useState } from "react";
import type { Switcher } from "../../gen/Switcher.ts";
import { directs } from "../../sync/directs.ts";

/** The server's switcher catalogue: `null` until it arrives (local results show meanwhile). */
export interface CatalogueState {
  readonly catalogue: Switcher | null;
  readonly loading: boolean;
}

let cached: Switcher | null = null;

/**
 * `GET /switcher` each time the switcher opens (`active`), the previous answer standing in until
 * the new one lands. A failure keeps whatever there was: the sidebar's rooms still search.
 */
export function useCatalogue(active: boolean): CatalogueState {
  const [state, setState] = useState<CatalogueState>({ catalogue: cached, loading: false });

  useEffect(() => {
    if (!active) {
      return;
    }

    let current = true;

    setState((held) => ({ ...held, loading: true }));

    directs.switcher().then(
      (catalogue) => {
        cached = catalogue;

        if (current) {
          setState({ catalogue, loading: false });
        }
      },
      () => {
        if (current) {
          setState((held) => ({ ...held, loading: false }));
        }
      },
    );

    return () => {
      current = false;
    };
  }, [active]);

  return state;
}
