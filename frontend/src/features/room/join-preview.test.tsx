import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { OpenRoomPreview } from "../../gen/OpenRoomPreview.ts";
import { JoinPreview, membersLabel } from "./join-preview.tsx";

const campfire: OpenRoomPreview = { id: 90, name: "campfire", memberCount: 3 };

describe("join preview", () => {
  it("shows the channel, that you aren't a member, and the member count", () => {
    render(
      <JoinPreview preview={campfire} joining={false} joinError={null} onJoin={() => undefined} />,
    );

    expect(screen.getByRole("heading", { name: "#campfire" }).textContent).toBe("#campfire");
    expect(screen.getByText("You're not a member of this channel.")).toBeTruthy();
    expect(screen.getByText("3 members")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Join channel" })).toBeTruthy();
  });

  it("joins when the button is pressed, and says why a join failed", async () => {
    const user = userEvent.setup();
    const onJoin = vi.fn();

    const { rerender } = render(
      <JoinPreview preview={campfire} joining={false} joinError={null} onJoin={onJoin} />,
    );

    await user.click(screen.getByRole("button", { name: "Join channel" }));

    expect(onJoin).toHaveBeenCalledOnce();

    rerender(
      <JoinPreview
        preview={{ ...campfire, memberCount: 1 }}
        joining={false}
        joinError="Room not found"
        onJoin={onJoin}
      />,
    );

    expect(screen.getByText("1 member")).toBeTruthy();
    expect(screen.getByText("Room not found")).toBeTruthy();
  });

  it("ignores a click while the join is in flight", async () => {
    const user = userEvent.setup();
    const onJoin = vi.fn();

    render(<JoinPreview preview={campfire} joining onJoin={onJoin} joinError={null} />);

    await user.click(screen.getByRole("button", { name: "Join channel" }));

    expect(onJoin).not.toHaveBeenCalled();
  });

  it("labels a single member", () => {
    expect(membersLabel(1)).toBe("1 member");
    expect(membersLabel(0)).toBe("0 members");
  });
});
