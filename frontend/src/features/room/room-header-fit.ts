import { useEffectEvent, useLayoutEffect, useRef } from "react";

function px(style: CSSStyleDeclaration, property: string): number {
  return Number.parseFloat(style.getPropertyValue(property)) || 0;
}

/** The narrowest the room's name gets (or its full width, if that's less) before the call's faces,
    the tag or topic and the pane buttons give way to it. */
const NAME_FLOOR_REM = 6;

/** What the header shows beyond room.css's steps: the call button's `label` off, `search` off,
    `compact` (room.css's narrowest step: no faces, tag or topic, the pane buttons in ⋯), and the
    heading's `floor` in px. They give way in that order. */
interface Fit {
  readonly label: boolean;
  readonly search: boolean;
  readonly compact: boolean;
  readonly floor: number;
}

const FULL: Fit = { label: true, search: true, compact: false, floor: 0 };

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
 * minimum (`--room-name-min`, or the floor if wider), first with the call button's label and then,
 * measured again, without it. Without the search, the label stays off and the faces, tag or topic
 * and pane buttons fold once the name would be narrower than its floor.
 */
function measureFit(header: HTMLElement): Fit {
  const heading = header.querySelector<HTMLElement>(".page-header-heading");
  const actions = header.querySelector<HTMLElement>(".page-header-actions");
  const search = header.querySelector<HTMLElement>(".header-search");

  if (heading === null || actions === null) {
    return FULL;
  }

  apply(header, { label: true, search: false, compact: false, floor: 0 });

  const style = getComputedStyle(header);
  const floor = nameFloor(heading, header.querySelector<HTMLElement>(".room-title-name"));
  // What the heading has beside the actions at their full width: all that's left, in this state.
  const room = heading.getBoundingClientRect().width;
  const nameNeeds = Math.max(px(style, "--room-name-min"), floor);

  let searchNeeds = 0;

  if (search !== null) {
    const searchStyle = getComputedStyle(search);

    searchNeeds =
      px(style, "--header-search-min") +
      px(searchStyle, "margin-left") +
      px(searchStyle, "margin-right") +
      px(getComputedStyle(actions), "column-gap");
  }

  if (room - searchNeeds >= nameNeeds) {
    return { label: true, search: true, compact: false, floor };
  }

  apply(header, { label: false, search: false, compact: false, floor: 0 });

  const roomWithoutLabel = heading.getBoundingClientRect().width;

  if (roomWithoutLabel - searchNeeds >= nameNeeds) {
    return { label: false, search: true, compact: false, floor };
  }

  return { label: false, search: false, compact: roomWithoutLabel < floor, floor };
}

function apply(header: HTMLElement, fit: Fit) {
  if (fit.label) {
    delete header.dataset.label;
  } else {
    header.dataset.label = "off";
  }

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

/** Where focus goes when the ⋯ menu or the layout goes from under it, in order: the pane
    buttons the menu held first. */
const FOCUS_FALLBACKS = [
  ".pane-buttons button",
  ".room-title-button",
  ".page-header-actions button",
];

/** Whether `element` is laid out: `checkVisibility()` without it (Safari before 17.4). */
function shown(element: Element): boolean {
  return element.getClientRects().length > 0;
}

function focusFallback(header: HTMLElement) {
  const target = FOCUS_FALLBACKS.flatMap((selector) => [
    ...header.querySelectorAll<HTMLElement>(selector),
  ]).find(shown);

  target?.focus({ preventScroll: true });
}

/** What changes in the header without changing its layout: the search's field and dropdown, which
    widen on focus and change with every keystroke, and tooltips and menus, in the top layer. */
const TRANSIENT = ".header-search, [popover]";

function transient(node: Node): boolean {
  const element = node instanceof Element ? node : node.parentElement;

  return element?.closest(TRANSIENT) != null;
}

function changesLayout(record: MutationRecord): boolean {
  if (transient(record.target)) {
    return false;
  }

  return (
    record.type !== "childList" ||
    [...record.addedNodes, ...record.removedNodes].some(
      (node) => !(node instanceof Element && node.matches("[popover]")),
    )
  );
}

/**
 * The wide room header's measured layout, for its `ref`: it sets the header's `data-label="off"`,
 * `data-search="off"` and `data-compact` (see `measureFit`) whenever its width or content changes,
 * and, while the ⋯ menu is open (`moreOpen`), calls `onMoreHidden` once the ⋯ button goes, with
 * focus moved to the pane buttons it held. A switch between the phone and wide layouts that takes
 * the header's focus with it moves focus to the first of those that shows.
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

    if (moreOpen && more !== null && !shown(more)) {
      focusFallback(header);
      onMoreHidden();
    }
  });

  // Whether focus was in the header when it last moved: a switch between the phone and wide
  // layouts removes the focused control (or the menu it was in) without a focus event, leaving
  // focus on the body.
  const hadFocus = useRef(false);

  useLayoutEffect(() => {
    const header = ref.current;

    if (header === null) {
      return;
    }

    const focusIn = () => {
      hadFocus.current = true;
    };

    const focusOut = (event: FocusEvent) => {
      // A menu closing drops focus to the body (with no relatedTarget) when its trigger has gone.
      const menuClosed =
        event.relatedTarget === null &&
        event.target instanceof Element &&
        event.target.closest("[popover]") !== null;

      if (!menuClosed) {
        hadFocus.current =
          event.relatedTarget instanceof Node && header.contains(event.relatedTarget);
      }
    };

    header.addEventListener("focusin", focusIn);
    header.addEventListener("focusout", focusOut);

    return () => {
      header.removeEventListener("focusin", focusIn);
      header.removeEventListener("focusout", focusOut);
    };
  }, []);

  useLayoutEffect(() => {
    const header = ref.current;

    // The layout changed under the header's focus: give it the first control that shows.
    const refocus = () => {
      if (header !== null && hadFocus.current && !header.contains(document.activeElement)) {
        focusFallback(header);
      }
    };

    // Without ResizeObserver (unit tests) room.css's steps alone place the search.
    if (!wide || header === null || !("ResizeObserver" in window)) {
      if (header !== null) {
        apply(header, FULL);
      }

      refocus();

      return;
    }

    // The header's own size changes with the window, a pane and the text size; what's in it
    // (someone joining the call, a new topic) changes without that, and fonts load late. Only the
    // header is observed for size, since measuring changes its parts' sizes. Measuring hides the
    // search for an instant, so typing in it or a tooltip opening mustn't measure (or the field's
    // widening on focus would snap rather than ease).
    const resize = new ResizeObserver(() => measure());

    const content = new MutationObserver((records) => {
      if (records.some(changesLayout)) {
        measure();
      }
    });

    const fonts = () => measure();

    resize.observe(header);
    content.observe(header, { childList: true, subtree: true, characterData: true });
    document.fonts.addEventListener("loadingdone", fonts);
    measure();
    refocus();

    return () => {
      resize.disconnect();
      content.disconnect();
      document.fonts.removeEventListener("loadingdone", fonts);
      apply(header, FULL);
    };
  }, [wide]);

  return ref;
}
