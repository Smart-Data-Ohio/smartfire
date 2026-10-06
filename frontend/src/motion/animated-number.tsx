import { useState } from "react";

interface AnimatedNumberProps {
  readonly value: number;
  readonly format?: (value: number) => string;
  readonly className?: string;
}

interface Snapshot {
  readonly value: number;
  readonly text: string;
  /** Characters (right-aligned positions) that differ from the previous text. */
  readonly changed: readonly boolean[];
  readonly direction: "up" | "down";
  /** Bumps on every change so changed digits remount and replay their animation. */
  readonly generation: number;
}

const defaultFormat = (value: number) => value.toLocaleString();

function changedPositions(previous: string, next: string): boolean[] {
  const offset = next.length - previous.length;

  return [...next].map((character, index) => previous[index - offset] !== character);
}

/** For each changed digit, its order among the changed ones (capped at 2), for the recipe's stagger. */
function staggerOrder(changed: readonly boolean[]): number[] {
  const orders: number[] = [];
  let next = 0;

  for (const isChanged of changed) {
    orders.push(Math.min(next, 2));

    if (isChanged) {
      next += 1;
    }
  }

  return orders;
}

/**
 * A number whose changed digits pop in (the transitions.dev "number pop-in" recipe): counting up
 * rises from below, counting down drops from above, unchanged digits stay put. Screen readers get
 * the whole number once, not digit by digit.
 */
export function AnimatedNumber({ value, format = defaultFormat, className }: AnimatedNumberProps) {
  const [snapshot, setSnapshot] = useState<Snapshot>(() => ({
    value,
    text: format(value),
    changed: [],
    direction: "up",
    generation: 0,
  }));

  if (snapshot.value !== value) {
    const text = format(value);

    setSnapshot({
      value,
      text,
      changed: changedPositions(snapshot.text, text),
      direction: value < snapshot.value ? "down" : "up",
      generation: snapshot.generation + 1,
    });
  }

  const order = staggerOrder(snapshot.changed);

  return (
    <span className={className === undefined ? "animated-number" : `animated-number ${className}`}>
      <span
        className={`t-digit-group${snapshot.generation > 0 ? " is-animating" : ""}`}
        data-direction={snapshot.direction}
        aria-hidden="true"
      >
        {[...snapshot.text].map((character, index) => {
          const changed = snapshot.changed[index] === true;
          const position = snapshot.text.length - index;

          if (!changed) {
            return (
              <span key={`${position}`} className="t-digit">
                {character}
              </span>
            );
          }

          return (
            <span
              key={`${position}:${snapshot.generation}`}
              className="t-digit"
              data-changed=""
              data-stagger={order[index]}
            >
              {character}
            </span>
          );
        })}
      </span>
      <span className="visually-hidden">{snapshot.text}</span>
    </span>
  );
}
