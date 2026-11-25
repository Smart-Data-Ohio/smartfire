import { inCoveredSpoiler } from "../messages/spoilers.ts";
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
 * Opens the user card from the mention chips in a body of message HTML: on hover, or on a click
 * or tap. The chips stay the server's spans; one listener on the body serves them all. Returns
 * the cleanup, which also closes a card hanging off a chip in this body.
 */
export function bindMentionCards(root: HTMLElement): () => void {
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
      openByClick({ userId: hit.userId, anchor: hit.element, placement: "bottom-start" });
    }
  };

  root.addEventListener("pointerover", onPointerOver);
  root.addEventListener("pointerout", onPointerOut);
  root.addEventListener("click", onClick);

  return () => {
    root.removeEventListener("pointerover", onPointerOver);
    root.removeEventListener("pointerout", onPointerOut);
    root.removeEventListener("click", onClick);
    releaseAnchor(root);
  };
}
