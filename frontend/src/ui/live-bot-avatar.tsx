import BotAvatar from "bot-avatars";
import type { BotAvatarSpec } from "../gen/BotAvatarSpec.ts";
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
 * A picked avatar drawn live by the library: the details pane, the profile and the picker's
 * preview. Lists draw the cached bitmap instead. Loaded on demand with the library's chunk.
 */
export default function LiveBotAvatar({
  spec,
  size,
  theme,
  state = "default",
  interactive = false,
}: LiveBotAvatarProps) {
  const wear = wearOf(spec.accessory);

  return (
    <BotAvatar
      className="live-bot-avatar"
      type={spec.type}
      color={colorOf(spec)}
      face={spec.face}
      state={state}
      size={size}
      theme={theme}
      interactive={interactive}
      hat={wear.hat}
      glasses={wear.glasses}
      headphones={wear.headphones}
      bowTie={wear.bowTie}
      aria-hidden="true"
    />
  );
}
