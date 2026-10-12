import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Sticker } from "../../gen/Sticker.ts";
import type { StickerCatalog } from "../../gen/StickerCatalog.ts";
import type { Workspace } from "../../gen/Workspace.ts";
import { admin } from "../../sync/admin.ts";
import { composerActions } from "../../sync/composer-actions.ts";
import { ActionError } from "../../sync/run.ts";
import { resetStickers, stickerStore } from "../composer/sticker-store.ts";
import { stickerCapacity, stickerFieldError, stickerLimitMessage } from "./admin-format.ts";
import { AdminContext } from "./admin-parts.tsx";
import { STICKER_FILE_TYPES } from "./sticker-file.ts";
import { StickersSection } from "./stickers-section.tsx";

const workspace: Workspace = {
  name: "Smartfire",
  logoUrl: "/logo",
  logoStillUrl: null,
  bannerUrl: null,
  bannerStillUrl: null,
  logoAttached: false,
  joinUrl: "/join/test",
  canAdminister: true,
  restrictRoomCreationToAdministrators: false,
  afkPolicy: { roomId: null, timeoutMinutes: 5 },
  uploadLimitBytes: 100 * 1024 * 1024,
  stageAudienceBudget: 300,
  version: "test",
  description: "",
  vanitySlug: null,
  vanityUrl: null,
};

function sticker(id: number, name: string, animated = false, tags = ""): Sticker {
  return {
    id,
    name,
    description: animated ? "Moves" : "",
    tags,
    imageUrl: `/stickers/${id}`,
    stillUrl: animated ? `/stickers/${id}?still=1` : `/stickers/${id}`,
    animated,
    uploaderId: 1,
    createdAt: "2026-10-12T12:00:00Z",
    updatedAt: "2026-10-12T12:00:00Z",
  };
}

const WAVE = sticker(1, "Wave", false, "👋 hello");

const DANCE = sticker(2, "Dance", true, "party");

const CATALOG: StickerCatalog = { stickers: [DANCE, WAVE], capacity: 60 };

/** A PNG's first bytes: its signature and an IHDR chunk naming `width` × `height`. */
function pngBytes(width: number, height: number): Uint8Array<ArrayBuffer> {
  const bytes = new Uint8Array(33);
  const view = new DataView(bytes.buffer);

  bytes.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  view.setUint32(8, 13);
  bytes.set([0x49, 0x48, 0x44, 0x52], 12);
  view.setUint32(16, width);
  view.setUint32(20, height);

  return bytes;
}

beforeEach(() => {
  resetStickers();
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );
});

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.motion;
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function renderSection(canAdminister = true) {
  return render(
    <AdminContext value={{ workspace: { ...workspace, canAdminister }, replace: () => {} }}>
      <StickersSection />
    </AdminContext>,
  );
}

function row(name: string): HTMLElement {
  const item = screen.getByText(name).closest("li");

  if (item === null) throw new Error(`No row for ${name}`);

  return item;
}

function choose(file: File) {
  fireEvent.change(screen.getByLabelText("Image"), { target: { files: [file] } });
}

