import { UserAvatar } from "../people/user-avatar.tsx";

interface GroupAvatarsProps {
  /** The DM's other members, in membership order; the first two show. */
  readonly ids: readonly number[];
  /** The square the pair fills, in px. */
  readonly size?: number;
}

/**
 * A group DM's glyph (Slack's): the first two members' avatars stacked corner to corner, the
 * front one cut out of the back one by a ring of the surface colour. At sidebar sizes initials
 * only fit once, so the back tile shows its colour (or photo) and the front carries the letters.
 * Decorative: the row's name already lists the members.
 */
export function GroupAvatars({ ids, size = 20 }: GroupAvatarsProps) {
  const [first, second] = ids;
  const inner = Math.round(size * 0.75);

  if (first === undefined) {
    return null;
  }

  if (second === undefined) {
    return <UserAvatar userId={first} size={size} decorative />;
  }

  return (
    <span className="group-avatars" style={{ "--group-size": `${size}px` }} aria-hidden="true">
      <span className="group-avatars-back">
        <UserAvatar userId={first} size={inner} decorative />
      </span>
      <span className="group-avatars-front">
        <UserAvatar userId={second} size={inner} decorative />
      </span>
    </span>
  );
}
