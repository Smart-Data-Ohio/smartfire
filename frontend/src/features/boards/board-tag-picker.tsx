import { type RefObject, useId, useState } from "react";
import type { BoardTag } from "../../gen/BoardTag.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { TagEmoji } from "./board-parts.tsx";

interface BoardTagPickerProps {
  readonly tags: readonly BoardTag[];
  readonly selected: readonly number[];
  readonly required: boolean;
  /** The board's default, named in the hint so the pre-selected chip explains itself. */
  readonly defaultName: string | undefined;
  /** The default the server will add because nothing curated is picked on a board that requires one. */
  readonly added: string | undefined;
  readonly error: string | undefined;
  /** Bump to shake again for the same error (a second failed submit). */
  readonly attempt: number;
  readonly onToggle: (id: number) => void;
  /** The first chip, for the form to focus when a required tag is missing. */
  readonly firstRef?: RefObject<HTMLButtonElement | null>;
}

/**
 * The board's curated tags as toggle chips (Discord's forum tag picker): each shows its emoji
 * and name, pressed when picked, in the board's order. When the board requires a tag, the legend
 * says so and a submit without one shakes the row and names the problem under it.
 */
export function BoardTagPicker({
  tags,
  selected,
  required,
  defaultName,
  added,
  error,
  attempt,
  onToggle,
  firstRef,
}: BoardTagPickerProps) {
  const id = useId();
  const [seen, setSeen] = useState({ error, attempt });
  const [shaking, setShaking] = useState(false);
  const invalid = error !== undefined && error !== "";

  if (seen.error !== error || seen.attempt !== attempt) {
    setSeen({ error, attempt });

    if (invalid) setShaking(true);
  }

  const hint =
    added !== undefined
      ? `${added} will be added, since this board requires a tag.`
      : defaultName !== undefined
        ? `${defaultName} is the board's default.`
        : required
          ? "Pick at least one."
          : "Pick any that fit.";

  return (
    <fieldset
      className={`field board-tag-picker t-input-wrap${invalid ? " is-error" : ""}`}
      aria-describedby={invalid ? `${id}-error` : `${id}-hint`}
      aria-invalid={invalid || undefined}
    >
      <legend className="field-label board-tag-picker-legend">
        Board tags
        {required ? (
          <span className="field-required">Required</span>
        ) : (
          <span className="field-optional">optional</span>
        )}
      </legend>
      <ul
        className={`board-tag-options t-input${shaking ? " is-shaking" : ""}`}
        onAnimationEnd={() => setShaking(false)}
      >
        {tags.map((tag, index) => {
          const on = selected.includes(tag.id);

          return (
            <li key={tag.id}>
              <button
                type="button"
                ref={index === 0 ? firstRef : undefined}
                className="board-tag-option"
                aria-pressed={on}
                onClick={() => onToggle(tag.id)}
              >
                <span className="board-tag-option-face">
                  <TagEmoji emoji={tag.emoji} />
                  <span className="board-tag-option-name">{tag.name}</span>
                  <Icon name="check" size={12} className="board-tag-option-check" />
                </span>
              </button>
            </li>
          );
        })}
      </ul>
      {invalid ? null : (
        <p id={`${id}-hint`} className="field-hint">
          {hint}
        </p>
      )}
      <p id={`${id}-error`} className="field-error t-error-msg" aria-live="polite">
        {invalid ? error : ""}
      </p>
    </fieldset>
  );
}
