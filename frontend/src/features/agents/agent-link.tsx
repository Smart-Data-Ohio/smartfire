import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { useStore } from "../../store/store.ts";
import "./agents.css";

interface AgentProfileLinkProps {
  readonly userId: number;
  readonly className?: string;
  /**
   * A second link to the same profile beside the name (the avatar): out of the tab order and
   * hidden from assistive tech, so the name is the one stop.
   */
  readonly duplicate?: boolean;
  readonly children: ReactNode;
}

/** The agent id behind a user, or `null` for people and for bots without an agent row. */
export function useAgentIdOf(userId: number | undefined): number | null {
  return useStore((state) =>
    userId === undefined ? null : (state.users[userId]?.agent?.agentId ?? null),
  );
}

/**
 * Opens an agent's profile from its name or avatar; anyone else's children render as they are, so
 * every author line can wrap its name in one.
 */
export function AgentProfileLink({
  userId,
  className,
  duplicate = false,
  children,
}: AgentProfileLinkProps) {
  const agentId = useAgentIdOf(userId);

  if (agentId === null) {
    return children;
  }

  return (
    <Link
      to="/agents/$agentId"
      params={{ agentId }}
      className={className === undefined ? "agent-link" : `agent-link ${className}`}
      preload={false}
      {...(duplicate ? { tabIndex: -1, "aria-hidden": true } : {})}
    >
      {children}
    </Link>
  );
}
