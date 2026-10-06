import { useState } from "react";
import { avatarHue } from "./avatar-palette.ts";
import "./avatar.css";

export type PresenceStatus = "online" | "away" | "dnd" | "offline";

interface AvatarProps {
  readonly name: string;
  /** Picks the tile colour, so it stays put when someone changes their name. Defaults to the name. */
  readonly userId?: string | number | undefined;
  readonly src?: string;
  /** Edge length in px: 20 (inline mentions), 24 (sidebar DMs), 36 (messages), 80 (profiles). */
  readonly size?: number;
  readonly presence?: PresenceStatus;
  /** Hide it from assistive tech when the name is already right next to it (message rows). */
  readonly decorative?: boolean;
}

const PRESENCE_LABEL = {
  online: "online",
  away: "away",
  dnd: "do not disturb",
  offline: "offline",
} as const;

function initials(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  const letters = words.length > 1 ? [words[0], words.at(-1)] : [words[0]];

  return letters
    .map((word) => [...(word ?? "")][0] ?? "")
    .join("")
    .toUpperCase();
}

/**
 * A person's avatar: a rounded square (Slack's shape), the photo or initials on a calm,
 * deterministic tint, and an optional presence badge cut into the corner.
 */
export function Avatar({
  name,
  userId,
  src,
  size = 36,
  presence,
  decorative = false,
}: AvatarProps) {
  const [failed, setFailed] = useState(false);
  const hue = avatarHue(userId ?? name);
  const showImage = src !== undefined && !failed;
  const label = presence === undefined ? name : `${name} (${PRESENCE_LABEL[presence]})`;

  const style = { "--avatar-size": `${size}px`, "--avatar-hue": hue };

  const content = (
    <>
      {showImage ? (
        <img
          className="avatar-image"
          src={src}
          alt=""
          width={size}
          height={size}
          loading="lazy"
          decoding="async"
          onError={() => setFailed(true)}
        />
      ) : (
        <span className="avatar-initials" aria-hidden="true">
          {initials(name)}
        </span>
      )}
      {presence === undefined ? null : (
        <span className="avatar-presence" data-presence={presence} aria-hidden="true" />
      )}
    </>
  );

  return decorative ? (
    <span className="avatar" aria-hidden="true" style={style}>
      {content}
    </span>
  ) : (
    <span className="avatar" role="img" aria-label={label} style={style}>
      {content}
    </span>
  );
}
