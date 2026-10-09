import { describe, expect, it } from "vitest";
import { planEdit } from "../messages/message-editor.tsx";
import {
  attachDriveFile,
  DRIVE_SEARCH_DEBOUNCE_MS,
  driveDisconnected,
  driveSearchStatus,
  grantEnabled,
  moveActive,
  pickerKeyAction,
  searchDelay,
} from "./drive-picker.ts";

const file = (id: string, kind = "document") => ({
  id,
  name: id,
  kind,
  url: `https://drive.google.com/open?id=${id}`,
});

describe("drive picker state", () => {
  it("searches immediately, then debounces", () => {
    expect(searchDelay(false)).toBe(0);
    expect(searchDelay(true)).toBe(DRIVE_SEARCH_DEBOUNCE_MS);
    expect(DRIVE_SEARCH_DEBOUNCE_MS).toBe(300);
  });

  it("moves the highlight and wraps", () => {
    expect(moveActive(-1, 1, 3)).toBe(0);
    expect(moveActive(-1, -1, 3)).toBe(2);
    expect(moveActive(2, 1, 3)).toBe(0);
    expect(moveActive(0, -1, 3)).toBe(2);
    expect(moveActive(0, 1, 0)).toBe(-1);
  });

  it("maps the classic keys", () => {
    expect(pickerKeyAction("Escape")).toBe("close");
    expect(pickerKeyAction("ArrowDown")).toBe("next");
    expect(pickerKeyAction("ArrowUp")).toBe("previous");
    expect(pickerKeyAction("Enter")).toBe("choose");
    expect(pickerKeyAction("a")).toBeNull();
  });

  it("attaches once, up to ten", () => {
    const first = attachDriveFile([], file("a"));

    expect(first.status).toBe("attached");
    expect(attachDriveFile(first.files, file("a")).status).toBe("duplicate");

    const ten = Array.from({ length: 10 }, (_, index) => file(String(index)));

    expect(attachDriveFile(ten, file("more")).status).toBe("full");
  });

  it("grants only when someone is checked and the file is not a folder", () => {
    expect(grantEnabled(0, "document")).toBe(false);
    expect(grantEnabled(1, "document")).toBe(true);
    expect(grantEnabled(2, "folder")).toBe(false);
  });

  it("treats a 404 as not connected and keeps the classic status lines", () => {
    expect(driveDisconnected({ tag: "NotFound", message: "Not found" })).toBe(true);
    expect(driveDisconnected({ tag: "ServerError", message: "The server answered 404" })).toBe(
      true,
    );
    expect(driveSearchStatus(null, 0)).toBe("No files found");
    expect(driveSearchStatus(null, 2)).toBe("");
    expect(driveSearchStatus({ tag: "ServerError", message: "The server answered 429" }, 0)).toBe(
      "Try again in a moment",
    );
    expect(driveSearchStatus({ tag: "ServerError", message: "The server answered 502" }, 0)).toBe(
      "Drive is unavailable right now",
    );
    expect(driveSearchStatus({ tag: "NotFound", message: "Not found" }, 0)).toBe("");
  });
});

describe("editor removal payload", () => {
  it("closes when nothing changed, deletes an empty message, and sends the removed ids", () => {
    expect(
      planEdit({
        markdown: "Same",
        original: "Same",
        removedIds: [],
        remainingDrive: 1,
        hasAttachment: false,
      }),
    ).toEqual({ kind: "close" });

    expect(
      planEdit({
        markdown: "Same",
        original: "Same",
        removedIds: ["1AbcDefGhIjKlMnOpQrSt"],
        remainingDrive: 0,
        hasAttachment: false,
      }),
    ).toEqual({
      kind: "save",
      body: {
        markdownSource: "Same",
        removeDriveFileIds: ["1AbcDefGhIjKlMnOpQrSt"],
      },
    });

    expect(
      planEdit({
        markdown: "   ",
        original: "Hi",
        removedIds: [],
        remainingDrive: 0,
        hasAttachment: false,
      }),
    ).toEqual({ kind: "delete" });

    expect(
      planEdit({
        markdown: "",
        original: "Hi",
        removedIds: ["abc"],
        remainingDrive: 1,
        hasAttachment: false,
      }),
    ).toEqual({
      kind: "save",
      body: { markdownSource: "", removeDriveFileIds: ["abc"] },
    });
  });
});
