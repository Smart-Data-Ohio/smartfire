import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { files as fetchFiles } from "../../api/pane-endpoints.ts";
import type { FileType } from "../../gen/FileType.ts";
import type { RoomFile } from "../../gen/RoomFile.ts";
import { formatFull } from "../../lib/time.ts";
import { mutations } from "../../store/store.ts";
import { runAction } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { Tabs } from "../../ui/tabs.tsx";
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
  showsAsGrid,
} from "./files.ts";
import { PaneFrame, RoomName } from "./pane-frame.tsx";
import { PaneEmpty, PaneError, PaneListSkeleton, PaneSearch } from "./pane-states.tsx";

/** Typing pauses this long before the search goes out. */
const SEARCH_DELAY_MS = 250;

interface Listing {
  readonly status: "loading" | "ready" | "error";
  readonly files: readonly RoomFile[];
  readonly nextPage: number | null;
  readonly loadingMore: boolean;
}

const LOADING: Listing = { status: "loading", files: [], nextPage: null, loadingMore: false };

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
      <a className="file-row-main" href={attachment.url} target="_blank" rel="noopener noreferrer">
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
            <span>{poster?.name ?? UNKNOWN_NAME}</span>
            <span aria-hidden="true">·</span>
            <time dateTime={file.createdAt} title={formatFull(file.createdAt)}>
              {timeAgo(file.createdAt, now)}
            </time>
          </span>
        </span>
      </a>
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

  useEffect(() => {
    const timer = window.setTimeout(() => setQuery(search.trim()), SEARCH_DELAY_MS);

    return () => window.clearTimeout(timer);
  }, [search]);

  const loadPage = (page: number) => {
    const ticket = ++request.current;

    setListing((held) => (page === 1 ? LOADING : { ...held, loadingMore: true }));

    runAction(fetchFiles(roomId, fileQuery(type, query, page))).then(
      (list) => {
        if (ticket !== request.current) {
          return;
        }

        mutations.mergeUsers(list.users);
        setListing((held) => ({
          status: "ready",
          files: page === 1 ? list.files : appendFiles(held.files, list.files),
          nextPage:
            list.nextPage !== null && list.nextPage <= LAST_FILE_PAGE ? list.nextPage : null,
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
  useEffect(() => loadPage(1), [roomId, type, query]);

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
