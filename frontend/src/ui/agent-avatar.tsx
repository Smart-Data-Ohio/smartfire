import { useEffect, useState } from "react";
import { useResolvedTheme } from "../lib/appearance.ts";
import { Icon } from "./icons/icon.tsx";
import "./avatar.css";
import "./effects.css";

interface AgentAvatarProps {
  /** Any stable id (the agent's user id): the same seed always draws the same bot. */
  readonly seed: string;
  readonly name: string;
  readonly size?: number;
  readonly decorative?: boolean;
}

/**
 * An agent's avatar: one of Jakub Antalik's bot-avatars, picked deterministically from the seed,
 * drawn once to a bitmap and cached (no live canvas per message row). Agents sit on the violet
 * tile that people never get.
 */
export function AgentAvatar({ seed, name, size = 36, decorative = false }: AgentAvatarProps) {
  const theme = useResolvedTheme();
  const [bitmap, setBitmap] = useState<{ key: string; src: string } | null>(null);
  const key = `${seed}|${size}|${theme}`;

  useEffect(() => {
    let current = true;

    import("./bot-bitmap.ts")
      .then(({ botBitmap }) => botBitmap(seed, size, theme))
      .then((src) => {
        if (current && src !== "") {
          setBitmap({ key: `${seed}|${size}|${theme}`, src });
        }
      })
      .catch(() => {
        // No canvas (or the chunk failed): the bot glyph stays.
      });

    return () => {
      current = false;
    };
  }, [seed, size, theme]);

  const src = bitmap?.key === key ? bitmap.src : null;

  const style = { "--avatar-size": `${size}px` };

  const content =
    src === null ? (
      <Icon name="bot" size={Math.round(size * 0.6)} />
    ) : (
      <img className="avatar-image" src={src} alt="" width={size} height={size} />
    );

  return decorative ? (
    <span className="avatar agent-avatar" aria-hidden="true" style={style}>
      {content}
    </span>
  ) : (
    <span className="avatar agent-avatar" role="img" aria-label={`${name} (agent)`} style={style}>
      {content}
    </span>
  );
}
