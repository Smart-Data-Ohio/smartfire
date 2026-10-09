import {
  type KeyboardEvent,
  type MouseEvent,
  type ReactNode,
  useEffect,
  useLayoutEffect,
  useRef,
} from "react";
import { readDurationMs } from "../../motion/durations.ts";
import { Menu, type MenuTriggerProps } from "../../ui/menu.tsx";

/** A context menu request: where it hangs (viewport coordinates) and how it was opened. */
export interface PointMenuRequest {
  /** Bumps per request, so a closing menu can't clear the one that replaced it. */
  readonly id: number;
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  /** Opened from the keyboard: the menu lands on its first item. */
  readonly keyboard: boolean;
}

/** A request at a pointer position (a right click). */
export function requestAtPoint(id: number, x: number, y: number): PointMenuRequest {
  return { id, x, y, width: 1, height: 1, keyboard: false };
}

/** A request hanging from an element (the menu key, Shift+F10): under its top-right corner. */
export function requestAtElement(id: number, element: Element): PointMenuRequest {
  const box = element.getBoundingClientRect();

  return {
    id,
    x: Math.max(0, box.right - 56),
    y: box.top + 8,
    width: 28,
    height: 28,
    keyboard: true,
  };
}

/** A long press's resting point: it opens there, the finger already lifted. */
export interface PressPoint {
  readonly x: number;
  readonly y: number;
}

/** What asks for a row's menu: a right click, the menu key, or a long press. */
export type MenuSource = MouseEvent<HTMLElement> | KeyboardEvent<HTMLElement> | PressPoint;

/** How long to wait for a pressed button's release before opening anyway. */
const RELEASE_WAIT_MS = 600;

/**
 * Asks for a context menu from a right click, a menu key or a long press: at the pointer, or
 * hanging from the row for a key (a menu key's click lands at 0, 0). A pressed button opens it
 * only once released, as the message rows do, so the release can't light-dismiss the menu it
 * just opened.
 */
export function requestMenu(
  id: number,
  event: MenuSource,
  open: (request: PointMenuRequest) => void,
): void {
  if (!("currentTarget" in event)) {
    open(requestAtPoint(id, event.x, event.y));

    return;
  }

  const pointer = "clientX" in event && !(event.clientX === 0 && event.clientY === 0);

  const request = pointer
    ? requestAtPoint(id, event.clientX, event.clientY)
    : requestAtElement(id, event.currentTarget);

  if (!("buttons" in event) || event.buttons === 0) {
    open(request);

    return;
  }

  const done = () => {
    window.removeEventListener("pointerup", done, true);
    window.removeEventListener("pointercancel", done, true);
    window.clearTimeout(fallback);
    window.setTimeout(() => open(request), 0);
  };

  const fallback = window.setTimeout(done, RELEASE_WAIT_MS);

  window.addEventListener("pointerup", done, true);
  window.addEventListener("pointercancel", done, true);
}

interface AnchorProps {
  readonly trigger: MenuTriggerProps;
  readonly request: PointMenuRequest;
  readonly onClosed: () => void;
}

/**
 * An invisible trigger fixed where the menu should hang, which clicks itself once at mount: the
 * design-system Menu opens from a trigger, and this lets a right click or a key open it anywhere.
 * The message rows' popups do the same (features/messages/row-popup.tsx).
 */
function Anchor({ trigger, request, onClosed }: AnchorProps) {
  const clicked = useRef(false);
  const seenOpen = useRef(false);
  const onClosedRef = useRef(onClosed);
  const expanded = trigger["aria-expanded"];
  const { ref, ...rest } = trigger;

  useLayoutEffect(() => {
    onClosedRef.current = onClosed;
  });

  // biome-ignore lint/correctness/useExhaustiveDependencies: opens once, at mount
  useLayoutEffect(() => {
    if (!clicked.current) {
      clicked.current = true;
      ref.current?.dispatchEvent(
        new MouseEvent("click", {
          bubbles: true,
          cancelable: true,
          detail: request.keyboard ? 0 : 1,
        }),
      );
    }
  }, []);

  useEffect(() => {
    if (expanded) {
      seenOpen.current = true;

      return;
    }

    if (!seenOpen.current) {
      return;
    }

    const timer = window.setTimeout(
      () => onClosedRef.current(),
      readDurationMs("--duration-small-exit"),
    );

    return () => window.clearTimeout(timer);
  }, [expanded]);

  return (
    <button
      {...rest}
      ref={ref}
      type="button"
      tabIndex={-1}
      aria-hidden="true"
      className="point-menu-anchor"
      style={{ left: request.x, top: request.y, width: request.width, height: request.height }}
    />
  );
}

interface PointMenuProps {
  readonly request: PointMenuRequest;
  /** The menu's accessible name. */
  readonly label: string;
  /** The menu finished closing (its exit has played). */
  readonly onClosed: (id: number) => void;
  readonly children: ReactNode;
}

/** A context menu at a point: the design-system Menu, opened by a right click or a key. */
export function PointMenu({ request, label, onClosed, children }: PointMenuProps) {
  return (
    <Menu
      label={label}
      placement="bottom-start"
      trigger={(trigger) => (
        <Anchor trigger={trigger} request={request} onClosed={() => onClosed(request.id)} />
      )}
    >
      {children}
    </Menu>
  );
}
