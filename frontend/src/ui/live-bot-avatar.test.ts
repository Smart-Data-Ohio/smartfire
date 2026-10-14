import { describe, expect, it } from "vitest";
import { liveBotAvatarProps } from "./live-bot-avatar.tsx";

const CAT = { type: "cat" as const, color: "#eaa06f", face: "mouth" as const, accessory: null };

describe("a live bot avatar", () => {
  it("moves and follows the pointer while motion is allowed", () => {
    const props = liveBotAvatarProps(
      { spec: CAT, size: 64, theme: "light", interactive: true },
      false,
    );

    expect(props).toMatchObject({
      paused: false,
      interactive: true,
      type: "cat",
      state: "default",
    });
  });

  it("holds its still pose whenever motion is reduced, in Smartfire or the OS", () => {
    const props = liveBotAvatarProps(
      { spec: CAT, size: 64, theme: "dark", state: "working", interactive: true },
      true,
    );

    expect(props).toMatchObject({ paused: true, interactive: false, state: "working" });
  });
});
