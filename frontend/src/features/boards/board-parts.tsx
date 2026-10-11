import type { BoardTag } from "../../gen/BoardTag.ts";
import type { User } from "../../gen/User.ts";
import type { WorkFacts } from "../../gen/WorkFacts.ts";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { AgentAvatar } from "../../ui/agent-avatar.tsx";
import { Avatar } from "../../ui/avatar.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { isAgent } from "../people/people.ts";
import { ownerLabel, WORK_STATUS_LABEL } from "./board-format.ts";
import { orderedPostTags, sameTagName } from "./board-tags.ts";

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

/** A curated tag's emoji, sized to sit in a pill; hidden from readers (the name says it). */
export function TagEmoji({ emoji }: { readonly emoji: string | null }) {
  return emoji === null || emoji === "" ? null : (
    <span className="board-tag-emoji" aria-hidden="true">
      {emoji}
    </span>
  );
}

/**
 * A post's tags as small pills; the filtered one stands out. The board's curated tags lead, in
 * its order and with their emoji; free-text tags follow with their "#", as they always have.
 */
export function TagList({
  tags,
  active = "",
  catalog = [],
}: {
  readonly tags: readonly string[];
  readonly active?: string;
  readonly catalog?: readonly BoardTag[];
}) {
  if (tags.length === 0) {
    return null;
  }

  return (
    <ul className="board-tags">
      {orderedPostTags(catalog, tags).map(({ name, tag }) => (
        <li
          key={name}
          className="board-tag"
          data-curated={tag === undefined ? undefined : true}
          data-active={(active !== "" && sameTagName(name, active)) || undefined}
        >
          {tag === undefined ? null : <TagEmoji emoji={tag.emoji} />}
          {tag?.name ?? name}
        </li>
      ))}
    </ul>
  );
}
