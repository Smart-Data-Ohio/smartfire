import { Avatar } from "../../ui/avatar.tsx";

/**
 * Someone from another service (an X author, a Fizzy assignee): their picture when the card has
 * one, else initials on a tint picked from `seed`. Decorative: their name is beside it.
 */
export function CardAvatar({
  name,
  seed,
  src,
  size,
}: {
  readonly name: string;
  readonly seed: string;
  readonly src: string | null;
  readonly size: number;
}) {
  return src === null ? (
    <Avatar name={name} userId={seed} size={size} decorative />
  ) : (
    <Avatar name={name} userId={seed} size={size} src={src} decorative />
  );
}
