import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import type { FileType } from "../../gen/FileType.ts";
import type { MessageDTO } from "../../gen/MessageDTO.ts";
import type { RoomFile } from "../../gen/RoomFile.ts";
import { formatFull } from "../../lib/time.ts";
import { panes } from "../../sync/panes.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { Tabs } from "../../ui/tabs.tsx";
import { FileSpoiler } from "../messages/file-spoiler.tsx";
import { messageFiles } from "../messages/message-files.ts";
import { UNKNOWN_NAME, useUser } from "../people/people.ts";
import { timeAgo } from "../threads/thread-format.ts";
import { useNow } from "../threads/use-now.ts";
import {
  appendFiles,
  emptyFilesText,
  FILE_TABS,
  fileExtension,
  fileIcon,
  fileKey,
  fileQuery,
  formatBytes,
  isFileType,
  LAST_FILE_PAGE,
  reconcileFiles,
  showsAsGrid,
} from "./files.ts";
import { PaneFrame, RoomName } from "./pane-frame.tsx";
import { PaneEmpty, PaneError, PaneListSkeleton, PaneSearch } from "./pane-states.tsx";

/** Typing pauses this long before the search goes out. */
const SEARCH_DELAY_MS = 250;

const FILES_REFRESH_DELAY_MS = 100;

interface Listing {
  readonly status: "loading" | "ready" | "error";
  readonly files: readonly RoomFile[];
  readonly nextPage: number | null;
  readonly loadingMore: boolean;
}

const LOADING: Listing = { status: "loading", files: [], nextPage: null, loadingMore: false };

/**
 * A live message as pane rows. A file already known keeps the time it was first seen; a file
 * first seen now was attached when the message was posted, or when the edit that added it landed.
 */
function liveEntry(
  message: MessageDTO,
  created: boolean,
  known: ReadonlyMap<string, string>,
): RoomFile[] {
  const attachedAt = created ? message.createdAt : message.updatedAt;

  return messageFiles(message).map((attachment) => ({
    messageId: message.id,
    threadId: message.threadId,
    creatorId: message.creatorId,
    attachment,
    createdAt: known.get(attachment.url) ?? attachedAt,
  }));
}

/** Starts a download of `url` (the `?disposition=attachment` link) without leaving the page. */
function download(url: string, filename: string): void {
  const link = document.createElement("a");

  link.href = url;
  link.download = filename;
  link.rel = "noopener";
  document.body.append(link);
  link.click();
  link.remove();
}

interface FileItemProps {
  readonly file: RoomFile;
  readonly now: number;
  readonly onJump: (file: RoomFile) => void;
}

/** A file as a row: its type glyph (or a thumbnail), name, size, poster and age. */
function FileRow({ file, now, onJump }: FileItemProps) {
  const poster = useUser(file.creatorId);
  const { attachment } = file;
  const extension = fileExtension(attachment.filename);

  return (
    <li className="file-row">
      <FileSpoiler spoiler={attachment.spoiler}>
        <a
          className="file-row-main"
          href={attachment.url}
          target="_blank"
          rel="noopener noreferrer"
        >
          {attachment.thumbnailUrl === null ? (
            <span className="file-glyph" data-kind={fileIcon(attachment)} aria-hidden="true">
              <Icon name={fileIcon(attachment)} size={18} />
            </span>
          ) : (
            <img className="file-thumb" src={attachment.thumbnailUrl} alt="" loading="lazy" />
          )}
          <span className="file-row-text">
            <span className="file-row-name">{attachment.filename}</span>
            <span className="file-row-meta">
              {extension === "" ? null : <span className="file-ext">{extension}</span>}
              <span className="tabular">{formatBytes(attachment.byteSize)}</span>
              <span aria-hidden="true">·</span>
              <span className="file-row-poster">{poster?.name ?? UNKNOWN_NAME}</span>
              <span aria-hidden="true">·</span>
              <time dateTime={file.createdAt} title={formatFull(file.createdAt)}>
                {timeAgo(file.createdAt, now)}
              </time>
            </span>
          </span>
        </a>
      </FileSpoiler>
      <span className="file-row-actions">
        <IconButton
          icon="arrow-up-right"
          size="sm"
          label="Jump to message"
          onClick={() => onJump(file)}
        />
        <IconButton
          icon="download"
          size="sm"
          label={`Download ${attachment.filename}`}
          onClick={() => download(attachment.downloadUrl, attachment.filename)}
        />
      </span>
    </li>
  );
}

