import { useEffectEvent, useLayoutEffect, useRef, useState } from "react";

function px(style: CSSStyleDeclaration, property: string): number {
  return Number.parseFloat(style.getPropertyValue(property)) || 0;
}

/**
 * Whether the wide header's search keeps its usable width (`--header-search-min`) beside the
 * room name's minimum (`--room-name-min`) and the other actions. room.css's steps count a plain
 * room's actions; a call's faces, a stage's button or a voice limit widen them, and only a
 * measurement adds those up. The other actions never narrow, so hiding the search doesn't change
 * the answer.
 */
function searchFits(header: HTMLElement): boolean {
  const actions = header.querySelector<HTMLElement>(".page-header-actions");
  const search = header.querySelector<HTMLElement>(".header-search");

  if (actions === null || search === null) {
    return true;
  }

  const style = getComputedStyle(header);
  const actionsGap = px(getComputedStyle(actions), "column-gap");
  const searchWidth = search.checkVisibility() ? search.offsetWidth + actionsGap : 0;
  // The actions' content, which overflows their box while they're squeezed.
  const others = actions.scrollWidth - searchWidth;

  const room =
    header.clientWidth -
    px(style, "padding-left") -
    px(style, "padding-right") -
    px(style, "--room-name-min") -
    px(style, "column-gap") -
    others -
    actionsGap;

  return room >= px(style, "--header-search-min");
}

/** Where focus goes when the ⋯ menu closes under it, in order: the pane buttons it held first. */
const FOCUS_FALLBACKS = [
  ".pane-buttons button",
  ".room-title-button",
  ".page-header-actions button",
];

/**
 * The wide room header's measured layout, for its `ref`: `searchFits` (see `searchFits` above),
 * and, while the ⋯ menu is open (`moreOpen`), `onMoreHidden` once the header widens past the step
 * that shows the ⋯ button (room.css), with focus moved to the pane buttons it held.
 */
export function useRoomHeaderFit({
  wide,
  moreOpen,
  onMoreHidden,
}: {
  readonly wide: boolean;
  readonly moreOpen: boolean;
  readonly onMoreHidden: () => void;
}) {
  const ref = useRef<HTMLElement | null>(null);
  const [fits, setFits] = useState(true);

  const measure = useEffectEvent(() => {
    const header = ref.current;

    if (header === null) {
      return;
    }

    setFits(searchFits(header));

    const more = header.querySelector<HTMLElement>(".page-header-overflow-wrap");

    if (moreOpen && more !== null && !more.checkVisibility()) {
      const target = FOCUS_FALLBACKS.flatMap((selector) => [
        ...header.querySelectorAll<HTMLElement>(selector),
      ]).find((element) => element.checkVisibility());

      target?.focus({ preventScroll: true });
      onMoreHidden();
    }
  });

  useLayoutEffect(() => {
    const header = ref.current;

    // Without ResizeObserver (unit tests) room.css's steps alone place the search.
    if (!wide || header === null || !("ResizeObserver" in window)) {
      setFits(true);

      return;
    }

    const observer = new ResizeObserver(() => measure());

    observer.observe(header);

    for (const part of header.querySelectorAll(".page-header-actions, .header-search")) {
      observer.observe(part);
    }

    measure();

    return () => observer.disconnect();
  }, [wide]);

  return { ref, searchFits: fits };
}
