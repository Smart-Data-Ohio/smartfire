import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Button } from "./button.tsx";
import { IconButton } from "./icon-button.tsx";
import { Tooltip } from "./tooltip.tsx";

function tooltip(): HTMLElement | null {
  // The bubble is aria-hidden: assistive tech gets the same text from the trigger itself.
  return screen.queryByRole("tooltip", { hidden: true });
}

describe("Tooltip", () => {
  it("shows on keyboard focus and hides on blur", async () => {
    const user = userEvent.setup();

    render(
      <>
        <Tooltip content="Room settings">
          <Button>Settings</Button>
        </Tooltip>
        <Button>Next</Button>
      </>,
    );

    await user.tab();
    await waitFor(() => expect(tooltip()?.textContent).toBe("Room settings"));

    await user.tab();
    await waitFor(() => expect(tooltip()).toBeNull());
  });

  it("describes its trigger", () => {
    render(
      <Tooltip content="Room settings">
        <Button>Settings</Button>
      </Tooltip>,
    );

    const describedBy = screen.getByRole("button").getAttribute("aria-describedby") ?? "";

    expect(document.getElementById(describedBy)?.textContent).toBe("Room settings");
  });

  it("hides on Escape", async () => {
    const user = userEvent.setup();

    render(
      <Tooltip content="Room settings">
        <Button>Settings</Button>
      </Tooltip>,
    );

    await user.tab();
    await waitFor(() => expect(tooltip()).not.toBeNull());

    await user.keyboard("{Escape}");
    await waitFor(() => expect(tooltip()).toBeNull());
  });

  it("names an icon button and shows its shortcut on focus", async () => {
    const user = userEvent.setup();

    render(<IconButton icon="search" label="Search" shortcut={["⌘", "K"]} />);

    const button = screen.getByRole("button", { name: "Search" });

    expect(button.getAttribute("aria-keyshortcuts")).toBe("Meta+K");
    // The label is already the name, so it isn't repeated as a description.
    expect(button.hasAttribute("aria-describedby")).toBe(false);

    await user.tab();
    await waitFor(() => expect(tooltip()?.textContent).toContain("Search"));
    expect(tooltip()?.querySelector("kbd")).not.toBeNull();
  });
});
