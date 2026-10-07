import { useEffect, useState } from "react";

export type Loaded<T> =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string; readonly missing: boolean }
  | { readonly status: "ready"; readonly value: T };

type Keyed<T> = Loaded<T> & { readonly key: string };

/**
 * Reads `read()` whenever `key` changes (or `reload` runs), dropping a reply that lands after a
 * newer read started. A reload keeps showing what it has meanwhile; `set` replaces the value with
 * one a write returned.
 */
export function useLoad<T>(key: string, read: () => Promise<T>) {
  const [state, setState] = useState<Keyed<T>>({ status: "loading", key });
  const [tries, setTries] = useState(0);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `key` names what `read` reads; a retry (`tries`) reads it again
  useEffect(() => {
    let live = true;

    setState((current) =>
      current.key === key && current.status === "ready" ? current : { status: "loading", key },
    );

    read().then(
      (value) => {
        if (live) setState({ status: "ready", value, key });
      },
      (failure: Error & { readonly tag?: string }) => {
        if (live) {
          setState({
            status: "error",
            message: failure.message,
            missing: failure.tag === "NotFound" || failure.tag === "Forbidden",
            key,
          });
        }
      },
    );

    return () => {
      live = false;
    };
  }, [key, tries]);

  const shown: Loaded<T> = state.key === key ? state : { status: "loading" };

  return {
    state: shown,
    reload: () => setTries((count) => count + 1),
    set: (value: T) => setState({ status: "ready", value, key }),
  };
}
