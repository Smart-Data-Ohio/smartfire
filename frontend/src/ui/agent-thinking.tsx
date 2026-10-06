import { lazy, Suspense } from "react";
import { useResolvedTheme } from "../lib/appearance.ts";
import { useReducedMotion } from "../motion/reduced-motion.ts";
import { useTokenRgb } from "./token-color.ts";
import "./effects.css";

export type AgentThinkingState = "working" | "searching" | "solving" | "composing" | "connecting";

interface AgentThinkingProps {
  readonly size?: 20 | 32 | 64;
  readonly state?: AgentThinkingState;
  /** Announced to screen readers. */
  readonly label?: string;
}

const Orb = lazy(() => import("./agent-thinking-orb.tsx"));

/** Three still dots: shown while the orb's chunk loads. */
function Dots() {
  return (
    <span className="agent-thinking-dots" aria-hidden="true">
      <span />
      <span />
      <span />
    </span>
  );
}

/**
 * An agent at work: Jakub Antalik's thinking-orbs, tinted with the agent violet. The canvas
 * library loads in its own chunk; under reduced motion the orb holds a still frame.
 */
export function AgentThinking({
  size = 20,
  state = "working",
  label = "Agent is thinking",
}: AgentThinkingProps) {
  const reduced = useReducedMotion();
  const theme = useResolvedTheme();
  const color = useTokenRgb("--agent", "rgb(155, 106, 222)");

  return (
    <span
      className="agent-thinking"
      role="img"
      aria-label={label}
      style={{ "--orb-size": `${size}px` }}
    >
      <Suspense fallback={<Dots />}>
        <Orb size={size} state={state} color={color} theme={theme} paused={reduced} />
      </Suspense>
    </span>
  );
}
