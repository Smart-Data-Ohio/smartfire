import { type MouseEvent, useEffect, useMemo, useRef } from "react";
import { actions } from "../../sync/runtime.ts";
import { toast } from "../../ui/toast-store.ts";
import { pageExit } from "./auth-navigation.ts";

/** The action under way, if any: a second click waits for it rather than going twice. */
let pending: { readonly release: () => void } | null = null;

/** Whether the page began to unload since the last action left (`pagehide`). */
let unloading = false;

/** How long a download keeps the control quiet: the page stays, so a double click would save twice. */
const SAVE_SETTLE_MS = 1000;

/**
 * How long a page on its way out may take to go. Still showing after this, it isn't going (the
 * person stayed on a "Leave site?" prompt, or the answer was empty): the controls go again.
 */
const LEAVE_SETTLE_MS = 3000;

if (typeof window !== "undefined") {
  window.addEventListener("pagehide", () => {
    unloading = true;
  });

  // Back from the back-forward cache: the page didn't go after all, as far as it knows.
  window.addEventListener("pageshow", () => {
    unloading = false;
    pending?.release();
  });
}

/**
 * Runs `go`, a full-page action the server confirms first (connecting Google or Slack, the audit
 * log export), once a fresh confirmation is in place: when there isn't one, the "Confirm it's
 * you" dialog asks for it here, rather than the server sending the page off to confirm. Cancelled,
 * nothing goes; confirmed with Google, the page leaves and the person clicks again once back.
 *
 * Once `go` runs, nothing goes again while the page is leaving; still here a moment later (a
 * download, `stays`, a little sooner), the controls go again. `signal` (the control went away: its
 * screen was left, or its picker closed) stops the check, closes the dialog it opened, and lets the
 * controls go again.
 */
export async function confirmThen(
  go: () => void,
  {
    stays = false,
    signal,
  }: { readonly stays?: boolean; readonly signal?: AbortSignal | undefined } = {},
): Promise<void> {
  if (pending !== null || signal?.aborted === true) {
    return;
  }

  let timer: ReturnType<typeof setTimeout> | undefined;

  const mine = {
    release: () => {
      if (pending === mine) {
        pending = null;
      }

      clearTimeout(timer);
      signal?.removeEventListener("abort", mine.release);
      document.removeEventListener("visibilitychange", settle);
    },
  };

  /** Still showing once the page should have gone: it didn't, and the controls go again. */
  const settle = () => {
    if (unloading) {
      return;
    }

    if (document.visibilityState === "visible") {
      mine.release();
    } else {
      document.addEventListener("visibilitychange", settle, { once: true });
    }
  };

  pending = mine;
  signal?.addEventListener("abort", mine.release, { once: true });

  // What follows: the controls go again now, after a download's moment, or once the page should
  // have gone.
  let after: "now" | "moment" | "leaving" = "now";

  try {
    const outcome = await actions.confirmation
      .confirmFirst(signal)
      .catch((failure: Error) => ({ kind: "unavailable", reason: failure.message }) as const);

    // Stopped meanwhile: the control went away, and nothing goes.
    if (pending !== mine) {
      return;
    }

    if (outcome.kind === "go") {
      unloading = false;
      go();
      after = stays ? "moment" : "leaving";
    } else if (outcome.kind === "signedOut") {
      // The session ended (in another tab, say): on to sign in.
      unloading = false;
      pageExit.assign(outcome.location);
      after = "leaving";
    } else if (outcome.kind === "unavailable") {
      toast({
        title: "Couldn't check your confirmation",
        description: outcome.reason,
        tone: "danger",
      });
    }
  } finally {
    if (after === "now") {
      mine.release();
    } else {
      timer = setTimeout(
        after === "moment" ? mine.release : settle,
        after === "moment" ? SAVE_SETTLE_MS : LEAVE_SETTLE_MS,
      );
    }
  }
}

/**
 * The pre-flight for a component's controls (`confirmThen`, `confirmLink`): what they start stops
 * when the component goes, or when `active` turns false (a picker closing).
 */
export function useConfirmFirst(active = true) {
  const current = useRef<AbortController | null>(null);

  useEffect(() => {
    if (!active) {
      return;
    }

    const controller = new AbortController();

    current.current = controller;

    return () => {
      controller.abort();

      if (current.current === controller) {
        current.current = null;
      }
    };
  }, [active]);

  return useMemo(() => {
    // Not showing (closed, or not yet mounted), nothing starts.
    const signal = () => current.current?.signal ?? AbortSignal.abort();

    return {
      confirmThen: (go: () => void) => confirmThen(go, { signal: signal() }),
      confirmLink: (event: MouseEvent<HTMLAnchorElement>) => confirmLink(event, signal()),
    };
  }, []);
}

/**
 * A link's click, for a link to a full-page action the server confirms first: a plain click
 * confirms (`confirmThen`) and then follows it. A click meant for a new tab or window, or a
 * download menu, is left to the browser.
 */
export function confirmLink(event: MouseEvent<HTMLAnchorElement>, signal?: AbortSignal): void {
  if (
    event.defaultPrevented ||
    event.button !== 0 ||
    event.metaKey ||
    event.ctrlKey ||
    event.shiftKey ||
    event.altKey
  ) {
    return;
  }

  event.preventDefault();

  const { href, download } = event.currentTarget;
  const saves = event.currentTarget.hasAttribute("download");

  void confirmThen(() => (saves ? save(href, download) : pageExit.assign(href)), {
    stays: saves,
    signal,
  });
}

/** Saves `href` as the link would have (a download, named `name` when it sets one). */
function save(href: string, name: string): void {
  const link = document.createElement("a");

  link.href = href;
  link.download = name;
  link.hidden = true;
  document.body.append(link);
  link.click();
  link.remove();
}
