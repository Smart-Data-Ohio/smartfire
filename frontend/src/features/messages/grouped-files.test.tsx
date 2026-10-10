import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import type { Attachment } from "../../gen/Attachment.ts";
import type { PendingMessage } from "../../store/model.ts";
import { PendingRow } from "../room/message-row.tsx";
import { MessageContent } from "./message-content.tsx";

const first: Attachment = {
  filename: "first.txt",
  contentType: "text/plain",
  byteSize: 5,
  width: null,
  height: null,
  preview: "file",
  url: "/files/first",
  downloadUrl: "/files/first?disposition=attachment",
  thumbnailUrl: null,
};

const second: Attachment = { ...first, filename: "second.txt", url: "/files/second" };

describe("grouped message files", () => {
  it.each([null, "2026-10-10T00:00:00Z"])(
    "renders every file once, including forwards (%s)",
    (forwardedAt) => {
      const message = {
        ...messageFixture(1, 12, { attachment: first, forwardedAt }),
        attachments: [first, second],
      };

      render(<MessageContent message={message} />);

      expect(screen.getAllByRole("link", { name: "first.txt" })).toHaveLength(1);
      expect(screen.getByRole("link", { name: "second.txt" }).getAttribute("href")).toBe(
        "/files/second",
      );
    },
  );

  it("keeps the legacy single-file message readable", () => {
    render(<MessageContent message={messageFixture(1, 12, { attachment: first })} />);
    expect(screen.getByRole("link", { name: "first.txt" }).getAttribute("href")).toBe(
      "/files/first",
    );
  });
});

const image = (name: string): Attachment => ({
  filename: name,
  contentType: "image/png",
  byteSize: 2048,
  width: 800,
  height: 600,
  preview: "image",
  url: `/files/${name}`,
  downloadUrl: `/files/${name}?disposition=attachment`,
  thumbnailUrl: `/thumbs/${name}`,
});

describe("the grouped gallery", () => {
  beforeEach(() => {
    vi.stubGlobal("matchMedia", (query: string) =>
      Object.assign(new EventTarget(), { matches: false, media: query, onchange: null }),
    );
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  const grouped = () => ({
    ...messageFixture(1, 12, { attachment: image("one.png") }),
    attachments: [image("one.png"), image("two.png"), first, image("three.png")],
  });

  it("tiles the images in a grid and lists other files below", () => {
    const { container } = render(<MessageContent message={grouped()} />);
    const grid = screen.getByRole("list", { name: "3 images and videos" });

    expect(grid.getAttribute("data-columns")).toBe("3");
    expect(
      within(grid)
        .getAllByRole("button")
        .map((button) => button.ariaLabel),
    ).toEqual(["Open one.png", "Open two.png", "Open three.png"]);
    expect(within(grid).queryByRole("link")).toBeNull();
    expect(screen.getByRole("link", { name: "first.txt" })).toBeTruthy();
    expect(container.querySelectorAll(".attachment-gallery")).toHaveLength(1);
  });

  it("opens the lightbox at the clicked image and steps through the message's images", async () => {
    render(<MessageContent message={grouped()} />);

    fireEvent.click(screen.getByRole("button", { name: "Open two.png" }));

    const dialog = await screen.findByRole("dialog", { name: "two.png" });

    expect(within(dialog).getByText(/^2 of 3/)).toBeTruthy();

    fireEvent.click(within(dialog).getByRole("button", { name: "Next image" }));
    expect(await screen.findByRole("dialog", { name: "three.png" })).toBeTruthy();

    fireEvent.keyDown(document, { key: "ArrowRight" });
    expect(await screen.findByRole("dialog", { name: "one.png" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Previous image" }));
    expect(await screen.findByRole("dialog", { name: "three.png" })).toBeTruthy();
  });

  it("draws a legacy single image as before, with no gallery or steps", async () => {
    const { container } = render(
      <MessageContent message={messageFixture(1, 12, { attachment: image("solo.png") })} />,
    );

    expect(container.querySelector(".attachment-gallery")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Open solo.png" }));

    const dialog = await screen.findByRole("dialog", { name: "solo.png" });

    expect(within(dialog).queryByRole("button", { name: "Next image" })).toBeNull();
    expect(within(dialog).queryByText(/of 1/)).toBeNull();
  });

  it("shows a grouped send's pending files as its gallery will be", () => {
    const pending: PendingMessage = {
      clientMessageId: "pending-1",
      roomId: 12,
      threadId: null,
      attachmentSignedId: null,
      attachment: null,
      attachmentSignedIds: ["a", "b", "c"],
      attachments: [
        { filename: "a.png", contentType: "image/png", byteSize: 10, previewUrl: "blob:a.png" },
        { filename: "b.png", contentType: "image/png", byteSize: 10, previewUrl: "blob:b.png" },
        { filename: "c.txt", contentType: "text/plain", byteSize: 5, previewUrl: null },
      ],
      replyToMessageId: null,
      replyNotifyAuthor: null,
      creatorId: 7,
      markdownSource: "",
      createdAt: "2026-10-10T09:00:00.000Z",
      state: "sending",
      error: null,
    };

    render(<PendingRow pending={pending} groupStart={false} />);

    const grid = screen.getByRole("list", { name: "2 images" });

    expect(
      within(grid)
        .getAllByRole("img")
        .map((img) => img.getAttribute("src")),
    ).toEqual(["blob:a.png", "blob:b.png"]);
    expect(screen.getByText("c.txt")).toBeTruthy();
  });
});
