import { useState } from "react";
import "./avatar.css";

export type PresenceStatus = "online" | "away" | "dnd" | "offline";

interface AvatarProps {
  readonly name: string;
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

/**
 * Hues for initials tiles. Violet (roughly 280-320) is left out on purpose: it belongs to agents,
 * so a person's tile can never be mistaken for one.
 */
const HUES = [25, 50, 75, 110, 150, 185, 210, 235, 255, 345] as const;

function hashString(value: string): number {
  let hash = 2166136261;

  for (const character of value) {
    hash ^= character.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16777619);
  }

  return hash >>> 0;
}

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
export function Avatar({ name, src, size = 36, presence, decorative = false }: AvatarProps) {
  const [failed, setFailed] = useState(false);
  const hue = HUES[hashString(name) % HUES.length] ?? HUES[0];
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
