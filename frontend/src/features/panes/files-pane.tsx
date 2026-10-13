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
  retireFileUpdates,
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

interface FileRefresh {
  pending: boolean;
  active: boolean;
  readonly schedule: () => void;
}

const LOADING: Listing = { status: "loading", files: [], nextPage: null, loadingMore: false };

/** Known files update immediately; unknown attach times need a read before insertion. */
function liveEntry(message: MessageDTO, created: boolean, known: ReadonlyMap<string, string>) {
  const files: RoomFile[] = [];
  let needsRead = false;

  for (const attachment of messageFiles(message)) {
    const createdAt =
      known.get(`${message.id}:${attachment.url}`) ?? (created ? message.createdAt : undefined);

    if (createdAt === undefined) {
      needsRead = true;
      continue;
    }

    files.push({
      messageId: message.id,
      threadId: message.threadId,
      creatorId: message.creatorId,
      attachment,
      createdAt,
    });
  }

  return { files, needsRead };
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
  const refreshRequest = useRef<FileRefresh | null>(null);
  const loadedPage = useRef(1);
  const pageLoading = useRef(false);
  const deletedMessages = useRef(new Set<number>());
  const knownTimes = useRef(new Map<string, string>());
  const pendingEdits = useRef(new Map<number, MessageDTO>());
  const removedThreads = useRef(new Set<number>());
  const liveFiles = useRef(new Map<number, readonly RoomFile[]>());

  useEffect(() => {
    const timer = window.setTimeout(() => setQuery(search.trim()), SEARCH_DELAY_MS);

    return () => window.clearTimeout(timer);
  }, [search]);

  const schedulePendingRefresh = () => {
    const refresh = refreshRequest.current;

    if (refresh?.pending === true) refresh.schedule();
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: deletion history belongs to the room, across filters
  useEffect(() => {
    deletedMessages.current.clear();
  }, [roomId]);

  const applyPendingEdits = (snapshot: ReadonlyMap<number, MessageDTO>) => {
    for (const [id, message] of snapshot) {
      if (pendingEdits.current.get(id) === message) pendingEdits.current.delete(id);
    }

    for (const [id, message] of pendingEdits.current) {
      const entry = liveEntry(message, false, knownTimes.current);

      liveFiles.current.set(id, entry.files);

      if (!entry.needsRead) pendingEdits.current.delete(id);
    }
  };

  const loadPage = (page: number) => {
    const ticket = ++request.current;
    const snapshot = new Map(liveFiles.current);
    const edits = new Map(pendingEdits.current);
    const upperBound = page === 1 ? undefined : listing.files.at(-1);

    pageLoading.current = true;

    if (refreshRequest.current?.active === true) refreshRequest.current.pending = true;

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

        pageLoading.current = false;

        for (const file of list.files) knownTimes.current.set(fileKey(file), file.createdAt);
        retireFileUpdates(liveFiles.current, snapshot, {
          files: list.files,
          filter: { type, search: query },
          complete: nextPage === null,
          upperBound,
        });
        applyPendingEdits(edits);
        setListing((held) => ({
          status: "ready",
          files: reconcileFiles(
            (page === 1 ? list.files : appendFiles(held.files, list.files)).filter(
              (file) =>
                !deletedMessages.current.has(file.messageId) &&
                (file.threadId === null || !removedThreads.current.has(file.threadId)),
            ),
            liveFiles.current,
            { type, search: query },
            nextPage === null,
          ),
          nextPage,
          loadingMore: false,
        }));
        schedulePendingRefresh();
      },
      () => {
        if (ticket === request.current) {
          pageLoading.current = false;
          setListing((held) =>
            page === 1 ? { ...LOADING, status: "error" } : { ...held, loadingMore: false },
          );
          schedulePendingRefresh();
        }
      },
    );
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: a new room, type or search starts over at page 1
  useEffect(() => {
    liveFiles.current = new Map();
    knownTimes.current.clear();
    pendingEdits.current.clear();
    let refreshTimer: number | undefined;

    const refreshState: FileRefresh = {
      pending: false,
      active: false,
      schedule: () => {
        refreshState.pending = true;

        if (refreshState.active || pageLoading.current || refreshTimer !== undefined) return;
        refreshTimer = window.setTimeout(() => {
          refreshTimer = undefined;
          void refresh();
        }, FILES_REFRESH_DELAY_MS);
      },
    };

    let stopped = false;

    const refresh = async () => {
      if (stopped || refreshState.active || pageLoading.current) return;
      const ticket = ++request.current;
      const snapshot = new Map(liveFiles.current);
      const edits = new Map(pendingEdits.current);

      refreshState.active = true;
      refreshState.pending = false;
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

        for (const file of files) knownTimes.current.set(fileKey(file), file.createdAt);
        retireFileUpdates(liveFiles.current, snapshot, {
          files,
          filter: { type, search: query },
          complete: nextPage === null,
        });
        applyPendingEdits(edits);
        setListing((held) => {
          if (ticket !== request.current) return held;

          return {
            status: "ready",
            files: reconcileFiles(
              files.filter(
                (file) =>
                  !deletedMessages.current.has(file.messageId) &&
                  (file.threadId === null || !removedThreads.current.has(file.threadId)),
              ),
              liveFiles.current,
              { type, search: query },
              nextPage === null,
            ),
            nextPage,
            loadingMore: false,
          };
        });
      } catch {
        if (ticket === request.current) {
          setListing((held) =>
            held.status === "loading"
              ? { ...LOADING, status: "error" }
              : { ...held, loadingMore: false },
          );
        }
      } finally {
        refreshState.active = false;

        if (!stopped && refreshState.pending) refreshState.schedule();
      }
    };

    refreshRequest.current = refreshState;

    const stopFiles = panes.onFilesChanged(roomId, () => {
      refreshState.pending = true;
      refreshState.schedule();
    });

    const stopThread = panes.onThreadRemoved(roomId, (threadId) => {
      removedThreads.current.add(threadId);
      setListing((held) => ({
        ...held,
        files: held.files.filter((file) => file.threadId !== threadId),
      }));
    });

    const applyRemoval = (messageId: number) => {
      deletedMessages.current.add(messageId);
      liveFiles.current.delete(messageId);
      pendingEdits.current.delete(messageId);
      setListing((held) => ({
        ...held,
        files: held.files.filter((file) => file.messageId !== messageId),
      }));
    };

    const stopChange = panes.onMessageChange(roomId, (message, created) => {
      if (message.threadId !== null || deletedMessages.current.has(message.id)) return;
      const entry = liveEntry(message, created, knownTimes.current);

      if (entry.needsRead) {
        pendingEdits.current.set(message.id, message);
        refreshState.schedule();
      } else {
        pendingEdits.current.delete(message.id);
      }

      for (const file of entry.files) knownTimes.current.set(fileKey(file), file.createdAt);
      liveFiles.current.set(message.id, entry.files);
      setListing((held) => {
        const files = reconcileFiles(
          held.files,
          liveFiles.current,
          { type, search: query },
          held.nextPage === null,
        );

        return files === held.files ? held : { ...held, files };
      });
    });

    const stopRemoved = panes.onMessageRemoved(roomId, applyRemoval);

    loadPage(1);

    return () => {
      stopped = true;
      stopChange();
      stopRemoved();
      stopFiles();
      stopThread();
      window.clearTimeout(refreshTimer);
      refreshRequest.current = null;
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
