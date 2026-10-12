import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import type { Attachment } from "../../gen/Attachment.ts";
import type { AvailableSticker, PendingMessage } from "../../store/model.ts";
import { PendingRow } from "../room/message-row.tsx";
import { MessageContent, snippet } from "./message-content.tsx";
import { stickerText, wordlessSummary } from "./message-files.ts";
import { STICKER_SIZE, StickerMessage } from "./sticker-message.tsx";

const DANCE: AvailableSticker = {
  kind: "available",
  id: 2,
  name: "Dance",
  imageUrl: "/stickers/2",
  stillUrl: "/stickers/2?still=1",
  animated: true,
};

const WAVE: AvailableSticker = {
  kind: "available",
  id: 1,
  name: "Wave",
  imageUrl: "/stickers/1",
  stillUrl: "/stickers/1",
  animated: false,
};

const file: Attachment = {
  filename: "notes.txt",
  contentType: "text/plain",
  byteSize: 5,
  width: null,
  height: null,
  preview: "file",
  url: "/files/notes",
  downloadUrl: "/files/notes?disposition=attachment",
  thumbnailUrl: null,
};

/** A message carrying `sticker`, with `markdown` as its caption. */
function withSticker(sticker: AvailableSticker | { kind: "deleted"; name: string }, markdown = "") {
  return messageFixture(1, 12, {
    markdownSource: markdown,
    bodyHtml: markdown === "" ? "" : `<p>${markdown}</p>`,
    sticker,
  });
}

beforeEach(() => {
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );
});

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.motion;
  vi.unstubAllGlobals();
});

describe("a message's sticker", () => {
  it("shows at a fixed size, named for screen readers", () => {
    render(<StickerMessage sticker={WAVE} />);

    const image = screen.getByRole("img", { name: "Sticker: Wave" });

    expect(image.getAttribute("src")).toBe("/stickers/1");
    expect(image.getAttribute("width")).toBe(String(STICKER_SIZE));
    expect(image.getAttribute("height")).toBe(String(STICKER_SIZE));
  });

  it("plays an animated sticker, and shows its still under reduced motion", async () => {
    render(<StickerMessage sticker={DANCE} />);

    const image = screen.getByRole("img", { name: "Sticker: Dance" });

    expect(image.getAttribute("src")).toBe("/stickers/2");

    document.documentElement.dataset.motion = "reduce";

    await waitFor(() => expect(image.getAttribute("src")).toBe("/stickers/2?still=1"));
  });

  it("follows the OS's reduced motion too", () => {
    vi.stubGlobal("matchMedia", (media: string) =>
      Object.assign(new EventTarget(), {
        media,
        matches: media === "(prefers-reduced-motion: reduce)",
        onchange: null,
      }),
    );
    render(<StickerMessage sticker={DANCE} />);

    expect(screen.getByRole("img").getAttribute("src")).toBe("/stickers/2?still=1");
  });

  it("leaves a quiet placeholder with its name once deleted", () => {
    const { container } = render(<StickerMessage sticker={{ kind: "deleted", name: "Dance" }} />);

    expect(container.textContent).toBe("Dance · Sticker deleted");
    expect(container.querySelector("img")).toBeNull();
  });
});

describe("a sticker in a message", () => {
  it("shows alone, with no empty text above it", () => {
    const { container } = render(<MessageContent message={withSticker(WAVE)} />);

    expect(screen.getByRole("img", { name: "Sticker: Wave" })).toBeTruthy();
    expect(container.querySelector(".message-body")).toBeNull();
  });

  it("shows under its caption", () => {
    const { container } = render(<MessageContent message={withSticker(WAVE, "hi all")} />);

    const body = container.querySelector(".message-sticker-body");

    expect(body?.firstElementChild?.textContent).toBe("hi all");
    expect(body?.lastElementChild?.getAttribute("alt")).toBe("Sticker: Wave");
  });

  it("is shown while it sends", () => {
    const pending: PendingMessage = {
      clientMessageId: "pending-1",
      roomId: 12,
      threadId: null,
      attachmentSignedId: null,
      attachment: null,
      replyToMessageId: null,
      replyNotifyAuthor: null,
      creatorId: 7,
      markdownSource: "",
      createdAt: "2026-10-12T09:00:00.000Z",
      state: "sending",
      error: null,
      sticker: DANCE,
    };

    render(<PendingRow pending={pending} groupStart={false} />);

    expect(screen.getByRole("img", { name: "Sticker: Dance" })).toBeTruthy();
  });
});

describe("a sticker message as text", () => {
  it("reads as the server writes it, kept after the sticker is deleted", () => {
    expect(stickerText("Dance")).toBe("Sticker: Dance");
    expect(snippet(withSticker(DANCE))).toBe("Sticker: Dance");
    expect(snippet(withSticker({ kind: "deleted", name: "Dance" }))).toBe("Sticker: Dance");
    expect(snippet(withSticker(DANCE, "party time"))).toBe("party time");
  });

  it("names files first, as the server's text does", () => {
    expect(wordlessSummary({ attachment: file, sticker: DANCE })).toBe("notes.txt");
    expect(wordlessSummary({ attachment: null })).toBe("Attachment");
  });
});
