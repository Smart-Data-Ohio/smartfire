import { type ComponentPropsWithRef, useId, useState } from "react";
import "./text-field.css";

interface TextFieldProps extends Omit<ComponentPropsWithRef<"input">, "id"> {
  readonly label: string;
  readonly hint?: string;
  /** Setting an error shakes the field once (the transitions.dev error-state shake). */
  readonly error?: string | undefined;
  /** Bump to shake again for the same error (a second failed submit). */
  readonly attempt?: number;
}

export function TextField({ label, hint, error, attempt = 0, className, ...rest }: TextFieldProps) {
  const id = useId();
  const [seen, setSeen] = useState({ error, attempt });
  const [shaking, setShaking] = useState(false);
  // Keep the last message while it fades out, so the text doesn't vanish before the fade.
  const [message, setMessage] = useState(error ?? "");

  if (seen.error !== error || seen.attempt !== attempt) {
    setSeen({ error, attempt });

    if (error !== undefined && error !== "") {
      setShaking(true);
      setMessage(error);
    }
  }

  const invalid = error !== undefined && error !== "";
  const describedBy = invalid ? `${id}-error` : hint === undefined ? undefined : `${id}-hint`;

  return (
    <div className={`field t-input-wrap${invalid ? " is-error" : ""}`}>
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      <input
        {...rest}
        id={id}
        className={`input t-input${invalid ? " is-error" : ""}${shaking ? " is-shaking" : ""}${
          className === undefined ? "" : ` ${className}`
        }`}
        aria-invalid={invalid || undefined}
        aria-describedby={describedBy}
        onAnimationEnd={() => setShaking(false)}
      />
      {hint === undefined || invalid ? null : (
        <p id={`${id}-hint`} className="field-hint">
          {hint}
        </p>
      )}
      <p id={`${id}-error`} className="field-error t-error-msg" aria-live="polite">
        {invalid ? error : message}
      </p>
    </div>
  );
}
