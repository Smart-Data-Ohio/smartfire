import { type ReactNode, useState } from "react";
import "./toggle.css";

interface ToggleProps {
  readonly checked: boolean;
  readonly onCheckedChange: (checked: boolean) => void;
  readonly label: ReactNode;
  readonly description?: ReactNode;
  readonly disabled?: boolean;
}

/**
 * A switch (role="switch") whose thumb travels with a small overshoot (the transitions.dev
 * toggle). The whole row is the label, so clicking the text flips it too.
 */
export function Toggle({ checked, onCheckedChange, label, description, disabled }: ToggleProps) {
  // Only animate after the first flip, so a switch that renders "on" doesn't slide on mount.
  const [touched, setTouched] = useState(false);

  return (
    <label className="field-row" data-disabled={disabled || undefined}>
      <span className="field-text">
        <span className="field-label">{label}</span>
        {description === undefined ? null : <span className="field-hint">{description}</span>}
      </span>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        className="toggle t-toggle"
        data-on={checked}
        data-init={touched || undefined}
        onClick={() => {
          setTouched(true);
          onCheckedChange(!checked);
        }}
      >
        <span className="toggle-thumb t-toggle-thumb" />
      </button>
    </label>
  );
}
