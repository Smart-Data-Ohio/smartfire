import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { COARSE_QUERY, PHONE_QUERY } from "../lib/breakpoints.ts";
import { Button } from "./button.tsx";
import { Dialog, focusOnOpen } from "./dialog.tsx";

interface Autofocus {
  /** `data-autofocus` on the name field. */
  readonly field?: true | "always";
  /** `data-autofocus` on Cancel. */
  readonly cancel?: true;
}

function RenameRoom({
  role,
  autofocus = {},
}: {
  readonly role?: "dialog" | "alertdialog";
  readonly autofocus?: Autofocus;
}) {
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
            <Button
              variant="secondary"
              onClick={() => setOpen(false)}
              data-autofocus={autofocus.cancel}
            >
              Cancel
            </Button>
            <Button>Save</Button>
          </>
        }
      >
        <label>
          Room name
          <input defaultValue="general" data-autofocus={autofocus.field} />
        </label>
      </Dialog>
    </>
  );
}

async function openDialog(role?: "dialog" | "alertdialog", autofocus: Autofocus = {}) {
  const user = userEvent.setup();

  render(<RenameRoom role={role ?? "dialog"} autofocus={autofocus} />);
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

  it("has a grabber to swipe it away, which a confirmation lacks", async () => {
    await openDialog();

    expect(document.querySelector(".dialog-grabber")).not.toBeNull();
  });

  it("gives a confirmation no grabber", async () => {
    await openDialog("alertdialog");

    expect(document.querySelector(".dialog-grabber")).toBeNull();
  });
});

/** A touch phone: (pointer: coarse) and the phone width match. */
function stubTouch() {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: query === COARSE_QUERY || query === PHONE_QUERY,
    media: query,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  }));
}

describe("Dialog on a touch screen", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    Reflect.deleteProperty(Element.prototype, "setPointerCapture");
    Reflect.deleteProperty(Element.prototype, "hasPointerCapture");
  });

  it("focuses itself rather than raise the keyboard for its first field", async () => {
    stubTouch();
    await openDialog();

    expect(document.activeElement).toBe(screen.getByRole("dialog", { name: "Rename room" }));
  });

  it("focuses itself rather than a data-autofocus field", async () => {
    stubTouch();
    await openDialog("dialog", { field: true });

    expect(document.activeElement).toBe(screen.getByRole("dialog", { name: "Rename room" }));
  });

  it("still focuses a field marked data-autofocus=always", async () => {
    stubTouch();
    await openDialog("dialog", { field: "always" });

    expect(document.activeElement).toBe(screen.getByRole("textbox", { name: "Room name" }));
  });

  it("still focuses a confirmation's Cancel", async () => {
    stubTouch();
    await openDialog("alertdialog", { cancel: true });

    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Cancel" }));
  });

  it("leaves a late-loading field unfocused, unlike a desktop", async () => {
    const field = document.createElement("input");

    document.body.append(field);
    focusOnOpen(field);
    expect(document.activeElement).toBe(field);

    field.blur();
    stubTouch();
    focusOnOpen(field);
    expect(document.activeElement).not.toBe(field);
    field.remove();
  });

  it("springs back from a swipe whose close its owner refuses", async () => {
    stubTouch();
    // jsdom has no pointer capture (afterEach takes these off again).
    Element.prototype.setPointerCapture = () => undefined;
    Element.prototype.hasPointerCapture = () => false;

    render(
      <Dialog open onOpenChange={() => undefined} title="Sending">
        <p>Still sending</p>
      </Dialog>,
    );

    const dialog = screen.getByRole("dialog", { name: "Sending" });
    const header = screen.getByRole("heading", { name: "Sending" });
    const touch = { pointerId: 1, pointerType: "touch", isPrimary: true, clientX: 100 };

    fireEvent.pointerDown(header, { ...touch, clientY: 40 });
    fireEvent.pointerMove(header, { ...touch, clientY: 60 });
    fireEvent.pointerMove(header, { ...touch, clientY: 240 });
    expect(dialog.style.translate).toBe("0 200px");

    fireEvent.pointerUp(header, { ...touch, clientY: 240 });
    await waitFor(() => expect(dialog.style.translate).toBe(""));
    expect(dialog.hasAttribute("data-dragging")).toBe(false);
  });
});
