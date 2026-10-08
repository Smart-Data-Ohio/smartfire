import type { User } from "../../store/model.ts";
import { AgentAvatar, type AgentAvatarState } from "../../ui/agent-avatar.tsx";
import { Avatar } from "../../ui/avatar.tsx";
import { agentTone, identityOf, toneLabel } from "./agent-identity.ts";
import { isAgent, UNKNOWN_NAME, usePresenceStatus, useUser } from "./people.ts";

interface UserAvatarProps {
  readonly userId: number;
  readonly size?: number;
  /** Show the presence dot (for an agent: the thinking orbs while it works, or its status dot). */
  readonly presence?: boolean;
  readonly decorative?: boolean;
}

/**
 * Anyone's avatar from the store: an agent gets its own icon or a generated bot-avatars tile
 * (greyed out while suspended or deactivated), a person their photo (falling back to initials on
 * their deterministic tint), with an optional presence dot.
 */
export function UserAvatar({
  userId,
  size = 36,
  presence = false,
  decorative = false,
}: UserAvatarProps) {
  const user = useUser(userId);
  const status = usePresenceStatus(presence ? userId : undefined);
  const name = user?.name ?? UNKNOWN_NAME;

  if (isAgent(user)) {
    const corner = agentCorner(user, presence);

    return (
      <AgentAvatar
        seed={`${userId}`}
        name={name}
        size={size}
        decorative={decorative}
        icon={user?.avatarIcon ?? null}
        {...corner}
      />
    );
  }

  return (
    <Avatar
      name={name}
      userId={userId}
      size={size}
      decorative={decorative}
      {...(user === undefined ? {} : { src: user.avatarUrl })}
      {...(status === undefined ? {} : { presence: status })}
    />
  );
}

/** The props an agent's avatar takes for its state. */
interface AgentCorner {
  readonly state?: AgentAvatarState;
  readonly stateLabel?: string;
}

/** What an agent's avatar shows: greyed out when it can't act, its status corner in lists. */
function agentCorner(user: User | undefined, presence: boolean): AgentCorner {
  const identity = identityOf(user);
  const tone = agentTone(identity);
  const status = identity.kind === "agent" ? identity.status : "idle";

  switch (tone) {
    case "suspended":
    case "inactive":
      return { state: "muted", stateLabel: toneLabel(tone, status).toLowerCase() };
    case "working":
    case "waiting":
    case "failed":
      return presence ? { state: tone, stateLabel: toneLabel(tone, status).toLowerCase() } : {};
    default:
      return {};
  }
}
