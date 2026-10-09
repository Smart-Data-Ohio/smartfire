import { useId } from "react";
import type { EventScope } from "../../gen/EventScope.ts";

const SCOPE_LABEL = {
  this_event: "This event",
  this_and_following: "This and following",
} as const satisfies Record<EventScope, string>;

/** "This event" or "This and following" for a series, as a two-segment radio group. */
export function ScopeChoice({
  legend,
  value,
  options,
  onChange,
}: {
  readonly legend: string;
  readonly value: EventScope;
  readonly options: readonly EventScope[];
  readonly onChange: (scope: EventScope) => void;
}) {
  const name = useId();

  return (
    <fieldset className="ev-scope">
      <legend className="field-label">{legend}</legend>
      <div className="ev-scope-track">
        {options.map((option) => (
          <label key={option} className="ev-scope-option">
            <input
              type="radio"
              name={name}
              value={option}
              checked={value === option}
              onChange={() => onChange(option)}
            />
            <span>{SCOPE_LABEL[option]}</span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}
