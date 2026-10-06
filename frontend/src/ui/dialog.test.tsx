import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { Button } from "./button.tsx";
import { Dialog } from "./dialog.tsx";

function RenameRoom({ role }: { readonly role?: "dialog" | "alertdialog" }) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button onClick={() => setOpen(true)}>Rename room</Button>
      <Button>Outside</Button>
      <Dialog
        open={open}
        onOpenChange={setOpen}
        title="Rename room"
        role={role ?? "dialog"}
        footer={
          <>
            <Button variant="secondary" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button>Save</Button>
          </>
        }
      >
        <label>
          Room name
          <input defaultValue="general" />
        </label>
      </Dialog>
    </>
  );
}

async function openDialog(role?: "dialog" | "alertdialog") {
  const user = userEvent.setup();

  render(<RenameRoom role={role ?? "dialog"} />);
  await user.click(screen.getByRole("button", { name: "Rename room" }));

  return user;
}

describe("Dialog", () => {
  it("opens labelled by its title, with focus on the first field", async () => {
    await openDialog();

    expect(screen.getByRole("dialog", { name: "Rename room" })).toBeTruthy();
    expect(document.activeElement).toBe(screen.getByRole("textbox", { name: "Room name" }));
  });

  it("keeps Tab and Shift+Tab inside the dialog", async () => {
    const user = await openDialog();

    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Cancel" }));

    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Save" }));

    // Past the last control, focus wraps to the first (the close button) rather than the page.
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Close" }));

    await user.tab({ shift: true });
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Save" }));
  });

  it("closes on Escape and gives focus back to the opener", async () => {
    const user = await openDialog();

    await user.keyboard("{Escape}");

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Rename room" }));
  });

  it("closes from the close button", async () => {
    const user = await openDialog();

    await user.click(screen.getByRole("button", { name: "Close" }));

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("renders an alertdialog without a close button, still closable with Escape", async () => {
    const user = await openDialog("alertdialog");

    expect(screen.getByRole("alertdialog", { name: "Rename room" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Close" })).toBeNull();

    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  });
});
