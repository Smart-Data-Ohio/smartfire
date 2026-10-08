import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { SHEET_QUERY } from "./action-sheet.tsx";
import { Button } from "./button.tsx";
import { Popover } from "./popover.tsx";

function SearchPopover() {
  return (
    <Popover label="Insert emoji" trigger={(props) => <Button {...props}>Emoji</Button>}>
      <input aria-label="Search emoji" data-autofocus />
    </Popover>
  );
}

describe("Popover", () => {
  afterEach(() => {
    Reflect.deleteProperty(window, "matchMedia");
  });

  it("opens beside its trigger with focus in its first field", async () => {
    const user = userEvent.setup();

    render(<SearchPopover />);
    await user.click(screen.getByRole("button", { name: "Emoji" }));

    const popover = screen.getByRole("dialog", { name: "Insert emoji" });

    expect(popover.classList.contains("floating")).toBe(true);
    expect(document.activeElement).toBe(screen.getByRole("textbox", { name: "Search emoji" }));
  });

  it("opens as a sheet on a touch phone, leaving its text field for a tap", async () => {
    // jsdom has no matchMedia: this one answers the sheet query, and only that, yes.
    window.matchMedia = (query: string) =>
      Object.assign(new EventTarget(), {
        matches: query === SHEET_QUERY,
        media: query,
        onchange: null,
        addListener: () => undefined,
        removeListener: () => undefined,
      });

    const user = userEvent.setup();

    render(<SearchPopover />);
    await user.click(screen.getByRole("button", { name: "Emoji" }));

    const popover = screen.getByRole("dialog", { name: "Insert emoji" });

    expect(popover.classList.contains("action-sheet")).toBe(true);
    expect(popover.dataset.placement).toBeUndefined();
    expect(document.activeElement).toBe(popover);
  });
});
