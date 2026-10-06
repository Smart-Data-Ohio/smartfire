import { describe, expect, it } from "vitest";
import type { Attachment } from "../../gen/Attachment.ts";
import type { RoomFile } from "../../gen/RoomFile.ts";
import {
  appendFiles,
  emptyFilesText,
  fileExtension,
  fileIcon,
  fileQuery,
  formatBytes,
  isFileType,
  showsAsGrid,
} from "./files.ts";

function attachment(
  filename: string,
  contentType: string,
  preview: Attachment["preview"] = "file",
): Attachment {
  return {
    filename,
    contentType,
    byteSize: 1200,
    width: null,
    height: null,
    preview,
    url: `/blobs/${filename}`,
    downloadUrl: `/blobs/${filename}?disposition=attachment`,
    thumbnailUrl: null,
  };
}

function file(messageId: number, filename = `f${messageId}.txt`): RoomFile {
  return {
    messageId,
    threadId: null,
    creatorId: 1,
    attachment: attachment(filename, "text/plain"),
    createdAt: "2026-10-06T09:00:00.000Z",
  };
}

describe("fileQuery", () => {
  it("trims the search and keeps the page between 1 and 20", () => {
    expect(fileQuery("images", "  plan ", 2)).toEqual({
      type: "images",
      filename: "plan",
      page: 2,
    });
    expect(fileQuery("all", "", 0).page).toBe(1);
    expect(fileQuery("all", "", 99).page).toBe(20);
    expect(fileQuery("all", "", 2.7).page).toBe(2);
  });
});

describe("appendFiles", () => {
  it("appends a page without repeating files already listed", () => {
    const merged = appendFiles([file(1), file(2)], [file(2), file(3)]);

    expect(merged.map((entry) => entry.messageId)).toEqual([1, 2, 3]);
  });
});

describe("file presentation", () => {
  it("grids images and videos only", () => {
    expect(showsAsGrid("images")).toBe(true);
    expect(showsAsGrid("videos")).toBe(true);
    expect(showsAsGrid("documents")).toBe(false);
    expect(isFileType("documents")).toBe(true);
    expect(isFileType("music")).toBe(false);
  });

  it("picks a glyph from the preview or content type", () => {
    expect(fileIcon(attachment("a.png", "image/png", "image"))).toBe("image");
    expect(fileIcon(attachment("a.mov", "video/quicktime"))).toBe("film");
    expect(fileIcon(attachment("a.pdf", "application/pdf"))).toBe("file-text");
    expect(fileIcon(attachment("a.zip", "application/zip"))).toBe("file");
  });

  it("tags the extension, and nothing for dotfiles or bare names", () => {
    expect(fileExtension("report.final.pdf")).toBe("PDF");
    expect(fileExtension(".env")).toBe("");
    expect(fileExtension("README")).toBe("");
    expect(fileExtension("trailing.")).toBe("");
  });

  it("formats sizes in decimal units", () => {
    expect(formatBytes(1)).toBe("1 byte");
    expect(formatBytes(820)).toBe("820 bytes");
    expect(formatBytes(14_000)).toBe("14 KB");
    expect(formatBytes(3_240_000)).toBe("3.2 MB");
    expect(formatBytes(2_000_000)).toBe("2 MB");
  });

  it("explains an empty list by search, then by type", () => {
    expect(emptyFilesText("images", " deck ")).toBe('No files match "deck".');
    expect(emptyFilesText("other", "")).toBe("No other files shared here yet.");
  });
});
