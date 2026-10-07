import { useEffect, useState } from "react";
import type { Icon as IconDTO } from "../gen/Icon.ts";
import { useResolvedTheme } from "../lib/appearance.ts";
import { AgentThinking } from "./agent-thinking.tsx";
import { Icon } from "./icons/icon.tsx";
import "./avatar.css";
import "./effects.css";

interface AgentAvatarProps {
  /** Any stable id (the agent's user id): the same seed always draws the same bot. */
  readonly seed: string;
  readonly name: string;
  readonly size?: number;
  readonly decorative?: boolean;
  /** The bot's chosen icon (`User.avatarIcon`): drawn on the violet tile instead of a generated bot. */
  readonly icon?: IconDTO | null;
  /**
   * How it reads: `"muted"` (suspended or deactivated) greys the tile out; `"working"` puts the
   * thinking orbs in the corner, `"waiting"` and `"failed"` a status dot. Only lists that show
   * presence pass it; message rows never animate.
   */
  readonly state?: AgentAvatarState;
  /** What the corner says to a screen reader, e.g. "working", appended to the name. */
  readonly stateLabel?: string;
}

/** The status an agent's avatar shows, if any. */
export type AgentAvatarState = "working" | "waiting" | "failed" | "muted";

/**
 * An agent's avatar: one of Jakub Antalik's bot-avatars, picked deterministically from the seed,
 * drawn once to a bitmap and cached (no live canvas per message row). Agents sit on the violet
 * tile that people never get.
 */
export function AgentAvatar({
  seed,
  name,
  size = 36,
  decorative = false,
  icon = null,
  state,
  stateLabel,
}: AgentAvatarProps) {
  const theme = useResolvedTheme();
  const [bitmap, setBitmap] = useState<{ key: string; src: string } | null>(null);
  const key = `${seed}|${size}|${theme}`;
  const chosen = icon !== null && (icon.character !== null || icon.imageUrl !== null) ? icon : null;
  const drawn = chosen === null;

  useEffect(() => {
    if (!drawn) {
      return;
    }

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
  }, [drawn, seed, size, theme]);

  const src = bitmap?.key === key ? bitmap.src : null;

  const style = { "--avatar-size": `${size}px` };

  const content = (
    <>
      {chosen === null ? (
        <DrawnBot src={src} size={size} />
      ) : (
        <ChosenIcon icon={chosen} size={size} />
      )}
      <Corner state={state} />
    </>
  );

  const label = stateLabel === undefined ? `${name} (agent)` : `${name} (agent, ${stateLabel})`;

  return decorative ? (
    <span className="avatar agent-avatar" data-agent-state={state} aria-hidden="true" style={style}>
      {content}
    </span>
  ) : (
    <span
      className="avatar agent-avatar"
      data-agent-state={state}
      role="img"
      aria-label={label}
      style={style}
    >
      {content}
    </span>
  );
}

function DrawnBot({ src, size }: { readonly src: string | null; readonly size: number }) {
  return src === null ? (
    <Icon name="bot" size={Math.round(size * 0.6)} />
  ) : (
    <img className="avatar-image" src={src} alt="" width={size} height={size} />
  );
}

/** The bot's own icon: an emoji character, or a brand or custom image, on the violet tile. */
function ChosenIcon({ icon, size }: { readonly icon: IconDTO; readonly size: number }) {
  if (icon.character !== null) {
    return (
      <span className="agent-avatar-emoji" aria-hidden="true">
        {icon.character}
      </span>
    );
  }

  return icon.imageUrl === null ? null : (
    <img
      className="agent-avatar-icon"
      src={icon.imageUrl}
      alt=""
      width={Math.round(size * 0.62)}
      height={Math.round(size * 0.62)}
      loading="lazy"
      decoding="async"
    />
  );
}

/** The corner: thinking orbs while working, a dot while waiting or failed, nothing otherwise. */
function Corner({ state }: { readonly state: AgentAvatarState | undefined }) {
  if (state === "working") {
    return (
      <span className="agent-avatar-working" aria-hidden="true">
        <AgentThinking size={20} label="" />
      </span>
    );
  }

  return state === "waiting" || state === "failed" ? (
    <span className="avatar-presence" data-agent-state={state} aria-hidden="true" />
  ) : null;
}
