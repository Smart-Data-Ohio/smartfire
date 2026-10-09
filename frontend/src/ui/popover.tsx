import {
  type KeyboardEvent,
  type ReactElement,
  type ReactNode,
  type RefObject,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type { Placement } from "../lib/anchor.ts";
import { showPopover, supportsPopover } from "../lib/popover.ts";
import { duringAppFocus } from "../lib/reader-focus.ts";
import { usePresence } from "../motion/presence.ts";
import { originFor, useFloating } from "./floating.ts";
import "./floating.css";
import "./popover.css";

export interface PopoverTriggerProps {
  readonly ref: RefObject<HTMLButtonElement | null>;
  readonly "aria-haspopup": "dialog";
  readonly "aria-expanded": boolean;
  readonly "aria-controls": string | undefined;
  readonly onClick: () => void;
}

interface PopoverProps {
  readonly trigger: (props: PopoverTriggerProps) => ReactElement;
  /** The popover's accessible name. */
  readonly label: string;
  readonly placement?: Placement;
  readonly children: ReactNode | ((close: () => void) => ReactNode);
}

/**
 * A non-modal floating panel (emoji picker, profile card, quick settings). It opens beside its
 * trigger in the top layer, moves focus inside, and closes on Esc (focus back to the trigger) or
 * an outside click (focus stays where the click put it).
 */
export function Popover({ trigger, label, placement = "bottom-start", children }: PopoverProps) {
  const id = useId();
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const [open, setOpen] = useState(false);
  const dismissedAt = useRef(Number.NEGATIVE_INFINITY);
  const presence = usePresence<HTMLDivElement>(open);

  useFloating(id, triggerRef, presence.ref, placement, presence.mounted);

  useLayoutEffect(() => {
    const surface = presence.ref.current;

    if (!open || surface === null) {
      return;
    }

    showPopover(surface);

    const first = surface.querySelector<HTMLElement>(
      "[data-autofocus], button, input, select, textarea, a[href], [tabindex]:not([tabindex='-1'])",
    );

    (first ?? surface).focus({ preventScroll: true });
  }, [open, presence.ref]);

  useEffect(() => {
    const surface = presence.ref.current;

    if (!open || surface === null) {
      return;
    }

    const dismiss = () => {
      dismissedAt.current = performance.now();
      setOpen(false);
    };

    const onToggle = (event: Event) => {
      if ("newState" in event && event.newState === "closed") {
        dismiss();
      }
    };

    const onPointerDown = (event: PointerEvent) => {
      const target = event.target;

      if (
        target instanceof Node &&
        !surface.contains(target) &&
        !(triggerRef.current?.contains(target) ?? false)
      ) {
        dismiss();
      }
    };

    if (supportsPopover(surface)) {
      surface.addEventListener("toggle", onToggle);

      return () => surface.removeEventListener("toggle", onToggle);
    }

    document.addEventListener("pointerdown", onPointerDown);

    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open, presence.ref]);

  const close = () => {
    setOpen(false);
    duringAppFocus(() => {
      triggerRef.current?.focus({ preventScroll: true });
    });
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close();
    }
  };

  const triggerProps: PopoverTriggerProps = {
    ref: triggerRef,
    "aria-haspopup": "dialog",
    "aria-expanded": open,
    "aria-controls": presence.mounted ? id : undefined,
    onClick: () => {
      if (open) {
        setOpen(false);
      } else if (performance.now() - dismissedAt.current > 300) {
        setOpen(true);
      }
    },
  };

  return (
    <>
      {trigger(triggerProps)}
      {presence.mounted ? (
        <div
          ref={presence.ref}
          id={id}
          role="dialog"
          aria-label={label}
          tabIndex={-1}
          popover="auto"
          className="popover floating t-dropdown"
          data-state={presence.state}
          data-placement={placement}
          data-origin={originFor(placement)}
          onKeyDown={onKeyDown}
        >
          {children instanceof Function ? children(close) : children}
        </div>
      ) : null}
    </>
  );
}
