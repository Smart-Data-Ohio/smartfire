import type { User } from "../../gen/User.ts";
import type { WorkFacts } from "../../gen/WorkFacts.ts";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { AgentAvatar } from "../../ui/agent-avatar.tsx";
import { Avatar } from "../../ui/avatar.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { isAgent } from "../people/people.ts";
import { ownerLabel, WORK_STATUS_LABEL } from "./board-format.ts";

/** A work status as a soft pill: neutral planned, accent in progress, red blocked, green done. */
export function StatusChip({ status }: { readonly status: WorkStatus }) {
  return (
    <span className="board-status" data-status={status}>
      <span className="board-status-dot" aria-hidden="true" />
      {WORK_STATUS_LABEL[status] ?? status}
    </span>
  );
}

/** A user's avatar from the user itself: owners ride on the post, not always in the store. */
export function UserFace({ user, size }: { readonly user: User; readonly size: number }) {
  return isAgent(user) ? (
    <AgentAvatar seed={`${user.id}`} name={user.name} size={size} decorative />
  ) : (
    <Avatar name={user.name} userId={user.id} src={user.avatarUrl} size={size} decorative />
  );
}

/** The owner: their face and name with the agent tag, or "Unassigned" with an empty ring. */
export function OwnerLine({
  work,
  size = 16,
}: {
  readonly work: WorkFacts;
  readonly size?: number;
}) {
  const owner = work.owner;

  return (
    <span className="board-owner" data-unassigned={owner === null || undefined}>
      {owner === null ? (
        <span
          className="board-owner-empty"
          style={{ width: size, height: size }}
          aria-hidden="true"
        >
          <Icon name="user-plus" size={Math.round(size * 0.7)} />
        </span>
      ) : (
        <UserFace user={owner} size={size} />
      )}
      <span
        className="board-owner-name"
        data-inactive={(owner !== null && !work.ownerActive) || undefined}
      >
        {ownerLabel(work)}
      </span>
      {owner !== null && isAgent(owner) ? <span className="message-agent-tag">Agent</span> : null}
    </span>
  );
}

/** A post's tags as small pills; the filtered one stands out. */
export function TagList({
  tags,
  active = "",
}: {
  readonly tags: readonly string[];
  readonly active?: string;
}) {
  if (tags.length === 0) {
    return null;
  }

  return (
    <ul className="board-tags">
      {tags.map((tag) => (
        <li key={tag} className="board-tag" data-active={tag === active || undefined}>
          {tag}
        </li>
      ))}
    </ul>
  );
}
