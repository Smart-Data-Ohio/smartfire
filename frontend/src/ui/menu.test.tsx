import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { Button } from "./button.tsx";
import { Menu, MenuItem, MenuSeparator, SubMenu } from "./menu.tsx";

function MessageMenu({ onSelect }: { readonly onSelect: (item: string) => void }) {
  return (
    <Menu trigger={(props) => <Button {...props}>More actions</Button>}>
      <MenuItem onSelect={() => onSelect("edit")}>Edit message</MenuItem>
      <MenuItem onSelect={() => onSelect("copy")}>Copy link</MenuItem>
      <SubMenu label="Move to">
        <MenuItem onSelect={() => onSelect("general")}>General</MenuItem>
        <MenuItem onSelect={() => onSelect("random")}>Random</MenuItem>
      </SubMenu>
      <MenuItem disabled>Pin to room</MenuItem>
      <MenuSeparator />
      <MenuItem tone="danger" onSelect={() => onSelect("delete")}>
        Delete message
      </MenuItem>
    </Menu>
  );
}

async function openWithKeyboard(key: "{ArrowDown}" | "{ArrowUp}") {
  const user = userEvent.setup();
  const selected: string[] = [];

  render(<MessageMenu onSelect={(item) => selected.push(item)} />);
  await user.tab();
  await user.keyboard(key);

  return { user, selected };
}

function focusedName(): string {
  return document.activeElement?.textContent ?? "";
}

describe("Menu", () => {
  it("uses controlled open state and reports selection and trigger changes", async () => {
    const changes: boolean[] = [];

    function ControlledMenu() {
      const [open, setOpen] = useState(true);

      return (
        <Menu
          trigger={(props) => <Button {...props}>Controlled actions</Button>}
          open={open}
          onOpenChange={(next) => {
            changes.push(next);
            setOpen(next);
          }}
        >
          <MenuItem>Choose</MenuItem>
        </Menu>
      );
    }

    const user = userEvent.setup();

    render(<ControlledMenu />);
    expect(screen.getByRole("menu", { name: "Controlled actions" })).toBeTruthy();
    await user.click(screen.getByRole("menuitem", { name: "Choose" }));
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    await user.click(screen.getByRole("button", { name: "Controlled actions" }));
    expect(screen.getByRole("menu")).toBeTruthy();
    expect(changes).toEqual([false, true]);
  });

  it("opens from the trigger on ArrowDown with the first item focused", async () => {
    await openWithKeyboard("{ArrowDown}");

    expect(screen.getByRole("menu", { name: "More actions" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "More actions" }).getAttribute("aria-expanded")).toBe(
      "true",
    );
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: "Edit message" }));
  });

  it("opens on ArrowUp with the last item focused", async () => {
    await openWithKeyboard("{ArrowUp}");

    expect(focusedName()).toBe("Delete message");
  });

  it("moves with the arrow keys, wraps, and jumps with Home and End", async () => {
    const { user } = await openWithKeyboard("{ArrowDown}");

    await user.keyboard("{ArrowDown}");
    expect(focusedName()).toBe("Copy link");

    await user.keyboard("{End}");
    expect(focusedName()).toBe("Delete message");

    await user.keyboard("{ArrowDown}");
    expect(focusedName()).toBe("Edit message");

    await user.keyboard("{ArrowUp}");
    expect(focusedName()).toBe("Delete message");

    await user.keyboard("{Home}");
    expect(focusedName()).toBe("Edit message");
  });

  it("jumps to items by typing their first letters", async () => {
    const { user } = await openWithKeyboard("{ArrowDown}");

    await user.keyboard("co");
    expect(focusedName()).toBe("Copy link");

    // Letters typed within half a second build one query; after a pause a new one starts.
    await new Promise((resolve) => setTimeout(resolve, 600));
    await user.keyboard("d");
    expect(focusedName()).toBe("Delete message");
  });

  it("chooses the focused item with Enter, closes, and returns focus to the trigger", async () => {
    const { user, selected } = await openWithKeyboard("{ArrowDown}");

    await user.keyboard("{ArrowDown}{Enter}");

    expect(selected).toEqual(["copy"]);
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "More actions" }));
  });

  it("ignores a disabled item", async () => {
    const { user, selected } = await openWithKeyboard("{ArrowDown}");

    await user.keyboard("p{Enter}");

    expect(focusedName()).toBe("Pin to room");
    expect(selected).toEqual([]);
    expect(screen.getByRole("menu")).toBeTruthy();
  });

  it("closes on Escape and returns focus to the trigger", async () => {
    const { user } = await openWithKeyboard("{ArrowDown}");

    await user.keyboard("{Escape}");

    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "More actions" }));
  });

  it("opens a submenu with ArrowRight and leaves it with ArrowLeft", async () => {
    const { user, selected } = await openWithKeyboard("{ArrowDown}");

    await user.keyboard("m{ArrowRight}");

    expect(screen.getByRole("menu", { name: "Move to" })).toBeTruthy();
    expect(focusedName()).toBe("General");

    await user.keyboard("{ArrowDown}");
    expect(focusedName()).toBe("Random");

    await user.keyboard("{ArrowLeft}");
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: "Move to" }));
    await waitFor(() => expect(screen.queryByRole("menu", { name: "Move to" })).toBeNull());

    await user.keyboard("{ArrowRight}{Enter}");
    expect(selected).toEqual(["general"]);
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
  });
});
