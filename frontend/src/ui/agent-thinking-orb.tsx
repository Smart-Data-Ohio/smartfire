import { ThinkingOrb } from "thinking-orbs";
import type { AgentThinkingState } from "./agent-thinking.tsx";

interface AgentThinkingOrbProps {
  readonly size: 20 | 32 | 64;
  readonly state: AgentThinkingState;
  readonly color: string;
  readonly theme: "light" | "dark";
  readonly paused: boolean;
}

/** The lazily loaded half of AgentThinking. */
export default function AgentThinkingOrb({
  size,
  state,
  color,
  theme,
  paused,
}: AgentThinkingOrbProps) {
  return (
    <ThinkingOrb
      size={size}
      state={state}
      color={color}
      theme={theme}
      paused={paused}
      aria-hidden="true"
    />
  );
}
