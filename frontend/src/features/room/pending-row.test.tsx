import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { PendingAttachment, PendingMessage } from "../../store/model.ts";
import { PendingRow } from "./message-row.tsx";

function pending(markdownSource: string, attachment: PendingAttachment | null): PendingMessage {
  return {
    clientMessageId: "pending-1",
    roomId: 1,
    threadId: null,
    attachmentSignedId: attachment === null ? null : "signed-1",
    attachment,
    replyToMessageId: null,
    replyNotifyAuthor: null,
    creatorId: 7,
    markdownSource,
    createdAt: "2026-10-06T09:00:00.000Z",
    state: "sending",
    error: null,
  };
}

describe("a pending row", () => {
  it("shows a file-only send's file, with no empty text", () => {
    const { container } = render(
      <PendingRow
        pending={pending("", {
          filename: "roadmap.pdf",
          contentType: "application/pdf",
          byteSize: 48_213,
          previewUrl: null,
        })}
        groupStart={false}
      />,
    );

    expect(screen.getByText("roadmap.pdf")).toBeTruthy();
    expect(container.querySelector("a, button:not([disabled])")).toBeNull();
  });

  it("previews an image from its local URL", () => {
    render(
      <PendingRow
        pending={pending("look", {
          filename: "shot.png",
          contentType: "image/png",
          byteSize: 1024,
          previewUrl: "blob:http://localhost/shot",
        })}
        groupStart={false}
      />,
    );

    expect(screen.getByRole("img", { name: "shot.png" }).getAttribute("src")).toBe(
      "blob:http://localhost/shot",
    );
    expect(screen.getByText("look")).toBeTruthy();
  });
});
