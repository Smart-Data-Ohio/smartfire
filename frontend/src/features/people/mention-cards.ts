import { inCoveredSpoiler, SPOILER_REVEALED } from "../messages/spoilers.ts";
import { hoverEnter, hoverLeave, openByClick, releaseAnchor } from "./user-card-store.ts";

/** A person's mention chip as the server writes it into message HTML (crates/richtext). */
const MENTION = ".mention[data-user-id]";

interface MentionHit {
  readonly element: HTMLElement;
  readonly userId: number;
}

/** The mention chip `target` is in, unless a covered spoiler still hides it. */
function mentionAt(target: EventTarget | null): MentionHit | null {
  const element = target instanceof Element ? target.closest(MENTION) : null;

  if (!(element instanceof HTMLElement) || inCoveredSpoiler(element)) {
    return null;
  }

  const userId = Number(element.dataset.userId);

  return Number.isSafeInteger(userId) && userId > 0 ? { element, userId } : null;
}

/** Whether the pointer moved between two parts of the same chip (its name and its @). */
function within(hit: MentionHit, other: EventTarget | null): boolean {
  return other instanceof Node && hit.element.contains(other);
}

/**
 * Makes each chip a stop in the tab order that announces its card, or takes that away from one a
 * covered spoiler hides (the spoiler is the stop until it is revealed). The chips stay spans.
 */
function syncChips(root: HTMLElement): void {
  for (const chip of root.querySelectorAll<HTMLElement>(MENTION)) {
    if (inCoveredSpoiler(chip)) {
      unmarkChip(chip);
    } else {
      chip.tabIndex = 0;
      chip.setAttribute("role", "button");
      chip.setAttribute("aria-haspopup", "dialog");
    }
  }
}

function unmarkChip(chip: HTMLElement): void {
  chip.removeAttribute("role");
  chip.removeAttribute("aria-haspopup");

  if (chip.getAttribute("tabindex") === "0") {
    chip.removeAttribute("tabindex");
  }
}

/**
 * Opens the user card from the mention chips in a body of message HTML: on hover, or on a click,
 * a tap, Enter or Space. The chips stay the server's spans; one listener on the body serves them
 * all. Returns the cleanup, which takes the chips out of the tab order again and closes a card
 * hanging off a chip in this body.
 */
export function bindMentionCards(root: HTMLElement): () => void {
  syncChips(root);

  const open = (hit: MentionHit) =>
    openByClick({ userId: hit.userId, anchor: hit.element, placement: "bottom-start" });

  const onPointerOver = (event: PointerEvent) => {
    const hit = mentionAt(event.target);

    if (hit !== null && !within(hit, event.relatedTarget)) {
      hoverEnter(
        { userId: hit.userId, anchor: hit.element, placement: "bottom-start" },
        event.pointerType,
      );
    }
  };

  const onPointerOut = (event: PointerEvent) => {
    const hit = mentionAt(event.target);

    if (hit !== null && !within(hit, event.relatedTarget)) {
      hoverLeave();
    }
  };

  const onClick = (event: MouseEvent) => {
    // The click that reveals a covered spoiler (bound on the same body, first) is the spoiler's.
    if (event.defaultPrevented) {
      return;
    }

    const hit = mentionAt(event.target);

    if (hit !== null) {
      open(hit);
    }
  };

  // Only the chip itself takes the key, as a button would; Enter or Space on a covered spoiler
  // around one is the spoiler's.
  const onKeyDown = (event: KeyboardEvent) => {
    if ((event.key !== "Enter" && event.key !== " ") || event.defaultPrevented) {
      return;
    }

    const hit = mentionAt(event.target);

    if (hit !== null && hit.element === event.target) {
      event.preventDefault();
      open(hit);
    }
  };

  const onSpoilerRevealed = () => syncChips(root);

  root.addEventListener("pointerover", onPointerOver);
  root.addEventListener("pointerout", onPointerOut);
  root.addEventListener("click", onClick);
  root.addEventListener("keydown", onKeyDown);
  root.addEventListener(SPOILER_REVEALED, onSpoilerRevealed);

  return () => {
    root.removeEventListener("pointerover", onPointerOver);
    root.removeEventListener("pointerout", onPointerOut);
    root.removeEventListener("click", onClick);
    root.removeEventListener("keydown", onKeyDown);
    root.removeEventListener(SPOILER_REVEALED, onSpoilerRevealed);

    for (const chip of root.querySelectorAll<HTMLElement>(MENTION)) {
      unmarkChip(chip);
    }

    releaseAnchor(root);
  };
}
