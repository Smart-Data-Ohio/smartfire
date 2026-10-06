import { useEffect, useRef, useState } from "react";
import { afterExit } from "./presence.ts";
import "./text-swap.css";

type Phase = "idle" | "exit" | "enter";

interface TextSwapProps {
  /** The text to show. Changing it plays the swap: the old text lifts out, the new one settles in. */
  readonly children: string;
  /**
   * Every label this slot can show. They are laid out invisibly in the same cell, so the slot is
   * as wide as the widest one and swapping never nudges the layout around it.
   */
  readonly reserve?: readonly string[];
  readonly className?: string;
}

const PHASE_CLASS = { idle: "", exit: " is-exit", enter: " is-enter-start" } as const;

/**
 * The transitions.dev "text states swap" recipe as a component: exit, swap the text, snap to the
 * enter start without a transition, then release so the new text settles in.
 */
export function TextSwap({ children, reserve = [], className }: TextSwapProps) {
  const [shown, setShown] = useState(children);
  const [phase, setPhase] = useState<Phase>("idle");
  const ref = useRef<HTMLSpanElement | null>(null);

  if (phase === "idle" && children !== shown) {
    setPhase("exit");
  }

  useEffect(() => {
    const element = ref.current;

    if (phase !== "exit" || element === null) {
      return;
    }

    return afterExit(element, () => {
      setShown(children);
      setPhase("enter");
    });
  }, [phase, children]);

  useEffect(() => {
    const element = ref.current;

    if (phase !== "enter" || element === null) {
      return;
    }

    // Commit the enter-start styles before releasing them, so the browser has a "from" to tween.
    element.getBoundingClientRect();

    const frame = requestAnimationFrame(() => setPhase("idle"));

    return () => cancelAnimationFrame(frame);
  }, [phase]);

  const labels = reserve.includes(children) ? reserve : [...reserve, children];

  return (
    <span className={className === undefined ? "text-swap" : `text-swap ${className}`}>
      {labels.map((label) => (
        <span key={label} className="text-swap-reserve" aria-hidden="true">
          {label}
        </span>
      ))}
      <span ref={ref} className={`text-swap-text t-text-swap${PHASE_CLASS[phase]}`}>
        {shown}
      </span>
    </span>
  );
}