/** An image or video as a tile: the still, the name on hover, open on click. */
function FileTile({ file, onJump }: Omit<FileItemProps, "now">) {
  const { attachment } = file;

  return (
    <li className="file-tile">
      <FileSpoiler spoiler={attachment.spoiler} tile>
        <a
          className="file-tile-link"
          href={attachment.url}
          target="_blank"
          rel="noopener noreferrer"
          aria-label={`Open ${attachment.filename}`}
        >
          {attachment.thumbnailUrl === null ? (
            <span className="file-tile-glyph" aria-hidden="true">
              <Icon name={fileIcon(attachment)} size={22} />
            </span>
          ) : (
            <img className="file-tile-image" src={attachment.thumbnailUrl} alt="" loading="lazy" />
          )}
          {attachment.preview === "video" ? (
            <span className="file-tile-badge" aria-hidden="true">
              <Icon name="film" size={12} />
            </span>
          ) : null}
          <span className="file-tile-name" aria-hidden="true">
            {attachment.filename}
          </span>
        </a>
      </FileSpoiler>
      <span className="file-tile-actions">
        <IconButton
          icon="arrow-up-right"
          size="sm"
          label="Jump to message"
          onClick={() => onJump(file)}
        />
        <IconButton
          icon="download"
          size="sm"
          label={`Download ${attachment.filename}`}
          onClick={() => download(attachment.downloadUrl, attachment.filename)}
        />
      </span>
    </li>
  );
}

function FileGridSkeleton() {
  return (
    <div className="file-grid-skeleton" aria-hidden="true">
      {[1, 2, 3, 4, 5, 6].map((index) => (
        <Skeleton key={index} width="100%" height={104} radius="md" />
      ))}
    </div>
  );
}

/**
 * The Files pane: everything attached in the room (replies too), newest first, filtered by type
 * and by name. Images and videos lay out as a grid of stills, everything else as rows; 30 a page
 * with "Load more". Open a file in a new tab, download it, or jump to the message it came with.
 */
