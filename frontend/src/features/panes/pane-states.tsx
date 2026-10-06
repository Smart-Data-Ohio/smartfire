import { useRef } from "react";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";

/** A quiet empty state: a glyph in a soft tile, a title and one short sentence. */
export function PaneEmpty({
  icon,
  title,
  text,
}: {
  readonly icon: IconName;
  readonly title: string;
  readonly text: string;
}) {
  return (
    <div className="pane-empty enter-fade">
      <span className="pane-empty-glyph" aria-hidden="true">
        <Icon name={icon} size={20} />
      </span>
      <p className="pane-empty-title">{title}</p>
      <p className="pane-empty-text">{text}</p>
    </div>
  );
}

/** A load that failed: what happened and a way to try again. */
export function PaneError({
  message,
  onRetry,
}: {
  readonly message: string;
  readonly onRetry?: (() => void) | undefined;
}) {
  return (
    <div className="pane-empty pane-error enter-fade" role="alert">
      <span className="pane-empty-glyph" aria-hidden="true">
        <Icon name="alert" size={20} />
      </span>
      <p className="pane-empty-title">{message}</p>
      {onRetry === undefined ? null : (
        <Button variant="secondary" size="sm" icon="rotate-ccw" onClick={onRetry}>
          Try again
        </Button>
      )}
    </div>
  );
}

/** Placeholder rows for a list pane: an avatar or icon and two lines. */
export function PaneListSkeleton({
  rows = 6,
  square = 32,
}: {
  readonly rows?: number;
  readonly square?: number;
}) {
  const widths = [72, 54, 86, 61, 47, 78, 66, 58].slice(0, rows);

  return (
    <div className="pane-skeleton">
      {widths.map((width) => (
        // The widths are distinct, so each one names its row.
        <div key={width} className="pane-skeleton-row">
          <Skeleton width={square} height={square} radius="md" />
          <div className="pane-skeleton-lines">
            <Skeleton width={`${Math.max(36, width - 24)}%`} height={11} />
            <Skeleton width={`${width}%`} height={10} />
          </div>
        </div>
      ))}
    </div>
  );
}

interface PaneSearchProps {
  readonly value: string;
  readonly onValueChange: (value: string) => void;
  /** The field's accessible name and placeholder. */
  readonly label: string;
}

/** A compact search field with a clear button, for filtering a pane's list. */
export function PaneSearch({ value, onValueChange, label }: PaneSearchProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);

  return (
    <div className="pane-search" data-filled={value !== "" || undefined}>
      <Icon name="search" size={14} className="pane-search-icon" />
      <input
        ref={inputRef}
        type="search"
        className="pane-search-input"
        value={value}
        placeholder={label}
        aria-label={label}
        data-pane-autofocus
        onChange={(event) => onValueChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape" && value !== "") {
            event.preventDefault();
            event.stopPropagation();
            onValueChange("");
          }
        }}
      />
      {value === "" ? null : (
        <IconButton
          icon="x"
          label="Clear search"
          size="sm"
          className="pane-search-clear"
          onClick={() => {
            onValueChange("");
            inputRef.current?.focus();
          }}
        />
      )}
    </div>
  );
}
