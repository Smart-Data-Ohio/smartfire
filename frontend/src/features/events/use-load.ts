import { useEffect, useRef, useState } from "react";

export type Loaded<T> =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string; readonly missing: boolean }
  | { readonly status: "ready"; readonly value: T };

type Keyed<T> = Loaded<T> & { readonly key: string };

/** Where a write's reply goes: shown if it's still the newest news for the screen it came from. */
export interface Landing<T> {
  readonly land: (value: T) => void;
  /**
   * The write failed, or its reply isn't for this screen after all: read the screen again (if
   * the viewer is still on it), since what it shows may have changed meanwhile.
   */
  readonly reread: () => void;
}

/** A landing for a screen the viewer has already left: nothing it brings back is shown. */
function nowhere<T>(): Landing<T> {
  return { land: () => {}, reread: () => {} };
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
  // The turn of the read on its way (`null` when none is), and whether news came in meanwhile.
  const reading = useRef<number | null>(null);
  const stale = useRef(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `key` names what `read` reads; a retry (`tries`) reads it again
  useEffect(() => {
    turn.current += 1;

    if (shownKey.current !== key) {
      stale.current = false;
    }

    shownKey.current = key;

    const mine = turn.current;
    const current = () => turn.current === mine && shownKey.current === key;

    reading.current = mine;
    setState((held) =>
      held.key === key && held.status === "ready" ? held : { status: "loading", key },
    );

    // Whatever became of this read, news that came in while it was on its way gets one more.
    const settled = () => {
      if (reading.current !== mine) {
        return;
      }

      reading.current = null;

      if (stale.current && shownKey.current === key) {
        stale.current = false;
        setTries((count) => count + 1);
      }
    };

    read().then(
      (value) => {
        if (current()) setState({ status: "ready", value, key });

        settled();
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

        settled();
      },
    );
  }, [key, tries]);

  // Unmounting ends every turn, so nothing lands afterwards.
  useEffect(
    () => () => {
      turn.current += 1;
      reading.current = null;
    },
    [],
  );

  const shown: Loaded<T> = state.key === key ? state : { status: "loading" };
  const reload = () => setTries((count) => count + 1);

  /**
   * News that what's shown changed elsewhere: read again, but while a read is on its way, once
   * more after it lands rather than another at once, so a burst of news costs two reads at most.
   */
  const refresh = () => {
    if (reading.current === null) {
      reload();
    } else {
      stale.current = true;
    }
  };

  /**
   * Starts a write whose reply (the server's facts) replaces what's shown. Call it when the write
   * starts, so a later write or read wins over this one. A reply that lost to a later read reads
   * again, as that read may have been answered before this write was. Called from a screen the
   * viewer has left (a stale callback), it takes no turn, so the shown screen's reads still land.
   */
  const begin = (): Landing<T> => {
    const at = key;

    if (shownKey.current !== at) {
      return nowhere();
    }

    turn.current += 1;

    const mine = turn.current;

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
      reread: () => {
        if (shownKey.current === at) reload();
      },
    };
  };

  return { state: shown, reload, refresh, begin };
}
