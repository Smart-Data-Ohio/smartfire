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

/** A file's identity in the list: its message and its blob URL. */
export function fileKey(file: RoomFile): string {
  return `${file.messageId}:${file.attachment.url}`;
}

/** The next page appended; a file already listed (the list shifted under us) stays put once. */
export function appendFiles(held: readonly RoomFile[], page: readonly RoomFile[]): RoomFile[] {
  const seen = new Set(held.map(fileKey));

  return [...held, ...page.filter((file) => !seen.has(fileKey(file)))];
}

/** The pane's active filter, as the server applies it to a page. */
export interface FileFilter {
  readonly type: FileType;
  readonly search: string;
}

const DOCUMENT_FILTER_TYPES = [
  "application/pdf",
  "application/msword",
  "application/vnd.ms-",
  "application/vnd.openxmlformats-officedocument.",
  "application/vnd.oasis.opendocument.",
  "text/",
];

/** Whether a live attachment belongs on the filtered list, by its type group and name. */
export function matchesFileFilter(attachment: Attachment, filter: FileFilter): boolean {
  const contentType = attachment.contentType.toLowerCase();
  const group = contentType.startsWith("image/")
    ? "images"
    : contentType.startsWith("video/")
      ? "videos"
      : DOCUMENT_FILTER_TYPES.some((prefix) =>
            prefix.endsWith("-") || prefix.endsWith(".") || prefix.endsWith("/")
              ? contentType.startsWith(prefix)
              : contentType === prefix,
          )
        ? "documents"
        : "other";

  if (filter.type !== "all" && group !== filter.type) {
    return false;
  }

  const search = filter.search.trim().toLowerCase();

  return search === "" || attachment.filename.toLowerCase().includes(search);
}

/** Newest first, as the server lists files: by attach time, then by message. */
function isNewer(candidate: RoomFile, file: RoomFile): boolean {
  return (
    candidate.createdAt > file.createdAt ||
    (candidate.createdAt === file.createdAt && candidate.messageId > file.messageId)
  );
}

/**
 * Live files keep loaded pages current, including pending reads: a spoiler flag flips in
 * place, a file its message no longer carries (an edit, a deletion) drops out, and a file a
 * message newly carries takes its sorted place. A new file older than everything loaded is
 * held back while further pages remain, since its place may be on one of them.
 */
export function reconcileFiles(
  held: readonly RoomFile[],
  updates: ReadonlyMap<number, readonly RoomFile[]>,
  filter: FileFilter,
  complete: boolean,
): readonly RoomFile[] {
  let changed = false;
  const files: RoomFile[] = [];

  for (const file of held) {
    const live = updates.get(file.messageId);

    if (live === undefined) {
      files.push(file);
      continue;
    }

    const current = live.find((candidate) => candidate.attachment.url === file.attachment.url);

    if (current === undefined) {
      changed = true;
    } else if (current.attachment.spoiler !== file.attachment.spoiler) {
      changed = true;
      files.push({
        ...file,
        attachment: { ...file.attachment, spoiler: current.attachment.spoiler },
      });
    } else {
      files.push(file);
    }
  }

  const listed = new Set(held.map(fileKey));
  const last = files.at(-1);
  const fresh = [...updates.values()]
    .flat()
    .filter(
      (file) =>
        !listed.has(fileKey(file)) &&
        matchesFileFilter(file.attachment, filter) &&
        (complete || last === undefined || !isNewer(last, file)),
    )
    .sort((a, b) => (isNewer(a, b) ? -1 : isNewer(b, a) ? 1 : 0));

  for (const file of fresh) {
    const at = files.findIndex((heldFile) => isNewer(file, heldFile));

    files.splice(at === -1 ? files.length : at, 0, file);
    changed = true;
  }

  return changed ? files : held;
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
