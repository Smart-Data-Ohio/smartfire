import {
  type KeyboardEvent,
  type RefObject,
  type SyntheticEvent,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { duringAppFocus } from "../../lib/reader-focus.ts";
import { usePresence } from "../../motion/presence.ts";
import { useReducedMotion } from "../../motion/reduced-motion.ts";
import { useStore } from "../../store/store.ts";
import { completeTour } from "../../sync/settings.ts";
import { Button } from "../../ui/button.tsx";
import { usePhoneLayout } from "../panes/use-right-pane.ts";
import { type Box, cardPosition, dockFor, sameBox } from "./tour-placement.ts";
import { createTour, useTourState } from "./tour-state.ts";
import { TOUR_STEPS, type TourStep } from "./tour-steps.ts";
import "./tour.css";

/** The app's one tour. Skipping or finishing it stamps completion through classic's endpoint. */
export const tour = createTour({ steps: TOUR_STEPS.length, stamp: completeTour });

/** The help menu's "Restart tour": from the first step, whether or not it was completed. */
export function restartTour(): void {
  tour.start();
}

/** How far the highlight ring stands off its anchor, as classic's outline offset. */
const RING_OFFSET = 4;

/** The step's first anchor that is laid out (not in a hidden column), or `null`. */
function findAnchor(step: TourStep): HTMLElement | null {
  for (const selector of step.anchors) {
    for (const element of document.querySelectorAll<HTMLElement>(selector)) {
      const rect = element.getBoundingClientRect();

      if (rect.width > 0 && rect.height > 0) {
        return element;
      }
    }
  }

  return null;
}

function boxOf(element: HTMLElement | null): Box | null {
  if (element === null) {
    return null;
  }

  const { top, left, width, height } = element.getBoundingClientRect();

  return { top, left, width, height };
}

function focusables(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>("button:not([disabled])")];
}

/**
 * Follows the step's anchor every frame while the tour shows, so the ring and card keep up with
 * a room that loads, a list that scrolls or a window that resizes. It only re-renders on a change.
 */
function useAnchorBox(step: TourStep): Box | null {
  const [box, setBox] = useState<Box | null>(null);

  useLayoutEffect(() => {
    let frame = 0;

    const measure = () => {
      const next = boxOf(findAnchor(step));

      setBox((current) => (sameBox(current, next) ? current : next));
      frame = requestAnimationFrame(measure);
    };

    measure();

    return () => cancelAnimationFrame(frame);
  }, [step]);

  return box;
}

/**
 * The first-run product tour (classic's layouts/_tour): five steps, each a card anchored to what
 * it describes, with the rest of the page dimmed around a ring on it. A step whose anchor isn't
 * on screen shows the same card centred. On a phone the card is a sheet docked to the edge away
 * from its anchor. Escape skips, ←/→ go back and forth, Tab stays in the card, and focus goes back
 * to where it was when the tour ends. Mounted once by the shell; it starts by itself for someone
 * who hasn't completed it.
 */
export function ProductTour() {
  const tourCompleted = useStore((state) => state.me?.preferences.tourCompleted ?? null);
  const open = useTourState(tour, (state) => state.open);
  const index = useTourState(tour, (state) => state.index);
  const presence = usePresence<HTMLDialogElement>(open);

  useEffect(() => {
    tour.autoStart(tourCompleted);
  }, [tourCompleted]);

  if (!presence.mounted) {
    return null;
  }

  const step = TOUR_STEPS[index] ?? TOUR_STEPS[0];

  return step === undefined ? null : (
    <TourCallout
      dialogRef={presence.ref}
      state={presence.state}
      step={step}
      index={index}
      count={TOUR_STEPS.length}
    />
  );
}

interface TourCalloutProps {
  readonly dialogRef: RefObject<HTMLDialogElement | null>;
  readonly state: "open" | "closing";
  readonly step: TourStep;
  readonly index: number;
  readonly count: number;
}

