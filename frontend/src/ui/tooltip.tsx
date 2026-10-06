import {
  type FocusEvent,
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
import { showPopover } from "../lib/popover.ts";
import { readDurationMs } from "../motion/durations.ts";
import { usePresence } from "../motion/presence.ts";
import { useFloating } from "./floating.ts";
import { Kbd } from "./kbd.tsx";
import "./floating.css";
import "./tooltip.css";

interface TooltipProps {
  readonly content: ReactNode;
  /** A keyboard shortcut shown after the text, e.g. ["⌘", "K"]. */
  readonly shortcut?: readonly string[] | undefined;
  readonly placement?: Placement;
  /**
   * Whether the trigger is described by the tooltip text (`aria-describedby`). Turn it off when
   * the text repeats the trigger's own accessible name, as on an icon button.
   */
  readonly describe?: boolean;
  /** One focusable element; the tooltip anchors to it. */
  readonly children: ReactElement;
}

/**
 * Tooltips feel "warm" for a moment after one closes: moving along a toolbar shows the next one at
 * once instead of waiting again, as in Slack and Discord.
 */
let lastHiddenAt = Number.NEGATIVE_INFINITY;

let openCount = 0;

function isWarm(): boolean {
  return openCount > 0 || performance.now() - lastHiddenAt < 600;
}

function matchesFocusVisible(element: Element): boolean {
  try {
    return element.matches(":focus-visible");
  } catch {
    return true;
  }
}

/**
 * A tooltip for one trigger. It shows on hover after a short cold delay (instantly when warm),
 * and on keyboard focus; it hides on leave, blur, Esc and press. The wrapper is `display:
 * contents`, so the trigger keeps its own layout.
 */
export function Tooltip({
  content,
  shortcut,
  placement = "top",
  describe = true,
  children,
}: TooltipProps) {
  const id = useId();
  const hostRef = useRef<HTMLSpanElement | null>(null);
  const anchorRef = useRef<HTMLElement | null>(null);
  const timer = useRef(0);
  const [open, setOpen] = useState(false);
  const [warm, setWarm] = useState(false);
  const presence = usePresence<HTMLDivElement>(open);
  const descriptionId = `${id}-description`;

  useLayoutEffect(() => {
    const first = hostRef.current?.firstElementChild;

    anchorRef.current = first instanceof HTMLElement ? first : null;

    if (!describe || anchorRef.current === null) {
      return;
    }

    const anchor = anchorRef.current;

    anchor.setAttribute("aria-describedby", descriptionId);

    return () => anchor.removeAttribute("aria-describedby");
  });

  useFloating(id, anchorRef, presence.ref, placement, presence.mounted);

  useLayoutEffect(() => {
    const element = presence.ref.current;

    if (open && element !== null) {
      showPopover(element);
    }
  }, [open, presence.ref]);

  useEffect(() => {
    if (!open) {
      return;
    }

    openCount += 1;

    return () => {
      openCount -= 1;
      lastHiddenAt = performance.now();
    };
  }, [open]);

  useEffect(() => () => window.clearTimeout(timer.current), []);

  const show = () => {
    window.clearTimeout(timer.current);

    if (isWarm()) {
      setWarm(true);
      setOpen(true);

      return;
    }

    setWarm(false);
    timer.current = window.setTimeout(
      () => setOpen(true),
      readDurationMs("--duration-tooltip-cold"),
    );
  };

  const hide = () => {
    window.clearTimeout(timer.current);
    setOpen(false);
  };

  const onFocus = (event: FocusEvent<HTMLSpanElement>) => {
    if (event.target === anchorRef.current && matchesFocusVisible(event.target)) {
      show();
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLSpanElement>) => {
    if (event.key === "Escape" && open) {
      hide();
    }
  };

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: a display:contents wrapper that only listens to its trigger's events
    <span
      ref={hostRef}
      className="tooltip-host"
      onPointerEnter={(event) => {
        if (event.pointerType === "mouse") {
          show();
        }
      }}
      onPointerLeave={hide}
      onPointerDown={hide}
      onFocus={onFocus}
      onBlur={hide}
      onKeyDown={onKeyDown}
    >
      {children}
      {describe ? (
        <span id={descriptionId} hidden>
          {content}
        </span>
      ) : null}
      {presence.mounted ? (
        <TooltipBubble
          bubbleRef={presence.ref}
          state={presence.state}
          placement={placement}
          warm={warm}
          content={content}
          shortcut={shortcut}
        />
      ) : null}
    </span>
  );
}

function TooltipBubble({
  bubbleRef,
  state,
  placement,
  warm,
  content,
  shortcut,
}: {
  readonly bubbleRef: RefObject<HTMLDivElement | null>;
  readonly state: "open" | "closing";
  readonly placement: Placement;
  readonly warm: boolean;
  readonly content: ReactNode;
  readonly shortcut: readonly string[] | undefined;
}) {
  return (
    <div
      ref={bubbleRef}
      className="tooltip floating t-tt"
      popover="manual"
      role="tooltip"
      aria-hidden="true"
      data-state={state}
      data-placement={placement}
      data-warm={warm || undefined}
    >
      <span className="tooltip-text">{content}</span>
      {shortcut === undefined ? null : <Kbd keys={shortcut} />}
    </div>
  );
}