export function FilesPane({ roomId }: { readonly roomId: number }) {
  const navigate = useNavigate();
  const now = useNow();
  const [type, setType] = useState<FileType>("all");
  const [search, setSearch] = useState("");
  const [query, setQuery] = useState("");
  const [listing, setListing] = useState<Listing>(LOADING);
  const request = useRef(0);
  const loadedPage = useRef(1);
  const removedThreads = useRef(new Set<number>());
  const liveFiles = useRef(new Map<number, readonly RoomFile[]>());

  useEffect(() => {
    const timer = window.setTimeout(() => setQuery(search.trim()), SEARCH_DELAY_MS);

    return () => window.clearTimeout(timer);
  }, [search]);

  const loadPage = (page: number) => {
    const ticket = ++request.current;

    loadedPage.current = page;
    removedThreads.current.clear();
    setListing((held) => (page === 1 ? LOADING : { ...held, loadingMore: true }));

    panes.files(roomId, fileQuery(type, query, page)).then(
      (list) => {
        if (ticket !== request.current) {
          return;
        }

        const nextPage =
          list.nextPage !== null && list.nextPage <= LAST_FILE_PAGE ? list.nextPage : null;

        setListing((held) => ({
          status: "ready",
          files: reconcileFiles(
            (page === 1 ? list.files : appendFiles(held.files, list.files)).filter(
              (file) => file.threadId === null || !removedThreads.current.has(file.threadId),
            ),
            liveFiles.current,
            { type, search: query },
            nextPage === null,
          ),
          nextPage,
          loadingMore: false,
        }));
      },
      () => {
        if (ticket === request.current) {
          setListing((held) =>
            page === 1 ? { ...LOADING, status: "error" } : { ...held, loadingMore: false },
          );
        }
      },
    );
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: a new room, type or search starts over at page 1
  useEffect(() => {
    liveFiles.current = new Map();
    let refreshTimer: number | undefined;

    const refresh = async () => {
      const ticket = ++request.current;
      const pages = loadedPage.current;
      let files: RoomFile[] = [];
      let nextPage: number | null = 1;

      removedThreads.current.clear();

      try {
        for (let page = 1; page <= pages && nextPage !== null; page += 1) {
          const list = await panes.files(roomId, fileQuery(type, query, page));

          if (ticket !== request.current) return;
          files = appendFiles(files, list.files);
          nextPage =
            list.nextPage !== null && list.nextPage <= LAST_FILE_PAGE ? list.nextPage : null;
        }

        setListing({
          status: "ready",
          files: reconcileFiles(
            files.filter(
              (file) => file.threadId === null || !removedThreads.current.has(file.threadId),
            ),
            liveFiles.current,
            { type, search: query },
            nextPage === null,
          ),
          nextPage,
          loadingMore: false,
        });
      } catch {
        if (ticket === request.current) {
          setListing((held) =>
            held.status === "loading"
              ? { ...LOADING, status: "error" }
              : { ...held, loadingMore: false },
          );
        }
      }
    };

    const stopFiles = panes.onFilesChanged(roomId, () => {
      window.clearTimeout(refreshTimer);
      refreshTimer = window.setTimeout(() => void refresh(), FILES_REFRESH_DELAY_MS);
    });

    const stopThread = panes.onThreadRemoved(roomId, (threadId) => {
      removedThreads.current.add(threadId);
      setListing((held) => ({
        ...held,
        files: held.files.filter((file) => file.threadId !== threadId),
      }));
    });

    const applyLive = () =>
      setListing((held) => {
        const files = reconcileFiles(
          held.files,
          liveFiles.current,
          { type, search: query },
          held.nextPage === null,
        );

        return files === held.files ? held : { ...held, files };
      });

    const stopChange = panes.onMessageChange(roomId, (message, created) => {
      if (message.threadId !== null) return;
      setListing((held) => {
        const known = new Map<string, string>();

        for (const file of held.files) {
          if (file.messageId === message.id) {
            known.set(file.attachment.url, file.createdAt);
          }
        }

        for (const file of liveFiles.current.get(message.id) ?? []) {
          known.set(file.attachment.url, file.createdAt);
        }

        liveFiles.current.set(message.id, liveEntry(message, created, known));

        const files = reconcileFiles(
          held.files,
          liveFiles.current,
          { type, search: query },
          held.nextPage === null,
        );

        return files === held.files ? held : { ...held, files };
      });
    });

    const stopRemoved = panes.onMessageRemoved(roomId, (messageId) => {
      liveFiles.current.set(messageId, []);
      applyLive();
    });

    loadPage(1);

    return () => {
      stopChange();
      stopRemoved();
      stopFiles();
      stopThread();
      window.clearTimeout(refreshTimer);
      request.current += 1;
    };
  }, [roomId, type, query]);

  const jump = (file: RoomFile) => {
    if (file.threadId === null) {
      void navigate({
        to: "/r/$roomId/m/$messageId",
        params: { roomId, messageId: file.messageId },
      });
    } else {
      void navigate({ to: "/r/$roomId/t/$threadId", params: { roomId, threadId: file.threadId } });
    }
  };

  const grid = showsAsGrid(type);

  return (
    <PaneFrame
      title="Files"
      subtitle={<RoomName roomId={roomId} />}
      toolbar={
        <>
          <PaneSearch value={search} onValueChange={setSearch} label="Search files by name" />
          <Tabs
            items={FILE_TABS}
            value={type}
            onValueChange={(value) => (isFileType(value) ? setType(value) : undefined)}
            label="File type"
          />
        </>
      }
    >
      {listing.status === "error" ? (
        <PaneError message="The files couldn't be loaded." onRetry={() => loadPage(1)} />
      ) : (
        <SkeletonReveal
          loading={listing.status === "loading"}
          skeleton={grid ? <FileGridSkeleton /> : <PaneListSkeleton rows={6} square={40} />}
        >
          {listing.status === "ready" && listing.files.length === 0 ? (
            <PaneEmpty icon="file" title="No files" text={emptyFilesText(type, query)} />
          ) : (
            <>
              <ul className={grid ? "file-grid" : "file-list"} aria-label="Files">
                {listing.files.map((file) =>
                  grid ? (
                    <FileTile key={fileKey(file)} file={file} onJump={jump} />
                  ) : (
                    <FileRow key={fileKey(file)} file={file} now={now} onJump={jump} />
                  ),
                )}
              </ul>
              {listing.nextPage === null ? null : (
                <div className="file-more">
                  <Button
                    variant="secondary"
                    size="sm"
                    loading={listing.loadingMore}
                    loadingLabel="Loading"
                    onClick={() => listing.nextPage !== null && loadPage(listing.nextPage)}
                  >
                    Load more
                  </Button>
                </div>
              )}
            </>
          )}
        </SkeletonReveal>
      )}
    </PaneFrame>
  );
}
