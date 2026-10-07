import { type ComponentPropsWithRef, useId } from "react";

interface WorkTextAreaProps extends Omit<ComponentPropsWithRef<"textarea">, "id"> {
  readonly label: string;
  readonly error?: string | undefined;
  /** Hidden label, for an editor whose heading already names it. */
  readonly hideLabel?: boolean;
}

/**
 * A labelled, growing textarea for the work forms (the result, a handoff's summary, links and
 * questions), with its error read out beside it. `src/ui` has no textarea yet; this keeps the
 * text field's look.
 */
export function WorkTextArea({
  label,
  error,
  hideLabel = false,
  className,
  ...rest
}: WorkTextAreaProps) {
  const id = useId();
  const invalid = error !== undefined && error !== "";

  return (
    <div className="work-field">
      <label htmlFor={id} className={hideLabel ? "visually-hidden" : "work-field-label"}>
        {label}
      </label>
      <textarea
        {...rest}
        id={id}
        className={className === undefined ? "work-textarea" : `work-textarea ${className}`}
        aria-invalid={invalid || undefined}
        aria-describedby={invalid ? `${id}-error` : undefined}
      />
      {invalid ? (
        <p id={`${id}-error`} className="work-field-error">
          {error}
        </p>
      ) : null}
    </div>
  );
}
