/**
 * The liquid + menu (its own chunk): the menu's rows squeeze out of the + button as droplets of
 * one liquid surface and fall back into it on close (Jakub Antalik's liquid-gooey, Morph effect).
 * The liquid is only the surface: every row is a real, crisp, focusable menu item on top.
 */
import { Liquid } from "liquid-gooey";
import { type KeyboardEvent, useEffect, useId, useRef, useState } from "react";
import { IconButton } from "../../../ui/icon-button.tsx";
import { Icon } from "../../../ui/icons/icon.tsx";
import { Kbd } from "../../../ui/kbd.tsx";
import type { PlusAction } from "./plus-menu.tsx";

interface GooeyPlusMenuProps {
  readonly actions: readonly PlusAction[];
  readonly label: string;
}

/** Rows are one width, so each one's centre can retract onto the + button's centre. */
const ROW_WIDTH = 224;

/** Rows touch, so open the droplets merge into one surface. */
const ROW_HEIGHT = 34;

/** A group break: the surface pinches into a waist between groups instead of a separator. */
const GROUP_GAP = 9;

const TRIGGER_SIZE = 28;

/** Space between the + button and the lowest row. */
const LIFT = 10;

/** Closed, a row sits shrunk on the + button's centre (its own centre is half a row away). */
const CLOSED_X = TRIGGER_SIZE / 2 - ROW_WIDTH / 2;

const CLOSED_Y = ROW_HEIGHT / 2 - TRIGGER_SIZE / 2;

const CLOSED_SCALE = 0.04;

/** Where the open rows sit: each row's lift above the + button, and the liquid's height. */
interface RowLayout {
  readonly lifts: readonly number[];
  readonly height: number;
}

/** Lays the rows out from the bottom up, adding a waist above each group's first row. */
function layout(actions: readonly PlusAction[]): RowLayout {
  const lifts = actions.map(() => 0);
  let below = 0;

  for (let index = actions.length - 1; index >= 0; index -= 1) {
    lifts[index] = TRIGGER_SIZE + LIFT + below;
    below += ROW_HEIGHT + (actions[index]?.groupStart === true ? GROUP_GAP : 0);
  }

  return { lifts, height: TRIGGER_SIZE + LIFT + below };
}

function rowItems(menu: HTMLElement | null): HTMLElement[] {
  return menu === null ? [] : [...menu.querySelectorAll<HTMLElement>('[role="menuitem"]')];
}

export default function GooeyPlusMenu({ actions, label }: GooeyPlusMenuProps) {
  const id = useId();
  const menuId = `${id}-menu`;
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const focusFirst = useRef(false);
  const count = actions.length;

  // Move focus in once the rows are interactive: the first row for a keyboard open, else the menu.
  useEffect(() => {
    if (!open) {
      return;
    }

    const menu = menuRef.current;

    if (focusFirst.current) {
      rowItems(menu)
        .find((item) => item.getAttribute("aria-disabled") !== "true")
        ?.focus({ preventScroll: true });
    } else {
      menu?.focus({ preventScroll: true });
    }
  }, [open]);

  // An outside press closes it (focus stays where the press put it).
  useEffect(() => {
    if (!open) {
      return;
    }

    const onPointerDown = (event: PointerEvent) => {
      const target = event.target;

      if (
        target instanceof Node &&
        !(menuRef.current?.contains(target) ?? false) &&
        !(triggerRef.current?.contains(target) ?? false)
      ) {
        setOpen(false);
      }
    };

    document.addEventListener("pointerdown", onPointerDown);

    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  const close = (restoreFocus: boolean) => {
    setOpen(false);

    if (restoreFocus) {
      triggerRef.current?.focus({ preventScroll: true });
    }
  };

  const openWith = (keyboard: boolean) => {
    focusFirst.current = keyboard;
    setOpen(true);
  };

  const onMenuKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const items = rowItems(menuRef.current);
    const index = items.findIndex((item) => item.matches(":focus"));
    const focusAt = (next: number) => items.at(next % items.length)?.focus({ preventScroll: true });

    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        focusAt(index + 1);
        break;
      case "ArrowUp":
        event.preventDefault();
        focusAt(index <= 0 ? items.length - 1 : index - 1);
        break;
      case "Home":
        event.preventDefault();
        focusAt(0);
        break;
      case "End":
        event.preventDefault();
        focusAt(items.length - 1);
        break;
      case "Escape":
        event.preventDefault();
        event.stopPropagation();
        close(true);
        break;
      case "Tab":
        close(false);
        break;
      default:
        break;
    }
  };

  const { lifts, height: groupHeight } = layout(actions);

  return (
    <span className="gooey-plus" data-open={open || undefined}>
      <Liquid
        className="gooey-plus-group"
        blur={7}
        contrast={22}
        fill="var(--bg-raised)"
        shadow="0 10px 24px var(--shadow-soft), 0 2px 5px var(--shadow-deep), 0 0 1px var(--shadow-ring)"
        style={{ position: "absolute", width: ROW_WIDTH + 8, height: groupHeight }}
      >
        <Liquid.Item
          className="gooey-plus-seed-slot"
          scale={open ? 1 : CLOSED_SCALE}
          transition="bouncy"
          delay={open ? 0 : count * 18}
        >
          <span className="gooey-plus-seed" />
        </Liquid.Item>
        <div
          ref={menuRef}
          id={menuId}
          role="menu"
          aria-label={label}
          tabIndex={-1}
          className="gooey-plus-menu"
          inert={!open}
          onKeyDown={onMenuKeyDown}
        >
          {actions.map((action, index) => {
            const fromBottom = count - 1 - index;

            return (
              <Liquid.Item
                key={action.id}
                className="gooey-plus-slot"
                x={open ? 0 : CLOSED_X}
                y={open ? -(lifts[index] ?? 0) : CLOSED_Y}
                scale={open ? 1 : CLOSED_SCALE}
                transition="bouncy"
                delay={open ? fromBottom * 28 : index * 18}
              >
                <button
                  type="button"
                  role="menuitem"
                  tabIndex={-1}
                  className="gooey-plus-item"
                  data-group-start={action.groupStart === true || undefined}
                  aria-disabled={action.disabled === true || undefined}
                  style={{
                    width: ROW_WIDTH,
                    height: ROW_HEIGHT,
                    "--row-delay": open ? `${fromBottom * 28 + 70}ms` : "0ms",
                  }}
                  onPointerMove={(event) => event.currentTarget.focus({ preventScroll: true })}
                  onClick={() => {
                    if (action.disabled === true) {
                      return;
                    }

                    close(false);
                    action.onSelect();
                  }}
                >
                  <span className="gooey-plus-glyph">
                    <Icon name={action.icon} />
                  </span>
                  <span className="gooey-plus-label">{action.label}</span>
                  {action.shortcut === undefined ? null : (
                    <span className="gooey-plus-shortcut" aria-hidden="true">
                      <Kbd keys={action.shortcut} />
                    </span>
                  )}
                </button>
              </Liquid.Item>
            );
          })}
        </div>
        <IconButton
          ref={triggerRef}
          icon="plus"
          label={label}
          size="sm"
          className="composer-plus gooey-plus-trigger"
          aria-haspopup="menu"
          aria-expanded={open}
          aria-controls={menuId}
          onMouseDown={(event) => event.preventDefault()}
          onClick={(event) => (open ? close(false) : openWith(event.detail === 0))}
          onKeyDown={(event) => {
            if (event.key === "ArrowUp" || event.key === "ArrowDown") {
              event.preventDefault();
              openWith(true);
            }
          }}
        />
      </Liquid>
    </span>
  );
}
