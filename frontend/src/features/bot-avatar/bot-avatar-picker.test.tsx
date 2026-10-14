import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AvatarChoice } from "./avatar-choice.ts";
import { BotAvatarPicker } from "./bot-avatar-picker.tsx";

const CAT = { type: "cat" as const, color: "#eaa06f", face: "eyes" as const, accessory: null };

beforeEach(() => {
  // jsdom has no matchMedia; the agent avatar asks it about the theme and reduced motion.
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

/** The picker in a form that keeps what it says, reporting every choice to `seen`. */
function Harness({
  start,
  seen,
  current = null,
}: {
  readonly start: AvatarChoice;
  readonly seen: AvatarChoice[];
  readonly current?: string | null;
}) {
  const [choice, setChoice] = useState(start);

  return (
    <BotAvatarPicker
      id="picker"
      name="Whiskers"
      value={choice}
      current={current === null ? null : <span>{current}</span>}
      onChange={(next) => {
        seen.push(next);
        setChoice(next);
      }}
    />
  );
}

const group = (name: string) => screen.getByRole("group", { name });

describe("the bot avatar picker", () => {
  it("names every body, colour, face and accessory as a radio", () => {
    render(<Harness start={{ kind: "bot", spec: CAT }} seen={[]} />);

    expect(within(group("Body")).getAllByRole("radio")).toHaveLength(18);
    expect(within(group("Body")).getByRole("radio", { name: "Cat" })).toHaveProperty(
      "checked",
      true,
    );
    expect(within(group("Colour")).getByRole("radio", { name: "Apricot" })).toHaveProperty(
      "checked",
      true,
    );
    expect(within(group("Face")).getByRole("radio", { name: "Eyes" })).toHaveProperty(
      "checked",
      true,
    );
    expect(within(group("Wearing")).getByRole("radio", { name: "Nothing" })).toHaveProperty(
      "checked",
      true,
    );
    expect(screen.getByText("Cat in its own colour")).toBeTruthy();
  });

  it("picks a body, a colour, a face and one thing to wear", () => {
    const seen: AvatarChoice[] = [];

    render(<Harness start={{ kind: "bot", spec: CAT }} seen={seen} />);
    fireEvent.click(screen.getByRole("radio", { name: "Ghost" }));
    fireEvent.click(screen.getByRole("radio", { name: "Charcoal" }));
    fireEvent.click(screen.getByRole("radio", { name: "Eyes and mouth" }));
    fireEvent.click(screen.getByRole("radio", { name: "Crown" }));
    fireEvent.click(screen.getByRole("radio", { name: "Shades" }));

    expect(seen.at(-1)).toEqual({
      kind: "bot",
      spec: { type: "ghost", color: "#3a3a44", face: "mouth", accessory: "shades" },
    });
    expect(screen.getByText("Ghost in Charcoal, shades")).toBeTruthy();
  });

  it("goes to an image and back without losing the picked avatar", () => {
    const seen: AvatarChoice[] = [];

    render(<Harness start={{ kind: "bot", spec: CAT }} seen={seen} />);
    fireEvent.click(screen.getByRole("radio", { name: "Upload an image" }));
    expect(seen.at(-1)).toEqual({ kind: "upload", file: null });
    expect(screen.queryByRole("group", { name: "Body" })).toBeNull();
    expect(screen.getByLabelText("Image")).toBeTruthy();

    fireEvent.click(screen.getByRole("radio", { name: "Bot avatar" }));
    expect(seen.at(-1)).toEqual({ kind: "bot", spec: CAT });
  });

  it("offers to keep an edited bot's own picture, and only then", () => {
    const seen: AvatarChoice[] = [];

    const { unmount } = render(
      <Harness start={{ kind: "keep" }} seen={seen} current="the old icon" />,
    );

    expect(screen.getByRole("radio", { name: "Keep current" })).toHaveProperty("checked", true);
    expect(screen.getByText("the old icon")).toBeTruthy();

    fireEvent.click(screen.getByRole("radio", { name: "Bot avatar" }));
    expect(seen.at(-1)?.kind).toBe("bot");
    unmount();

    render(<Harness start={{ kind: "bot", spec: CAT }} seen={[]} />);
    expect(screen.queryByRole("radio", { name: "Keep current" })).toBeNull();
  });

  it("shuffles to another avatar", () => {
    const seen: AvatarChoice[] = [];

    vi.spyOn(Math, "random").mockReturnValue(0);
    render(<Harness start={{ kind: "bot", spec: CAT }} seen={seen} />);
    fireEvent.click(screen.getByRole("button", { name: "Shuffle" }));

    expect(seen.at(-1)).toEqual({
      kind: "bot",
      spec: { type: "clover", color: "#41c4ff", face: "eyes", accessory: "beret" },
    });
  });
});
