import { type ReactNode, Suspense, useEffect, useState } from "react";
import type { BotAvatarSpec } from "../gen/BotAvatarSpec.ts";
import type { Icon as IconDTO } from "../gen/Icon.ts";
import { useResolvedTheme } from "../lib/appearance.ts";
import { EmojiImage } from "../lib/emoji/emoji-image.tsx";
import { lazyForUpdate } from "../service-worker/lazy.ts";
import { loadForUpdate } from "../service-worker/update-required.ts";
import { AgentThinking } from "./agent-thinking.tsx";
import type { BotPose } from "./bot-bitmap.ts";
import { Icon } from "./icons/icon.tsx";
import "./avatar.css";
import "./effects.css";

interface AgentAvatarProps {
  /** Any stable id (the agent's user id): the same seed always draws the same bot. */
  readonly seed: string;
  readonly name: string;
  readonly size?: number;
  readonly decorative?: boolean;
  /** The bot's picked bot avatar (`User.botAvatar`): drawn ahead of anything else. */
  readonly spec?: BotAvatarSpec | null;
  /** The bot's chosen icon (`User.avatarIcon`): drawn on the violet tile instead of a generated bot. */
  readonly icon?: IconDTO | null;
  /** The bot's uploaded picture (`User.avatarUrl` when `hasAvatar`), shown when it has no icon. */
  readonly picture?: string | null;
  /**
   * Draw a picked avatar live (it breathes, works and sleeps) rather than as a still bitmap: the
   * details pane, profiles and the picker. Lists stay still.
   */
  readonly live?: boolean;
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

const LiveBotAvatar = lazyForUpdate(() => import("./live-bot-avatar.tsx"));

/**
 * An agent's avatar, on the violet tile that people never get: the bot avatar its creator picked,
 * else its icon, else its uploaded picture, else one of Jakub Antalik's bot-avatars picked
 * deterministically from the seed. Bots are drawn once to a cached bitmap (no live canvas per
 * message row) unless `live` asks for the animated one.
 */
export function AgentAvatar({
  seed,
  name,
  size = 36,
  decorative = false,
  spec = null,
  icon = null,
  picture = null,
  live = false,
  state,
  stateLabel,
}: AgentAvatarProps) {
  const theme = useResolvedTheme();

  const chosen =
    spec === null && icon !== null && (icon.character !== null || icon.imageUrl !== null)
      ? icon
      : null;

  const uploaded = spec === null && chosen === null ? picture : null;
  const pose: BotPose = state === "muted" ? "sleeping" : "default";
  const drawn = chosen === null && uploaded === null;
  const specKey = spec === null ? "" : `${spec.type}|${spec.color}|${spec.face}|${spec.accessory}`;
  const key = `${seed}|${specKey}|${size}|${theme}|${pose}`;
  const src = useBotBitmap(drawn, key, seed, spec, size, theme, pose);

  const style = { "--avatar-size": `${size}px` };

  let body: ReactNode;

  if (chosen !== null) {
    body = <ChosenIcon icon={chosen} size={size} />;
  } else if (uploaded !== null) {
    body = <img className="avatar-image" src={uploaded} alt="" width={size} height={size} />;
  } else {
    // Until the bitmap lands, a picked avatar shows its uploaded render, others the bot glyph.
    body = <DrawnBot src={src ?? (spec === null ? null : picture)} size={size} />;
  }

  const content = (
    <>
      {body}
      {live && spec !== null ? (
        <Suspense fallback={null}>
          <span className="agent-avatar-live" aria-hidden="true">
            <LiveBotAvatar
              spec={spec}
              size={Math.round(size * 0.8)}
              theme={theme}
              state={liveState(state)}
              interactive={!decorative}
            />
          </span>
        </Suspense>
      ) : null}
      <Corner state={state} />
    </>
  );

  const label = stateLabel === undefined ? `${name} (agent)` : `${name} (agent, ${stateLabel})`;

  return decorative ? (
    <span
      className="avatar agent-avatar"
      data-agent-state={state}
      data-bot-avatar={spec?.type}
      aria-hidden="true"
      style={style}
    >
      {content}
    </span>
  ) : (
    <span
      className="avatar agent-avatar"
      data-agent-state={state}
      data-bot-avatar={spec?.type}
      role="img"
      aria-label={label}
      style={style}
    >
      {content}
    </span>
  );
}

/** What a live avatar acts out: asleep while it can't act, busy while it works. */
function liveState(state: AgentAvatarState | undefined): "default" | "working" | "sleeping" {
  switch (state) {
    case "muted":
      return "sleeping";
    case "working":
      return "working";
    default:
      return "default";
  }
}

/** The bot's still bitmap once drawn (lazily, with the library's chunk); `null` until then. */
function useBotBitmap(
  drawn: boolean,
  key: string,
  seed: string,
  spec: BotAvatarSpec | null,
  size: number,
  theme: "light" | "dark",
  pose: BotPose,
): string | null {
  const [bitmap, setBitmap] = useState<{ key: string; src: string } | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `key` covers seed, spec, size, theme and pose
  useEffect(() => {
    if (!drawn) {
      return;
    }

    let current = true;

    loadForUpdate(() => import("./bot-bitmap.ts"))
      .then(({ botBitmap, specBitmap }) =>
        spec === null ? botBitmap(seed, size, theme, pose) : specBitmap(spec, size, theme, pose),
      )
      .then((src) => {
        if (current && src !== "") {
          setBitmap({ key, src });
        }
      })
      .catch(() => {
        // No canvas (or the chunk failed): the bot glyph (or the uploaded render) stays.
      });

    return () => {
      current = false;
    };
  }, [drawn, key]);

  return bitmap?.key === key ? bitmap.src : null;
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
    <EmojiImage
      className="agent-avatar-icon"
      src={icon.imageUrl}
      still={icon.stillUrl}
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
