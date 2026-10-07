import type { KeyboardEvent, MouseEvent, ReactNode } from "react";
import type { RowMotion } from "./list-motion.ts";

interface ListRowProps {
  /** Enter on a live arrival, leave before it goes; nothing otherwise. */
  readonly motion: RowMotion;
  /** Styling hooks: `unread`, `done`, `busy`… */
  readonly state?: string | undefined;
  /** Right click (or the menu key) on the row. */
  readonly onContextMenu?: (event: MouseEvent<HTMLDivElement>) => void;
  readonly onKeyDown?: (event: KeyboardEvent<HTMLDivElement>) => void;
  /** The hover actions, top right; always shown on touch screens. */
  readonly actions?: ReactNode;
  readonly children: ReactNode;
}

/**
 * One row of a destination's list. The row's own content carries a stretched open button (its
 * first `.list-row-open`), so the whole row is a target; the actions float above it. A leaving
 * row collapses and fades in place; a live arrival rises in. Neither plays under the first load.
 */
export function ListRow({
  motion,
  state,
  onContextMenu,
  onKeyDown,
  actions,
  children,
}: ListRowProps) {
  return (
    // biome-ignore lint/a11y/useSemanticElements: virtua renders its rows inside divs, so the list semantics come from roles
    <div
      role="listitem"
      className={motion === "enter" ? "list-row enter-rise" : "list-row"}
      data-motion={motion}
      data-state={state}
      inert={motion === "leave"}
      onContextMenu={onContextMenu}
      onKeyDown={onKeyDown}
    >
      <div className="list-row-clip">
        <div className="list-row-inner">
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
