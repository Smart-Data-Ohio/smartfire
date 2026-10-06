import { AgentAvatar } from "../../ui/agent-avatar.tsx";
import { Avatar } from "../../ui/avatar.tsx";
import { isAgent, UNKNOWN_NAME, usePresenceStatus, useUser } from "./people.ts";

interface UserAvatarProps {
  readonly userId: number;
  readonly size?: number;
  /** Show the presence dot. */
  readonly presence?: boolean;
  readonly decorative?: boolean;
}

/**
 * Anyone's avatar from the store: an agent gets its generated bot-avatars tile, a person their
 * photo (falling back to initials on their deterministic tint), with an optional presence dot.
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
    return <AgentAvatar seed={`${userId}`} name={name} size={size} decorative={decorative} />;
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
