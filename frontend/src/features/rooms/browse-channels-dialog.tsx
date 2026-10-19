import { useNavigate } from "@tanstack/react-router";
import { type KeyboardEvent, useEffect, useEffectEvent, useId, useRef, useState } from "react";
import type { DirectoryChannel } from "../../gen/DirectoryChannel.ts";
import { SuccessCheck } from "../../motion/success-check.tsx";
import { useStore } from "../../store/store.ts";
import { Button, Spinner } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Highlighted } from "../switcher/highlighted.tsx";
import { timeAgo } from "../threads/thread-format.ts";
import { useNow } from "../threads/use-now.ts";
import { memberCountLabel, useChannelDirectory, useChannelJoins } from "./channel-directory.ts";
import { openNewRoom } from "./new-room-store.ts";
import "./browse-channels.css";

interface BrowseChannelsDialogProps {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

/** ↑/↓ between the rows' open buttons, ↑ off the first row back to the search field. */
function moveFocus(event: KeyboardEvent<HTMLElement>, list: HTMLElement | null) {
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") {
    return;
  }

  const rows = [...(list?.querySelectorAll<HTMLElement>(".browse-open") ?? [])];
  const at = rows.indexOf(event.currentTarget);
  const next = event.key === "ArrowDown" ? at + 1 : at - 1;

  event.preventDefault();

  if (next < 0) {
    list?.closest(".browse")?.querySelector<HTMLElement>(".browse-input")?.focus();
  } else {
    rows[Math.min(next, rows.length - 1)]?.focus();
  }
}

interface RowProps {
  readonly channel: DirectoryChannel;
  readonly query: string;
  readonly joined: boolean;
  readonly now: number;
  readonly onOpen: () => void;
  readonly onJoin: () => void;
  readonly onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
}

function ChannelRow({ channel, query, joined, now, onOpen, onJoin, onKeyDown }: RowProps) {
  const name = channel.name === "" ? "Untitled channel" : channel.name;

  return (
    <li className="browse-row enter-fade" data-joined={joined || undefined}>
      <button type="button" className="browse-open" onClick={onOpen} onKeyDown={onKeyDown}>
        <span className="browse-name">
          <Icon name="hash" size={14} className="browse-hash" />
          <span className="browse-label">
            <Highlighted text={name} query={query} />
          </span>
        </span>
        {channel.topic === null || channel.topic === "" ? null : (
          <span className="browse-topic">
            <Highlighted text={channel.topic} query={query} />
          </span>
        )}
        <span className="browse-meta">
          {memberCountLabel(channel.memberCount)} · active {timeAgo(channel.lastActiveAt, now)}
        </span>
      </button>
      {joined ? (
        <span className="browse-joined">
          <SuccessCheck size={14} />
          Joined
        </span>
      ) : (
        <Button
          size="sm"
          variant="secondary"
          className="browse-join"
          aria-label={`Join #${name}`}
          onClick={onJoin}
        >
          Join
        </Button>
      )}
    </li>
  );
}

function NextPage({ loading, onVisible }: { loading: boolean; onVisible: () => void }) {
  const ref = useRef<HTMLDivElement | null>(null);
  const reached = useEffectEvent(onVisible);

  useEffect(() => {
    const node = ref.current;

    if (node === null || !("IntersectionObserver" in window)) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          reached();
        }
      },
      { root: node.closest(".browse-results"), rootMargin: "0px 0px 240px 0px" },
    );

    observer.observe(node);

    return () => observer.disconnect();
  }, []);

  return (
    <div ref={ref} className="browse-end">
      {loading ? <Spinner label="Loading more channels" /> : null}
    </div>
  );
}

/**
 * Browse channels: every open room, searched by name or topic, with Join beside each one you
 * aren't in (a hidden one counts as left). Joining shows Joined at once and adds the sidebar row
 * when the server answers; a failure puts Join back and says why. A row opens its channel.
 */
export default function BrowseChannelsDialog({ open, onOpenChange }: BrowseChannelsDialogProps) {
  const id = useId();
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [wasOpen, setWasOpen] = useState(open);
  const directory = useChannelDirectory(open, query);
  const { joined, join } = useChannelJoins();
  const canCreateRooms = useStore((state) => state.sidebar.canCreateRooms);
  const now = useNow();
  const listRef = useRef<HTMLUListElement | null>(null);

  if (open !== wasOpen) {
    setWasOpen(open);

    if (open) {
      setQuery("");
    }
  }

  const trimmed = query.trim();
  const searching = directory.status === "loading" || directory.query !== trimmed;

  const openChannel = (channel: DirectoryChannel) => {
    void navigate({ to: "/r/$roomId", params: { roomId: channel.roomId } });
    onOpenChange(false);
  };

  const onSearchKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      listRef.current?.querySelector<HTMLElement>(".browse-open")?.focus();
    }
  };

  const empty = directory.status === "ready" && directory.channels.length === 0 && !searching;

  return (
    <Dialog open={open} onOpenChange={onOpenChange} title="Browse channels">
      <div className="browse">
        <div className="browse-search">
          <Icon name="search" size={16} className="browse-search-icon" />
          <input
            className="browse-input"
            type="search"
            aria-label="Search channels"
            aria-controls={`${id}-list`}
            placeholder="Search by name or topic"
            autoComplete="off"
            spellCheck={false}
            value={query}
            data-autofocus="always"
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={onSearchKeyDown}
          />
          {searching && directory.status !== "failed" ? <Spinner label="Searching" /> : null}
        </div>
        <div className="browse-results" aria-busy={searching}>
          <ul
            ref={listRef}
            id={`${id}-list`}
            className="browse-list"
            aria-label="Channels"
            data-stale={searching || undefined}
          >
            {directory.channels.map((channel) => (
              <ChannelRow
                key={channel.roomId}
                channel={channel}
                query={directory.query}
                joined={joined(channel)}
                now={now}
                onOpen={() => openChannel(channel)}
                onJoin={() => join(channel)}
                onKeyDown={(event) => moveFocus(event, listRef.current)}
              />
            ))}
          </ul>
          {directory.loadMore === null || directory.nextCursor === null ? null : (
            // A new observer reports at once, so a page that leaves this in view loads the next.
            <NextPage
              key={directory.nextCursor}
              loading={directory.loadingMore}
              onVisible={directory.loadMore}
            />
          )}
          {directory.status === "failed" ? (
            <div className="browse-empty" role="status">
              <span className="browse-empty-title">Couldn't load channels</span>
              <span>Check your connection and try the search again.</span>
            </div>
          ) : null}
          {empty ? (
            <div className="browse-empty enter-fade" role="status">
              <span className="browse-empty-title">
                {trimmed === "" ? "No channels yet" : `No channels match “${trimmed}”`}
              </span>
              {canCreateRooms ? (
                <Button
                  size="sm"
                  variant="ghost"
                  icon="plus"
                  onClick={() => {
                    onOpenChange(false);
                    openNewRoom();
                  }}
                >
                  Create a channel
                </Button>
              ) : null}
            </div>
          ) : null}
        </div>
      </div>
    </Dialog>
  );
}
