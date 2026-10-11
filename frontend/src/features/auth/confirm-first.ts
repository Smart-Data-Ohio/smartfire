import type { MouseEvent } from "react";
import { actions } from "../../sync/runtime.ts";
import { toast } from "../../ui/toast-store.ts";
import { pageExit } from "./auth-navigation.ts";

/** Whether a check is under way: a second click waits for the first rather than going twice. */
let checking = false;

/**
 * Runs `go`, a full-page action the server confirms first (connecting Google or Slack, the audit
 * log export), once a fresh confirmation is in place: when there isn't one, the "Confirm it's
 * you" dialog asks for it here, rather than the server sending the page off to confirm. Cancelled,
 * nothing goes; confirmed with Google, the page leaves and the person clicks again once back.
 */
export async function confirmThen(go: () => void): Promise<void> {
  if (checking) {
    return;
  }

  checking = true;

  try {
    const outcome = await actions.confirmation
      .confirmFirst()
      .catch((failure: Error) => ({ kind: "unavailable", reason: failure.message }) as const);

    if (outcome.kind === "go") {
      go();
    } else if (outcome.kind === "unavailable" && outcome.reason !== null) {
      toast({
        title: "Couldn't check your confirmation",
        description: outcome.reason,
        tone: "danger",
      });
    }
  } finally {
    checking = false;
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

  void confirmThen(() => (saves ? save(href, download) : pageExit.assign(href)));
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
