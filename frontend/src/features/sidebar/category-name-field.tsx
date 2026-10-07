import { type FormEvent, useLayoutEffect, useRef, useState } from "react";
import { CATEGORY_NAME_LIMIT } from "../../store/organize.ts";
import { TextField } from "../../ui/text-field.tsx";

interface CategoryNameFieldProps {
  /** The field's accessible name ("Rename Launch", "New category name"). */
  readonly label: string;
  readonly initial?: string;
  readonly placeholder?: string;
  /**
   * A trimmed name of 1 to 50 characters that differs from `initial`. `refocus` says focus is
   * the sidebar's to place, since the field is going (Enter, or a blur that went nowhere); a blur
   * to another control leaves focus there.
   */
  readonly onSubmit: (name: string, refocus: boolean) => void;
  /** Escape, or nothing (new) to save: `refocus` as for `onSubmit`. */
  readonly onCancel: (refocus: boolean) => void;
}

/**
 * The inline field that names a category, in place of its heading: Enter saves, Escape cancels,
 * and leaving it saves a changed name (an empty one just cancels). A blank name on Enter shakes
 * the field with the reason.
 */
export function CategoryNameField({
  label,
  initial = "",
  placeholder,
  onSubmit,
  onCancel,
}: CategoryNameFieldProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const settled = useRef(false);
  const [value, setValue] = useState(initial);
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);

  useLayoutEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  const settle = (name: string, refocus: boolean) => {
    settled.current = true;

    if (name === "" || name === initial.trim()) {
      onCancel(refocus);
    } else {
      onSubmit(name, refocus);
    }
  };

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const name = value.trim();

    if (name === "") {
      setError("Give the category a name");
      setAttempt((count) => count + 1);

      return;
    }

    settle(name, true);
  };

  return (
    <form className="sidebar-name-field" onSubmit={submit}>
      <TextField
        ref={inputRef}
        label={label}
        value={value}
        placeholder={placeholder}
        maxLength={CATEGORY_NAME_LIMIT}
        enterKeyHint="done"
        autoComplete="off"
        spellCheck={false}
        error={error}
        attempt={attempt}
        onChange={(event) => {
          setValue(event.currentTarget.value);
          setError(undefined);
        }}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            settled.current = true;
            onCancel(true);
          }
        }}
        onBlur={(event) => {
          if (!settled.current) {
            settle(value.trim(), event.relatedTarget === null);
          }
        }}
      />
    </form>
  );
}
