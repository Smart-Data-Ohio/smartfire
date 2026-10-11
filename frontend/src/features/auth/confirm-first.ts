import type { MouseEvent } from "react";
import { actions } from "../../sync/runtime.ts";
import { toast } from "../../ui/toast-store.ts";
import { pageExit } from "./auth-navigation.ts";

/** Whether an action is under way: a second click waits for the first rather than going twice. */
let checking = false;

/** Ends a download's quiet spell early (`release`). */
let settling: ReturnType<typeof setTimeout> | undefined;

/** How long a download keeps the control quiet: the page stays, so a double click would save twice. */
const SAVE_SETTLE_MS = 1000;

/** Lets the controls go again. */
function release(): void {
  checking = false;
  clearTimeout(settling);
  window.removeEventListener("pageshow", release);
}

/**
 * Runs `go`, a full-page action the server confirms first (connecting Google or Slack, the audit
 * log export), once a fresh confirmation is in place: when there isn't one, the "Confirm it's
 * you" dialog asks for it here, rather than the server sending the page off to confirm. Cancelled,
 * nothing goes; confirmed with Google, the page leaves and the person clicks again once back.
 *
 * Once `go` runs, nothing goes again until the page has left (back from the back-forward cache, it
 * can); a download (`stays`) leaves the page where it is, and lets go again after a moment.
 */
export async function confirmThen(
  go: () => void,
  { stays = false }: { readonly stays?: boolean } = {},
): Promise<void> {
  if (checking) {
    return;
  }

  checking = true;

  // What follows: the controls go again now, after a download's moment, or once the page left.
  let after: "now" | "moment" | "left" = "now";

  try {
    const outcome = await actions.confirmation
      .confirmFirst()
      .catch((failure: Error) => ({ kind: "unavailable", reason: failure.message }) as const);

    if (outcome.kind === "go") {
      go();
      after = stays ? "moment" : "left";
    } else if (outcome.kind === "signedOut") {
      // The session ended (in another tab, say): on to sign in.
      pageExit.assign(outcome.location);
      after = "left";
    } else if (outcome.kind === "unavailable") {
      toast({
        title: "Couldn't check your confirmation",
        description: outcome.reason,
        tone: "danger",
      });
    }
  } finally {
    if (after === "now") {
      release();
    } else {
      // Back from the back-forward cache, the page can go again.
      window.addEventListener("pageshow", release);

      if (after === "moment") {
        settling = setTimeout(release, SAVE_SETTLE_MS);
      }
    }
  }
}

/**
 * A link's click, for a link to a full-page action the server confirms first: a plain click
 * confirms (`confirmThen`) and then follows it. A click meant for a new tab or window, or a
 * download menu, is left to the browser.
 */
export function confirmLink(event: MouseEvent<HTMLAnchorElement>): void {
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

  void confirmThen(() => (saves ? save(href, download) : pageExit.assign(href)), { stays: saves });
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
