import { type RefObject, useEffect, useState } from "react";

/** How far the list scrolls before the banner folds away, and where it comes back. */
const FOLD_AT = 12;

/** The banner's extra height over the plain header, in px (`--sidebar-banner-height` minus it). */
export const BANNER_EXTRA = 88;

/**
 * Whether the sidebar's banner has folded into the plain header: once the list scrolls past a
 * few pixels, and back at the top. A list too short to keep scrolling once the banner folds
 * never folds it (folding would grow the list's box, clamp the scroll to the top and unfold it).
 */
export function shouldFold(
  folded: boolean,
  scrollTop: number,
  scrollHeight: number,
  clientHeight: number,
): boolean {
  if (scrollTop <= 0) {
    return false;
  }

  if (folded) {
    return true;
  }

  return scrollTop > FOLD_AT && scrollHeight - clientHeight > BANNER_EXTRA + FOLD_AT;
}

/** {@link shouldFold} for `scroller`, while `enabled` (a banner is showing). */
export function useBannerFold(scroller: RefObject<HTMLElement | null>, enabled: boolean): boolean {
  const [folded, setFolded] = useState(false);

  useEffect(() => {
    const element = scroller.current;

    if (!enabled || element === null) {
      setFolded(false);

      return;
    }

    const update = () =>
      setFolded((current) =>
        shouldFold(current, element.scrollTop, element.scrollHeight, element.clientHeight),
      );

    update();
    element.addEventListener("scroll", update, { passive: true });

    return () => element.removeEventListener("scroll", update);
  }, [scroller, enabled]);

  return folded;
}
