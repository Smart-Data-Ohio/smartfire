import { useEffectEvent, useLayoutEffect, useRef } from "react";

function px(style: CSSStyleDeclaration, property: string): number {
  return Number.parseFloat(style.getPropertyValue(property)) || 0;
}

/** The narrowest the room's name gets (or its full width, if that's less) before the call's faces,
    the tag or topic and the pane buttons give way to it. */
const NAME_FLOOR_REM = 6;

/** What the header shows beyond room.css's steps: `search` off, `compact` (room.css's narrowest
    step: no faces, tag or topic, the pane buttons in ⋯), and the heading's `floor` in px. */
interface Fit {
  readonly search: boolean;
  readonly compact: boolean;
  readonly floor: number;
}

const FULL: Fit = { search: true, compact: false, floor: 0 };

/**
 * The heading's width that leaves the name `NAME_FLOOR_REM` (or its full width, if less): the
 * name plus what the heading holds besides it, a channel's glyph, chevron and padding (its
 * button, which may take the heading's whole width) or a DM's avatar before it. scrollWidth
 * rounds, so a pixel more keeps the full name from an ellipsis.
 */
function nameFloor(heading: HTMLElement, name: HTMLElement | null): number {
  if (name === null) {
    return 0;
  }

  const rem = px(getComputedStyle(document.documentElement), "font-size");
  const part = name.closest<HTMLElement>(".room-title > *") ?? name;
  const nameBox = name.getBoundingClientRect();

  const around =
    part === name
      ? nameBox.left - heading.getBoundingClientRect().left
      : part.getBoundingClientRect().width - nameBox.width;

  return around + Math.min(NAME_FLOOR_REM * rem, name.scrollWidth + 1);
}

/**
 * The wide header's fit, measured with the search hidden and nothing folded (`data-search="off"`,
 * where nothing in the actions narrows), so the answer is the same whatever the header showed
 * before and can't flip from one frame to the next. room.css's steps count a plain room's
 * actions; a call's faces, a stage's button or a voice limit widen them, and only a measurement
 * adds those up.
 *
 * The search shows while it keeps its usable width (`--header-search-min`) beside the name's
 * minimum (`--room-name-min`, or the floor if wider). Without it, the faces, tag or topic and pane
 * buttons fold once the name would be narrower than its floor.
 */
function measureFit(header: HTMLElement): Fit {
  const heading = header.querySelector<HTMLElement>(".page-header-heading");
  const actions = header.querySelector<HTMLElement>(".page-header-actions");
  const search = header.querySelector<HTMLElement>(".header-search");

  if (heading === null || actions === null) {
    return FULL;
  }

  apply(header, { search: false, compact: false, floor: 0 });

  const style = getComputedStyle(header);
  const floor = nameFloor(heading, header.querySelector<HTMLElement>(".room-title-name"));
  // What the heading has beside the actions at their full width: all that's left, in this state.
  const room = heading.getBoundingClientRect().width;

  let searchNeeds = 0;

  if (search !== null) {
    const searchStyle = getComputedStyle(search);

    searchNeeds =
      px(style, "--header-search-min") +
      px(searchStyle, "margin-left") +
      px(searchStyle, "margin-right") +
      px(getComputedStyle(actions), "column-gap");
  }

  if (room - searchNeeds >= Math.max(px(style, "--room-name-min"), floor)) {
    return { search: true, compact: false, floor };
  }

  return { search: false, compact: room < floor, floor };
}

function apply(header: HTMLElement, fit: Fit) {
  if (fit.search) {
    delete header.dataset.search;
  } else {
    header.dataset.search = "off";
  }

  if (fit.compact) {
    header.dataset.compact = "";
  } else {
    delete header.dataset.compact;
  }

  // While the search shows, the heading keeps at least this (room.css).
  if (fit.floor > 0) {
    header.style.setProperty("--room-name-floor", `${fit.floor}px`);
  } else {
    header.style.removeProperty("--room-name-floor");
  }
}

/** Where focus goes when the ⋯ menu closes under it, in order: the pane buttons it held first. */
const FOCUS_FALLBACKS = [
  ".pane-buttons button",
  ".room-title-button",
  ".page-header-actions button",
];

/**
 * The wide room header's measured layout, for its `ref`: it sets the header's `data-search="off"`
 * and `data-compact` (see `measureFit`) whenever its width or content changes, and, while the ⋯
 * menu is open (`moreOpen`), calls `onMoreHidden` once the ⋯ button goes, with focus moved to the
 * pane buttons it held.
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

  const measure = useEffectEvent(() => {
    const header = ref.current;

    if (header === null) {
      return;
    }

    apply(header, measureFit(header));

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
      if (header !== null) {
        apply(header, FULL);
      }

      return;
    }

    // The header's own size changes with the window, a pane and the text size; what's in it
    // (someone joining the call, a new topic) changes without that, and fonts load late. Only the
    // header is observed for size, since measuring changes its parts' sizes.
    const resize = new ResizeObserver(() => measure());
    const content = new MutationObserver(() => measure());
    const fonts = () => measure();

    resize.observe(header);
    content.observe(header, { childList: true, subtree: true, characterData: true });
    document.fonts.addEventListener("loadingdone", fonts);
    measure();

    return () => {
      resize.disconnect();
      content.disconnect();
      document.fonts.removeEventListener("loadingdone", fonts);
      apply(header, FULL);
    };
  }, [wide]);

  return ref;
}
