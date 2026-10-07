/**
 * A run page's reads: polling while the run is active, and keeping a stale answer from
 * overwriting a newer one.
 */
import { useEffect, useRef, useState } from "react";
import type { SlackRun } from "../../gen/SlackRun.ts";
import { ActionError } from "../../sync/run.ts";
import { POLL_MS } from "./slack-format.ts";

/** Hands out tickets for reads and writes; only the newest ticket's answer counts. */
export interface Tickets {
  /** Takes the next ticket, making every earlier one stale; answers whether it's still newest. */
  readonly take: () => () => boolean;
}

/** A fresh ticket counter (one per page). */
export function newestOnly(): Tickets {
  let newest = 0;

  return {
    take: () => {
      newest += 1;

      const mine = newest;

      return () => mine === newest;
    },
  };
}

/** Poll failures that end the polling: the run or the session is gone. */
const FINAL = new Set(["Unauthorized", "Forbidden", "NotFound", "TwoFactorRequired"]);

/** The longest wait between reads after failures. */
const MAX_BACKOFF_MS = 60_000;

/** Why the run on the page may be out of date: reads are failing, or they stopped. */
export type PollTrouble = "retrying" | "stopped" | null;

/**
 * Reads the run again every few seconds while it is active, as the classic page's frame poll
 * does; `onSettled` hears when it stops being active. `read` answers `null` when a newer read or
 * write got there first. Polling pauses while the tab is hidden and reads at once when it is
 * shown again. A failed read waits longer each time (the last known status staying on the page);
 * a run or session that is gone stops it.
 */
export function usePoll(
  run: SlackRun | null,
  read: (id: number) => Promise<SlackRun | null>,
  onSettled: () => void,
): PollTrouble {
  const [failures, setFailures] = useState(0);
  const [stopped, setStopped] = useState(false);
  const [hidden, setHidden] = useState(() => document.hidden);
  // Failed reads in a row; kept across a hidden spell so the backoff carries on after it.
  const failed = useRef(0);
  // Set when the tab is shown again, so the next read goes at once.
  const woke = useRef(false);
  const id = run?.id ?? null;
  const active = run?.active ?? false;

  useEffect(() => {
    const changed = () => {
      woke.current = !document.hidden;
      setHidden(document.hidden);
    };

    document.addEventListener("visibilitychange", changed);

    return () => document.removeEventListener("visibilitychange", changed);
  }, []);

  useEffect(() => {
    if (id === null || !active || stopped || hidden) {
      return;
    }

    let live = true;
    let timer = 0;
    const backoff = () => Math.min(POLL_MS * 2 ** failed.current, MAX_BACKOFF_MS);

    const poll = () => {
      read(id).then(
        (fresh) => {
          if (!live) return;

          failed.current = 0;
          setFailures(0);

          if (fresh === null || fresh.active) timer = window.setTimeout(poll, POLL_MS);
          else onSettled();
        },
        (error: Error) => {
          if (!live) return;

          if (error instanceof ActionError && FINAL.has(error.tag)) {
            setStopped(true);

            return;
          }

          failed.current += 1;
          setFailures(failed.current);
          timer = window.setTimeout(poll, backoff());
        },
      );
    };

    timer = window.setTimeout(poll, woke.current ? 0 : backoff());
    woke.current = false;

    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [id, active, stopped, hidden, read, onSettled]);

  if (stopped) return "stopped";

  return failures > 0 ? "retrying" : null;
}
