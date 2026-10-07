import { useEffect, useRef, useState } from "react";

export type Loaded<T> =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string; readonly missing: boolean }
  | { readonly status: "ready"; readonly value: T };

type Keyed<T> = Loaded<T> & { readonly key: string };

/** Where a write's reply goes: shown if it's still the newest news for the screen it came from. */
export interface Landing<T> {
  readonly land: (value: T) => void;
  /** The write failed: read the screen again, since what it showed may have changed meanwhile. */
  readonly fail: () => void;
}

/**
 * Reads `read()` whenever `key` changes (or `reload` runs), keeping only the newest news: every
 * read and every write (`begin`) takes a turn, and a reply lands only while its turn is the latest
 * and its key is still the one shown. So a slow reply never replaces a newer one, and one for a
 * screen the viewer has left never lands on the screen they went to. A reload keeps showing what
 * it has meanwhile.
 */
export function useLoad<T>(key: string, read: () => Promise<T>) {
  const [state, setState] = useState<Keyed<T>>({ status: "loading", key });
  const [tries, setTries] = useState(0);
  // The latest turn, and the key reads are for. Touched only in effects and callbacks.
  const turn = useRef(0);
  const shownKey = useRef(key);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `key` names what `read` reads; a retry (`tries`) reads it again
  useEffect(() => {
    turn.current += 1;
    shownKey.current = key;

    const mine = turn.current;
    const current = () => turn.current === mine && shownKey.current === key;

    setState((held) =>
      held.key === key && held.status === "ready" ? held : { status: "loading", key },
    );

    read().then(
      (value) => {
        if (current()) setState({ status: "ready", value, key });
      },
      (failure: Error & { readonly tag?: string }) => {
        if (current()) {
          setState({
            status: "error",
            message: failure.message,
            missing: failure.tag === "NotFound" || failure.tag === "Forbidden",
            key,
          });
        }
      },
    );
  }, [key, tries]);

  // Unmounting ends every turn, so nothing lands afterwards.
  useEffect(
    () => () => {
      turn.current += 1;
    },
    [],
  );

  const shown: Loaded<T> = state.key === key ? state : { status: "loading" };
  const reload = () => setTries((count) => count + 1);

  /**
   * Starts a write whose reply (the server's facts) replaces what's shown. Take the turn when the
   * write starts: a later write or read then wins over this one. A reply that lost to a later read
   * reads again, as that read may have been answered before this write was.
   */
  const begin = (): Landing<T> => {
    turn.current += 1;

    const mine = turn.current;
    const at = key;

    return {
      land: (value) => {
        if (shownKey.current !== at) {
          return;
        }

        if (turn.current === mine) {
          setState({ status: "ready", value, key: at });
        } else {
          reload();
        }
      },
      fail: () => {
        if (shownKey.current === at) reload();
      },
    };
  };

  return { state: shown, reload, begin };
}
