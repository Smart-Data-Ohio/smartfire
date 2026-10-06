import { useState } from "react";
import { AnimatedNumber } from "../motion/animated-number.tsx";
import "./badge.css";

export type BadgeTone = "danger" | "accent" | "neutral" | "mention";

interface BadgeProps {
  /** 0 hides the badge (it pops out). */
  readonly count: number;
  /** A dot instead of a number: "something new", no count. */
  readonly dot?: boolean;
  readonly tone?: BadgeTone;
  readonly max?: number;
  /** Pin it to the top-right corner of the nearest positioned ancestor (rail icons, avatars). */
  readonly floating?: boolean;
  /** What a screen reader hears, e.g. "3 mentions". Defaults to the number. */
  readonly label?: string;
}

/**
 * An unread or mention count that pops in on a spring (the transitions.dev notification badge)
 * and ticks its digits when the count changes.
 */
export function Badge({
  count,
  dot = false,
  tone = "danger",
  max = 99,
  floating = false,
  label,
}: BadgeProps) {
  // Keep showing the last count while the badge pops out, so it doesn't flash "0".
  const [shown, setShown] = useState(count);

  if (count > 0 && count !== shown) {
    setShown(count);
  }

  const open = count > 0;
  const format = (value: number) => (value > max ? `${max}+` : `${value}`);

  return (
    <span
      className="badge t-badge"
      data-open={open}
      data-tone={tone}
      data-dot={dot || undefined}
      data-floating={floating || undefined}
    >
      <span className="badge-pill t-badge-dot" aria-hidden="true">
        {dot ? null : <AnimatedNumber value={shown} format={format} />}
      </span>
      {open ? <span className="visually-hidden">{label ?? format(count)}</span> : null}
    </span>
  );
}
