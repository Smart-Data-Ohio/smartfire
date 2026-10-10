import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import type { Attachment } from "../../gen/Attachment.ts";
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
