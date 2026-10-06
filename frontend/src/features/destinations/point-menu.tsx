import { type ReactNode, useEffect, useLayoutEffect, useRef } from "react";
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
