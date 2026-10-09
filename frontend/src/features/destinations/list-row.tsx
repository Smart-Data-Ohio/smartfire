import { type KeyboardEvent, type MouseEvent, type ReactNode, useRef } from "react";
import { Icon } from "../../ui/icons/icon.tsx";
import type { RowMotion } from "./list-motion.ts";
import { type RowSwipe, useRowGestures } from "./row-gestures.ts";

interface ListRowProps {
  /** Enter on a live arrival, leave before it goes; nothing otherwise. */
  readonly motion: RowMotion;
  /** Styling hooks: `unread`, `done`, `busy`… */
  readonly state?: string | undefined;
  /** Right click (or the menu key) on the row. */
  readonly onContextMenu?: ((event: MouseEvent<HTMLDivElement>) => void) | undefined;
  readonly onKeyDown?: (event: KeyboardEvent<HTMLDivElement>) => void;
  /** The hover actions, top right. Touch screens show them only to a keyboard's focus. */
  readonly actions?: ReactNode;
  /** A finger resting on the row: open its menu there (an action sheet on phones). */
  readonly onLongPress?: ((x: number, y: number) => void) | undefined;
  /** The row's one swipe action, which its menu and keys also offer. */
  readonly swipe?: RowSwipe | undefined;
  readonly children: ReactNode;
}

/**
 * One row of a destination's list. The row's own content carries a stretched open button (its
 * first `.list-row-open`), so the whole row is a target; the actions float above it. On touch
 * screens a long press opens the row's menu and a swipe slides it over its one action (see
 * row-gestures.ts). A leaving row collapses and fades in place; a live arrival rises in. Neither
 * plays under the first load.
 */
export function ListRow({
  motion,
  state,
  onContextMenu,
  onKeyDown,
  actions,
  onLongPress,
  swipe,
  children,
}: ListRowProps) {
  const rowRef = useRef<HTMLDivElement | null>(null);
  const innerRef = useRef<HTMLDivElement | null>(null);
  const gestures = useRowGestures(rowRef, innerRef, swipe, onLongPress);

  return (
    // biome-ignore lint/a11y/useSemanticElements: virtua renders its rows inside divs, so the list semantics come from roles
    <div
      ref={rowRef}
      role="listitem"
      className={motion === "enter" ? "list-row enter-rise" : "list-row"}
      data-motion={motion}
      data-state={state}
      inert={motion === "leave"}
      {...gestures.props}
      onContextMenu={(event) => {
        if (gestures.claimContextMenu()) {
          event.preventDefault();

          return;
        }

        onContextMenu?.(event);
      }}
      onKeyDown={onKeyDown}
    >
      <div className="list-row-clip">
        {swipe === undefined ? null : (
          <div className="list-row-swipe" data-tone={swipe.tone} aria-hidden="true">
            <Icon name={swipe.icon} size={20} />
            <span className="list-row-swipe-label">{swipe.label}</span>
          </div>
        )}
        <div ref={innerRef} className="list-row-inner">
          {children}
          {actions === undefined ? null : <div className="list-row-bar">{actions}</div>}
        </div>
      </div>
    </div>
  );
}

/** Moves focus to the previous or next row's open button; `false` at either end. */
export function focusSiblingRow(from: Element, step: 1 | -1): boolean {
  const row = from.closest(".list-row");
  const list = row?.closest("[data-list-root]") ?? null;

  if (row === null || list === null) {
    return false;
  }

  const rows = [...list.querySelectorAll(".list-row:not([data-motion='leave'])")];
  const next = rows[rows.indexOf(row) + step];
  const target = next?.querySelector<HTMLElement>(".list-row-open");

  if (target === null || target === undefined) {
    return false;
  }

  target.focus();
  target.scrollIntoView({ block: "nearest" });

  return true;
}
