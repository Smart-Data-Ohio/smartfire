import { afterEach, describe, expect, it, vi } from "vitest";
import { preparePicker } from "./google-picker.ts";
import { pickerFiles } from "./s2/drive.ts";

const config = {
  clientId: "mock-client",
  apiKey: "mock-key",
  projectNumber: "123456",
  accountEmail: "maya@37signals.com",
};

const showModal = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, "showModal");

afterEach(() => {
  vi.restoreAllMocks();

  if (showModal) Object.defineProperty(HTMLDialogElement.prototype, "showModal", showModal);
  else Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal");

  for (const dialog of document.querySelectorAll("dialog")) dialog.remove();
});

describe("mock Picker account identity", () => {
  it("opens the file dialog for the connected account", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(Response.json(pickerFiles()));
    Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.open = true;
      },
    });
    const session = await preparePicker(config);
    const selection = session.choose();

    const file = [...document.querySelectorAll("dialog button")].find(
      (button) => button.textContent === "Existing private plan",
    );

    expect(file).toBeDefined();
    file?.dispatchEvent(new MouseEvent("click"));
    expect(await selection).toMatchObject({ id: "3ExistingDriveFile" });
    expect(document.querySelector("dialog")).toBeNull();
    session.dispose();
  });

  it("rejects a different account before showing any files", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      Response.json({ ...pickerFiles(), accountEmail: "other@example.com" }),
    );
    const session = await preparePicker(config);

    await expect(session.choose()).rejects.toThrow(
      "Pick files from maya@37signals.com, the Google account connected to Smartfire",
    );
    expect(document.querySelector("dialog")).toBeNull();
    session.dispose();
  });
});
