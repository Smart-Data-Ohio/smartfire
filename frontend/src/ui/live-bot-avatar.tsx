import BotAvatar, { type BotAvatarProps } from "bot-avatars";
import type { BotAvatarSpec } from "../gen/BotAvatarSpec.ts";
import { useReducedMotion } from "../motion/reduced-motion.ts";
import { colorOf, wearOf } from "./bot-avatar-spec.ts";

export interface LiveBotAvatarProps {
  readonly spec: BotAvatarSpec;
  /** The toy's size in CSS pixels (the canvas spills past it for hats and bounce). */
  readonly size: number;
  readonly theme: "light" | "dark";
  /** `working` while the agent works, `sleeping` while it can't act. */
  readonly state?: "default" | "working" | "sleeping";
  /** Turns toward the pointer and jumps when clicked. */
  readonly interactive?: boolean;
}

/**
 * The library's props for a picked avatar. `reduced` (Smartfire's "Reduce motion", or the OS's
 * when Smartfire follows it) holds it in its still pose and stops it reacting to the pointer: the
 * library itself checks only the OS setting.
 */
export function liveBotAvatarProps(
  { spec, size, theme, state = "default", interactive = false }: LiveBotAvatarProps,
  reduced: boolean,
): BotAvatarProps {
  const wear = wearOf(spec.accessory);

  return {
    className: "live-bot-avatar",
    type: spec.type,
    color: colorOf(spec),
    face: spec.face,
    state,
    size,
    theme,
    interactive: interactive && !reduced,
    paused: reduced,
    hat: wear.hat,
    glasses: wear.glasses,
    headphones: wear.headphones,
    bowTie: wear.bowTie,
    "aria-hidden": true,
  };
}

/**
 * A picked avatar drawn live by the library: the details pane, the profile and the picker's
 * preview. Lists draw the cached bitmap instead. Loaded on demand with the library's chunk.
 */
export default function LiveBotAvatar(props: LiveBotAvatarProps) {
  return <BotAvatar {...liveBotAvatarProps(props, useReducedMotion())} />;
}
