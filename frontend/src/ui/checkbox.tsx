import { type ReactNode, useLayoutEffect, useRef } from "react";
import "./checkbox.css";

interface CheckboxProps {
  readonly checked: boolean | "mixed";
  readonly onCheckedChange: (checked: boolean) => void;
  readonly label: ReactNode;
  readonly disabled?: boolean;
}

/**
 * A native checkbox (so forms, Space, and assistive tech all just work) dressed as a 16 px box
 * whose mark draws itself in: the transitions.dev checkbox check.
 */
export function Checkbox({ checked, onCheckedChange, label, disabled }: CheckboxProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);

  useLayoutEffect(() => {
    if (inputRef.current !== null) {
      inputRef.current.indeterminate = checked === "mixed";
    }
  }, [checked]);

  return (
    <label className="checkbox-row" data-disabled={disabled || undefined}>
      <span className="checkbox t-check" data-checked={checked}>
        <input
          ref={inputRef}
          type="checkbox"
          className="checkbox-input"
          checked={checked === true}
          disabled={disabled}
          onChange={(event) => onCheckedChange(event.target.checked)}
        />
        <svg
          width="12"
          height="12"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.25"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          {checked === "mixed" ? (
            <path d="M4 8h8" pathLength={15} />
          ) : (
            <path d="M3.5 8.5l3 3 6-6.5" pathLength={15} />
          )}
        </svg>
      </span>
      <span>{label}</span>
    </label>
  );
}