describe("the stickers section", () => {
  it("lists each sticker with its words and counts them against the limit", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);
    renderSection();

    expect(await screen.findByText("Stickers: 2 of 60 used.")).toBeTruthy();
    expect(within(row("Dance")).getByText("Animated")).toBeTruthy();
    expect(within(row("Dance")).getByText("Moves")).toBeTruthy();
    expect(within(row("Wave")).getByText("Tags: 👋 hello")).toBeTruthy();
    expect(screen.getByLabelText("Image").getAttribute("accept")).toBe(STICKER_FILE_TYPES);
  });

  it("plays an animated sticker, and shows its still under reduced motion", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);
    renderSection();
    await screen.findByText("Dance");

    const image = () => row("Dance").querySelector("img");

    expect(image()?.getAttribute("src")).toBe("/stickers/2");

    document.documentElement.dataset.motion = "reduce";

    await waitFor(() => expect(image()?.getAttribute("src")).toBe("/stickers/2?still=1"));
  });

  it("says when there are no stickers yet", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue({ stickers: [], capacity: 60 });
    renderSection();

    expect(await screen.findByText("No stickers yet. Add one above.")).toBeTruthy();
  });

  it("is for administrators only", () => {
    const catalog = vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);

    renderSection(false);

    expect(screen.getByText("Only administrators can see this page.")).toBeTruthy();
    expect(catalog).not.toHaveBeenCalled();
  });

  it("shows a full catalog plainly and stops taking uploads", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue({ ...CATALOG, capacity: 2 });
    renderSection();

    expect(
      await screen.findByText(
        "Stickers: 2 of 2 used. The catalog is full: delete a sticker to add another.",
      ),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: "Add sticker" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByLabelText("Image").hasAttribute("disabled")).toBe(true);
  });

  it("refuses a file that isn't a sticker before it uploads", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);

    const create = vi.spyOn(admin, "createSticker");

    renderSection();
    await screen.findByText("Stickers: 2 of 60 used.");

    choose(new File(["<svg/>"], "logo.svg", { type: "image/svg+xml" }));
    expect(await screen.findByText("Image must be a PNG, APNG or GIF image.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Add sticker" }).hasAttribute("disabled")).toBe(true);

    choose(new File([pngBytes(640, 320)], "wide.png", { type: "image/png" }));
    expect(
      await screen.findByText("Image must be at most 320 × 320 pixels (this one is 640 × 320)."),
    ).toBeTruthy();

    const heavy = new File([pngBytes(64, 64)], "heavy.png", { type: "image/png" });

    Object.defineProperty(heavy, "size", { value: 512 * 1024 + 1 });
    choose(heavy);
    expect(await screen.findByText("Image must be 512 KB or smaller.")).toBeTruthy();

    choose(new File([pngBytes(320, 320)], "fine.png", { type: "image/png" }));
    await waitFor(() => expect(screen.queryByText(/^Image must/)).toBeNull());
    expect(screen.getByRole("button", { name: "Add sticker" }).hasAttribute("disabled")).toBe(
      false,
    );
    expect(create).not.toHaveBeenCalled();
  });

  it("shows the server's refusals under their fields", async () => {
    const catalog = vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);

    const create = vi.spyOn(admin, "createSticker").mockRejectedValue(
      new ActionError("Validation", "Validation failed", {
        name: ["has already been taken"],
        image: ["isn't an uploaded file"],
      }),
    );

    renderSection();
    await screen.findByText("Stickers: 2 of 60 used.");
    fireEvent.change(screen.getByRole("textbox", { name: /Name/ }), { target: { value: "Wave" } });
    fireEvent.change(screen.getByRole("textbox", { name: /Tags/ }), { target: { value: "hi" } });
    fireEvent.click(screen.getByRole("button", { name: "Add sticker" }));

    await screen.findByText("Name has already been taken.");
    expect(screen.getByText("Choose an image to upload.")).toBeTruthy();
    expect(create).toHaveBeenCalledExactlyOnceWith({
      name: "Wave",
      description: "",
      tags: "hi",
      signedId: null,
    });
    // A refusal refetches the catalog: another administrator may have changed it.
    await waitFor(() => expect(catalog).toHaveBeenCalledTimes(2));
  });

  it("says plainly when the catalog filled up meanwhile", async () => {
    const catalog = vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);

    vi.spyOn(admin, "createSticker").mockRejectedValue(
      new ActionError("StickerLimitReached", "Sticker capacity reached (limit: 60 stickers)"),
    );
    renderSection();
    await screen.findByText("Stickers: 2 of 60 used.");
    fireEvent.click(screen.getByRole("button", { name: "Add sticker" }));

    expect(await screen.findByText(stickerLimitMessage(60))).toBeTruthy();
    await waitFor(() => expect(catalog).toHaveBeenCalledTimes(2));
  });

  it("edits a sticker's words, and every picker sees the new catalog", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);

    const renamed = {
      ...CATALOG,
      stickers: [DANCE, { ...WAVE, name: "Hello", description: "Hi there", tags: "hi" }],
    };

    const update = vi.spyOn(admin, "updateSticker").mockResolvedValue(renamed);

    renderSection();
    await screen.findByText("Wave");
    fireEvent.click(screen.getByRole("button", { name: "Edit Wave" }));

    const dialog = await screen.findByRole("dialog");

    fireEvent.change(within(dialog).getByRole("textbox", { name: /Name/ }), {
      target: { value: "Hello" },
    });
    fireEvent.change(within(dialog).getByRole("textbox", { name: /Description/ }), {
      target: { value: "Hi there" },
    });
    fireEvent.change(within(dialog).getByRole("textbox", { name: /Tags/ }), {
      target: { value: "hi" },
    });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));

    await screen.findByText("Hello");
    expect(update).toHaveBeenCalledExactlyOnceWith(1, {
      name: "Hello",
      description: "Hi there",
      tags: "hi",
    });

    const held = stickerStore.getState().load;

    expect(held.status === "ready" ? held.catalog.stickers[1]?.name : null).toBe("Hello");
  });

  it("deletes a sticker after asking", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);

    const destroy = vi
      .spyOn(admin, "destroySticker")
      .mockResolvedValue({ ...CATALOG, stickers: [DANCE] });

    renderSection();
    await screen.findByText("Wave");
    fireEvent.click(screen.getByRole("button", { name: "Delete Wave" }));

    const confirm = await screen.findByRole("alertdialog");

    expect(within(confirm).getByText("Delete Wave?")).toBeTruthy();
    fireEvent.click(within(confirm).getByRole("button", { name: "Delete" }));

    await waitFor(() => expect(screen.queryByText("Wave")).toBeNull());
    expect(destroy).toHaveBeenCalledExactlyOnceWith(1);
  });
});

describe("sticker words", () => {
  it("count stickers of the limit and say when it is full", () => {
    expect(stickerCapacity(0, 60)).toBe("Stickers: 0 of 60 used.");
    expect(stickerCapacity(60, 60)).toBe(
      "Stickers: 60 of 60 used. The catalog is full: delete a sticker to add another.",
    );
  });

  it("name the field in front of the server's message", () => {
    expect(stickerFieldError("name", "must be between 2 and 30 characters")).toBe(
      "Name must be between 2 and 30 characters.",
    );
    expect(stickerFieldError("image", "must be 512 KB or smaller")).toBe(
      "Image must be 512 KB or smaller.",
    );
    expect(stickerFieldError("image", "must be attached")).toBe("Choose an image to upload.");
    expect(stickerFieldError("other", "went wrong")).toBe("went wrong");
    expect(stickerFieldError("name", undefined)).toBeUndefined();
  });
});