function TourCallout({ dialogRef, state, step, index, count }: TourCalloutProps) {
  const id = useId();
  const cardRef = useRef<HTMLElement | null>(null);
  const nextRef = useRef<HTMLButtonElement | null>(null);
  const box = useAnchorBox(step);
  const phone = usePhoneLayout();
  const reduced = useReducedMotion();
  const last = index === count - 1;

  // The top layer, with the page behind it inert; focus goes back to where it was at the end.
  useLayoutEffect(() => {
    const dialog = dialogRef.current;

    if (dialog === null) {
      return;
    }

    const opener = document.activeElement;

    if ("showModal" in dialog && !dialog.open) {
      dialog.showModal();
    } else {
      dialog.setAttribute("open", "");
    }

    return () => {
      if ("close" in dialog && dialog.open) {
        dialog.close();
      }

      duringAppFocus(() => {
        if (opener instanceof HTMLElement && opener !== document.body && opener.isConnected) {
          opener.focus({ preventScroll: true });
        }
      });
    };
  }, [dialogRef]);

  // Each step brings its anchor into view and puts focus on Next, as classic's does.
  useEffect(() => {
    findAnchor(step)?.scrollIntoView({
      behavior: reduced ? "auto" : "smooth",
      block: "nearest",
      inline: "nearest",
    });
    nextRef.current?.focus({ preventScroll: true });
  }, [step, reduced]);

  // Beside the anchor on wider screens, again whenever the card's size changes (a step's copy);
  // phones and unanchored steps are placed by tour.css.
  useLayoutEffect(() => {
    const card = cardRef.current;

    if (card === null) {
      return;
    }

    if (box === null || phone) {
      card.style.removeProperty("left");
      card.style.removeProperty("top");

      return;
    }

    const place = () => {
      const at = cardPosition(
        box,
        { width: card.offsetWidth, height: card.offsetHeight },
        { width: window.innerWidth, height: window.innerHeight },
      );

      card.style.left = `${at.left}px`;
      card.style.top = `${at.top}px`;
    };

    place();

    if (typeof ResizeObserver === "undefined") {
      return;
    }

    const observer = new ResizeObserver(place);

    observer.observe(card);

    return () => observer.disconnect();
  }, [box, phone]);

  const onKeyDown = (event: KeyboardEvent<HTMLDialogElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      tour.skip();
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      tour.next();
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      tour.back();
    } else if (event.key === "Tab" && cardRef.current !== null) {
      const items = focusables(cardRef.current);
      const first = items[0];
      const lastItem = items.at(-1);
      const active = document.activeElement;
      const inside = active instanceof Node && cardRef.current.contains(active);

      if (first === undefined || lastItem === undefined) {
        event.preventDefault();
      } else if (event.shiftKey && (active === first || !inside)) {
        event.preventDefault();
        lastItem.focus();
      } else if (!event.shiftKey && (active === lastItem || !inside)) {
        event.preventDefault();
        first.focus();
      }
    }
  };

  const onCancel = (event: SyntheticEvent<HTMLDialogElement>) => {
    event.preventDefault();
    tour.skip();
  };

  const ring =
    box === null
      ? undefined
      : {
          top: box.top - RING_OFFSET,
          left: box.left - RING_OFFSET,
          width: box.width + RING_OFFSET * 2,
          height: box.height + RING_OFFSET * 2,
        };

  return (
    <dialog
      ref={dialogRef}
      className="tour"
      aria-modal="true"
      aria-labelledby={`${id}-title`}
      aria-describedby={`${id}-body`}
      data-state={state}
      data-anchored={box === null ? undefined : ""}
      data-dock={box === null || !phone ? undefined : dockFor(box, window.innerHeight)}
      data-step={step.id}
      onKeyDown={onKeyDown}
      onCancel={onCancel}
    >
      <div className="tour-spotlight" style={ring} aria-hidden="true" />
      <section ref={cardRef} className="tour-card">
        <p className="tour-progress">
          Step {index + 1} of {count}
        </p>
        <h2 id={`${id}-title`} className="tour-title">
          {step.title}
        </h2>
        <p id={`${id}-body`} className="tour-body">
          {step.body}
        </p>
        <div className="tour-actions">
          {index > 0 ? (
            <Button variant="ghost" onClick={tour.back}>
              Back
            </Button>
          ) : null}
          <span className="tour-spacer" aria-hidden="true" />
          <Button variant="ghost" onClick={tour.skip}>
            Skip tour
          </Button>
          <Button ref={nextRef} variant="primary" onClick={tour.next}>
            {last ? "Finish" : "Next"}
          </Button>
        </div>
      </section>
    </dialog>
  );
}
