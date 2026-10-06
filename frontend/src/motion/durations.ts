/** Milliseconds in a CSS time value ("160ms", "0.16s"); 0 for anything unparsable. */
export function parseTimeMs(value: string): number {
  const match = /^\s*(-?[\d.]+)(ms|s)\s*$/.exec(value);

  if (match === null) {
    return 0;
  }

  const amount = Number(match[1]);

  return match[2] === "s" ? amount * 1000 : amount;
}

/**
 * Reads a motion token (`--duration-tooltip-cold`) as milliseconds, so JavaScript timers follow
 * the same tokens as the CSS. Reduced motion is already folded into the token's value.
 */
export function readDurationMs(token: string, element: Element = document.documentElement): number {
  return parseTimeMs(getComputedStyle(element).getPropertyValue(token));
}

/** The longest duration plus delay of the element's current transitions and animations. */
export function longestMotionMs(element: Element): number {
  const style = getComputedStyle(element);

  const longest = (durations: string, delays: string) => {
    const delayList = delays.split(",");

    return durations
      .split(",")
      .reduce(
        (max, duration, index) =>
          Math.max(max, parseTimeMs(duration) + parseTimeMs(delayList[index] ?? "0s")),
        0,
      );
  };

  return Math.max(
    longest(style.transitionDuration, style.transitionDelay),
    longest(style.animationDuration, style.animationDelay),
  );
}
