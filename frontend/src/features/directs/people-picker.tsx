import { type KeyboardEvent, useEffect, useId, useRef, useState } from "react";
import type { DirectCandidate } from "../../gen/DirectCandidate.ts";
import { useStore } from "../../store/store.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { UNKNOWN_NAME, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { MAX_OTHERS, pickerOptions, removeLast, toggleSelected } from "./picker.ts";
import "./directs.css";

interface PeoplePickerProps {
  /** `null` while the list loads. */
  readonly candidates: readonly DirectCandidate[] | null;
  readonly error: string | null;
  readonly selected: readonly number[];
  readonly onSelectedChange: (selected: readonly number[]) => void;
  /** People who can't be picked (a DM's current members). */
  readonly excluded?: ReadonlySet<number>;
  /** People already in, shown as chips that can't be taken off (a 1:1's other person). */
  readonly fixed?: readonly number[];
  /** How many chips fit. */
  readonly limit?: number;
  /** Enter in an empty field with people chosen, or ⌘/Ctrl+Enter any time. */
  readonly onSubmit: () => void;
  readonly label: string;
  readonly placeholder: string;
}

function Chip({
  userId,
  onRemove,
}: {
  readonly userId: number;
  /** Absent for a fixed chip. */
  readonly onRemove?: () => void;
}) {
  const name = useUser(userId)?.name ?? UNKNOWN_NAME;

  if (onRemove === undefined) {
    return (
      <span className="picker-chip" data-fixed>
        <UserAvatar userId={userId} size={20} decorative />
        <span className="picker-chip-name">{name}</span>
      </span>
    );
  }

  return (
    <span className="picker-chip enter-chip">
      <UserAvatar userId={userId} size={20} decorative />
      <span className="picker-chip-name">{name}</span>
      <button
        type="button"
        className="picker-chip-remove"
        aria-label={`Remove ${name}`}
        tabIndex={-1}
        onClick={onRemove}
      >
        <Icon name="x" size={12} />
      </button>
    </span>
  );
}

function PickerSkeleton() {
  return (
    <div className="picker-skeleton" aria-hidden="true">
      {[64, 48, 72, 56].map((width) => (
        // The widths are distinct, so each one names its row.
        <span key={width} className="picker-skeleton-row">
          <Skeleton width={28} height={28} radius="md" />
          <Skeleton width={`${width}%`} height={10} />
        </span>
      ))}
    </div>
  );
}

/**
 * A people search with chips (Slack's "To:" field): type to filter, ↑/↓ and Enter (or a click)
 * to add someone as a chip, Backspace in the empty field to take the last chip off. Enter with
 * an empty field submits once someone is chosen.
 */
export function PeoplePicker({
  candidates,
  error,
  selected,
  onSelectedChange,
  excluded,
  fixed = [],
  limit = MAX_OTHERS,
  onSubmit,
  label,
  placeholder,
}: PeoplePickerProps) {
  const id = useId();
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const users = useStore((state) => state.users);
  const presence = useStore((state) => state.presence);
  const hidden = new Set([...selected, ...(excluded ?? [])]);
  const options = candidates === null ? [] : pickerOptions(candidates, users, query, hidden);
  const active = Math.min(activeIndex, Math.max(options.length - 1, 0));
  const activeOption = options[active];
  const full = selected.length >= limit;

  useEffect(() => {
    if (activeOption !== undefined) {
      document.getElementById(`${id}-${activeOption.userId}`)?.scrollIntoView({ block: "nearest" });
    }
  }, [activeOption, id]);

  const pick = (userId: number) => {
    onSelectedChange(toggleSelected(selected, userId, limit));
    setQuery("");
    setActiveIndex(0);
    inputRef.current?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.nativeEvent.isComposing) {
      return;
    }

    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();

      if (options.length > 0) {
        const step = event.key === "ArrowDown" ? 1 : -1;

        setActiveIndex((active + step + options.length) % options.length);
      }

      return;
    }

    if (event.key === "Backspace" && query === "" && selected.length > 0) {
      event.preventDefault();
      onSelectedChange(removeLast(selected));

      return;
    }

    if (event.key !== "Enter") {
      return;
    }

    event.preventDefault();

    const submitNow =
      event.metaKey || event.ctrlKey || (query.trim() === "" && selected.length > 0);

    if (submitNow) {
      onSubmit();
    } else if (activeOption !== undefined && !full) {
      pick(activeOption.userId);
    }
  };

  return (
    <div className="picker">
      {/* A label, so a click anywhere in the field (between chips) focuses the input. */}
      <label className="picker-field">
        <span className="picker-to" id={`${id}-label`}>
          {label}
        </span>
        {fixed.map((userId) => (
          <Chip key={userId} userId={userId} />
        ))}
        {selected.map((userId) => (
          <Chip key={userId} userId={userId} onRemove={() => pick(userId)} />
        ))}
        <input
          ref={inputRef}
          className="picker-input"
          type="text"
          role="combobox"
          aria-labelledby={`${id}-label`}
          aria-expanded="true"
          aria-controls={`${id}-list`}
          aria-autocomplete="list"
          aria-activedescendant={
            activeOption === undefined ? undefined : `${id}-${activeOption.userId}`
          }
          placeholder={selected.length === 0 && fixed.length === 0 ? placeholder : ""}
          autoComplete="off"
          spellCheck={false}
          value={query}
          data-autofocus
          onChange={(event) => {
            setQuery(event.target.value);
            setActiveIndex(0);
          }}
          onKeyDown={onKeyDown}
        />
      </label>
      {full ? (
        <p className="picker-note">That's everyone a conversation can hold ({limit + 1} people).</p>
      ) : null}
      {error !== null && candidates === null ? (
        <p className="picker-note picker-error">Couldn't load people: {error}</p>
      ) : null}
      {candidates === null && error === null ? <PickerSkeleton /> : null}
      {candidates === null ? null : (
        <div
          id={`${id}-list`}
          role="listbox"
          aria-label="People"
          aria-multiselectable="true"
          className="picker-list"
        >
          {options.map((option) => {
            const isActive = option === activeOption;
            const status = presence[option.userId]?.statusText ?? null;

            return (
              // The input owns the keyboard (aria-activedescendant); options take clicks.
              // biome-ignore lint/a11y/useKeyWithClickEvents: the combobox input handles the keys.
              <div
                key={option.userId}
                id={`${id}-${option.userId}`}
                role="option"
                tabIndex={-1}
                aria-selected={false}
                aria-disabled={full || undefined}
                className="picker-option"
                data-active={isActive || undefined}
                onPointerMove={isActive ? undefined : () => setActiveIndex(options.indexOf(option))}
                onClick={() => (full ? undefined : pick(option.userId))}
              >
                <UserAvatar userId={option.userId} size={28} presence decorative />
                <span className="picker-option-text">
                  <span className="picker-option-name">{option.name}</span>
                  {status === null ? null : <span className="picker-option-status">{status}</span>}
                </span>
                {option.agent ? <span className="picker-tag">Agent</span> : null}
                {option.starred ? (
                  <span className="picker-star" role="img" aria-label="Starred">
                    <Icon name="star" size={14} />
                  </span>
                ) : null}
              </div>
            );
          })}
          {options.length === 0 ? (
            <p className="picker-empty">
              {query.trim() === ""
                ? "Everyone's already here."
                : `Nobody matches “${query.trim()}”.`}
            </p>
          ) : null}
        </div>
      )}
    </div>
  );
}
