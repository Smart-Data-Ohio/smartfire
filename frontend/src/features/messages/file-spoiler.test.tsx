import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import type { Attachment } from "../../gen/Attachment.ts";
import { AttachmentGallery, AttachmentView, PendingAttachmentView } from "./attachments.tsx";
import { MessageContent } from "./message-content.tsx";

const file = (name: string, preview: Attachment["preview"], spoiler = true): Attachment => ({
  filename: name,
  contentType: preview === "file" ? "audio/ogg" : `${preview}/png`,
  byteSize: 100,
  width: 800,
  height: 600,
  preview,
  spoiler,
  url: `/files/${name}`,
  downloadUrl: `/files/${name}?disposition=attachment`,
  thumbnailUrl: preview === "file" ? null : `/thumbs/${name}`,
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

beforeEach(() =>
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { matches: true, media, onchange: null }),
  ),
);

describe("file spoilers", () => {
  it.each(["image", "video", "file"] as const)(
    "mounts no %s media until the cover is activated",
    async (preview) => {
      const selectors = { image: "img", video: "video", file: "audio" };
      const { container } = render(<AttachmentView attachment={file("secret", preview)} />);
      expect(container.querySelector("img, video, audio, a")).toBeNull();
      const cover = screen.getByRole("button", { name: "Spoiler, activate to reveal" });
      cover.focus();
      await userEvent.setup().keyboard("{Enter}");
      expect(container.querySelector(selectors[preview])?.getAttribute("src")).toBe(
        preview === "image" ? "/thumbs/secret" : "/files/secret",
      );
      expect(screen.queryByRole("button", { name: "Spoiler, activate to reveal" })).toBeNull();
    },
  );

  it("excludes covered images from lightbox navigation and reveals only the clicked file", async () => {
    const files = [
      file("public.png", "image", false),
      file("secret.png", "image"),
      file("other.png", "image"),
    ];

    const { container } = render(<AttachmentGallery attachments={files} />);
    expect(container.querySelectorAll("img")).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "Open public.png" }));
    const dialog = await screen.findByRole("dialog", { name: "public.png" });
    expect(within(dialog).queryByRole("button", { name: "Next image" })).toBeNull();
    expect(container.querySelector('img[src="/files/secret.png"]')).toBeNull();
    fireEvent.click(within(dialog).getByRole("button", { name: "Close" }));
    fireEvent.click(
      screen.getAllByRole("button", { name: "Spoiler, activate to reveal" })[0] ?? document.body,
    );
    expect(screen.getAllByRole("button", { name: "Spoiler, activate to reveal" })).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "Open secret.png" }));
    expect(await screen.findByRole("dialog", { name: "secret.png" })).toBeTruthy();
    expect(screen.getByText(/^2 of 2/)).toBeTruthy();
  });

  it("resets reveal when a view changes messages and keeps separate views independent", () => {
    const attachment = file("secret.png", "image");
    const message = messageFixture(1, 12, { attachment });

    const { rerender, container } = render(
      <>
        <MessageContent message={message} />
        <MessageContent message={message} />
      </>,
    );

    fireEvent.click(
      screen.getAllByRole("button", { name: "Spoiler, activate to reveal" })[0] ?? document.body,
    );
    expect(container.querySelectorAll("img")).toHaveLength(1);
    expect(screen.getAllByRole("button", { name: "Spoiler, activate to reveal" })).toHaveLength(1);
    rerender(<MessageContent message={{ ...message, id: 2 }} />);
    expect(container.querySelector("img")).toBeNull();
  });

  it("covers optimistic local previews too", () => {
    const { container } = render(
      <PendingAttachmentView
        attachment={{
          filename: "secret.png",
          contentType: "image/png",
          byteSize: 100,
          previewUrl: "blob:secret",
          spoiler: true,
        }}
      />,
    );

    expect(container.querySelector("img")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Spoiler, activate to reveal" }));
    expect(container.querySelector("img")?.getAttribute("src")).toBe("blob:secret");
  });
});
