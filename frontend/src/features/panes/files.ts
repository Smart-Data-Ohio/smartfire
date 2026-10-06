/** The Files pane's query, paging and file-type presentation. */

import type { Attachment } from "../../gen/Attachment.ts";
import type { FileType } from "../../gen/FileType.ts";
import type { RoomFile } from "../../gen/RoomFile.ts";
import type { FileQuery } from "../../sync/panes.ts";
import type { IconName } from "../../ui/icons/icon.tsx";

export const FILE_TABS: readonly { readonly value: FileType; readonly label: string }[] = [
  { value: "all", label: "All" },
  { value: "images", label: "Images" },
  { value: "videos", label: "Videos" },
  { value: "documents", label: "Docs" },
  { value: "other", label: "Other" },
];

export function isFileType(value: string): value is FileType {
  return FILE_TABS.some((tab) => tab.value === value);
}

/** The server stops at page 20. */
export const LAST_FILE_PAGE = 20;

/** The request for one page: the search trimmed (blank searches everything), pages from 1 to 20. */
export function fileQuery(type: FileType, search: string, page: number): FileQuery {
  return {
    type,
    filename: search.trim(),
    page: Math.min(LAST_FILE_PAGE, Math.max(1, Math.floor(page))),
  };
}

/** A file's identity in the list: its message (one file per message) and its blob URL. */
export function fileKey(file: RoomFile): string {
  return `${file.messageId}:${file.attachment.url}`;
}

/** The next page appended; a file already listed (the list shifted under us) stays put once. */
export function appendFiles(held: readonly RoomFile[], page: readonly RoomFile[]): RoomFile[] {
  const seen = new Set(held.map(fileKey));

  return [...held, ...page.filter((file) => !seen.has(fileKey(file)))];
}

/** Images and videos show as tiles; everything else as rows. */
export function showsAsGrid(type: FileType): boolean {
  return type === "images" || type === "videos";
}

const DOCUMENT_TYPES = [
  "application/pdf",
  "application/msword",
  "application/vnd.openxmlformats-officedocument",
  "application/vnd.ms-",
  "application/vnd.oasis.opendocument",
  "application/rtf",
  "text/",
];

/** The glyph beside a file's name. */
export function fileIcon(attachment: Attachment): IconName {
  if (attachment.preview === "image" || attachment.contentType.startsWith("image/")) {
    return "image";
  }

  if (attachment.preview === "video" || attachment.contentType.startsWith("video/")) {
    return "film";
  }

  return DOCUMENT_TYPES.some((prefix) => attachment.contentType.startsWith(prefix))
    ? "file-text"
    : "file";
}

/** "PDF", "PNG", "ZIP": the file's extension, upper-cased, as a short type tag. */
export function fileExtension(filename: string): string {
  const dot = filename.lastIndexOf(".");

  return dot <= 0 || dot === filename.length - 1 ? "" : filename.slice(dot + 1).toUpperCase();
}

const UNITS = ["KB", "MB", "GB", "TB"];

/** "820 bytes", "14 KB", "3.2 MB". */
export function formatBytes(bytes: number): string {
  if (bytes < 1000) {
    return `${bytes} ${bytes === 1 ? "byte" : "bytes"}`;
  }

  let value = bytes / 1000;
  let unit = 0;

  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit += 1;
  }

  return `${value < 10 ? value.toFixed(1).replace(/\.0$/, "") : Math.round(value)} ${UNITS[unit]}`;
}

/** What an empty list says, for a search or for a type. */
export function emptyFilesText(type: FileType, search: string): string {
  if (search.trim() !== "") {
    return `No files match "${search.trim()}".`;
  }

  const what = {
    all: "files",
    images: "images",
    videos: "videos",
    documents: "documents",
    other: "other files",
  } as const satisfies Record<FileType, string>;

  return `No ${what[type]} shared here yet.`;
}
