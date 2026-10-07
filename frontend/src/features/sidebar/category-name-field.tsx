import { type FormEvent, useLayoutEffect, useRef, useState } from "react";
import { CATEGORY_NAME_LIMIT } from "../../store/organize.ts";
import { TextField } from "../../ui/text-field.tsx";

interface CategoryNameFieldProps {
  /** The field's accessible name ("Rename Launch", "New category name"). */
  readonly label: string;
  readonly initial?: string;
  readonly placeholder?: string;
  /** A trimmed name of 1 to 50 characters that differs from `initial`. */
  readonly onSubmit: (name: string) => void;
  readonly onCancel: () => void;
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

  const settle = (name: string) => {
    settled.current = true;

    if (name === "" || name === initial.trim()) {
      onCancel();
    } else {
      onSubmit(name);
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

    settle(name);
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
            onCancel();
          }
        }}
        onBlur={() => {
          if (!settled.current) {
            settle(value.trim());
          }
        }}
      />
    </form>
  );
}
