import type { RouterHistory } from "@tanstack/react-router";

interface MessageLink {
  readonly messageId: number;
  readonly search: string;
  readonly hash: string;
}

declare module "@tanstack/react-router" {
  interface HistoryState {
    readonly smartfireMessageLink?: MessageLink;
  }
}

/** The original query and fragment of a bare message link, before router serialization. */
export function messageLinkSnapshot(href: string): MessageLink | null {
  const url = new URL(href, "http://smartfire.local");
  let pathname: string;

  try {
    pathname = decodeURIComponent(url.pathname);
  } catch {
    return null;
  }

  const match = /^\/app\/m\/([^/]+)\/?$/.exec(pathname);
  const messageId = Number(match?.[1]);

  return Number.isSafeInteger(messageId) && messageId > 0
    ? { messageId, search: url.search, hash: url.hash }
    : null;
}

/** Initial router canonicalization keeps history state but rewrites repeated query keys. */
export function captureInitialMessageLink(history: RouterHistory): void {
  const { href, state } = history.location;
  const snapshot = messageLinkSnapshot(href);

  if (snapshot !== null && state.smartfireMessageLink?.messageId !== snapshot.messageId) {
    history.replace(href, { ...state, smartfireMessageLink: snapshot });
  }
}
