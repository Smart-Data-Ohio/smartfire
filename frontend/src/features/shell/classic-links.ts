import { useRouter } from "@tanstack/react-router";
import { useEffect } from "react";
import { spaUrlFor } from "../../lib/screens.ts";

/** A plain left click: no modifier keys, so it isn't asking for a new tab or window. */
function isPlainClick(event: MouseEvent): boolean {
  return (
    event.button === 0 &&
    !event.defaultPrevented &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.shiftKey &&
    !event.altKey
  );
}

/**
 * The SPA path a click on `anchor` should open in place: a same-origin link to a classic page the
 * SPA has ported (a permalink pasted into a message, `/rooms/12/@34`). `null` leaves the click to
 * the browser: other origins, downloads, links to their own window, and classic pages the SPA
 * hasn't ported (they load in full, and the server serves them as ever).
 */
export function inPlaceTarget(
  anchor: HTMLAnchorElement,
  origin = window.location.origin,
): string | null {
  const target = anchor.getAttribute("target");

  if ((target !== null && target !== "" && target !== "_self") || anchor.hasAttribute("download")) {
    return null;
  }

  const url = new URL(anchor.href, origin);

  if (url.origin !== origin || new URLSearchParams(url.search).has("classic")) {
    return null;
  }

  const spa = spaUrlFor(url.pathname, url.search);

  return spa === null ? null : `${spa}${url.hash}`;
}

/**
 * Opens links to ported classic pages inside the SPA instead of loading the page (which would
 * only redirect back here): one document-level listener, so message bodies' server-rendered HTML
 * is covered too.
 */
export function useClassicLinks(): void {
  const router = useRouter();

  useEffect(() => {
    const onClick = (event: MouseEvent) => {
      if (!isPlainClick(event) || !(event.target instanceof Element)) {
        return;
      }

      const anchor = event.target.closest("a[href]");

      if (!(anchor instanceof HTMLAnchorElement)) {
        return;
      }

      const spa = inPlaceTarget(anchor);

      if (spa === null) {
        return;
      }

      event.preventDefault();
      // `href` is the public path: the router strips its `/app/` basepath itself.
      void router.navigate({ href: spa });
    };

    document.addEventListener("click", onClick);

    return () => document.removeEventListener("click", onClick);
  }, [router]);
}
