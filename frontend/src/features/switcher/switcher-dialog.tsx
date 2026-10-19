import { useNavigate } from "@tanstack/react-router";
import { type KeyboardEvent, useEffect, useId, useState } from "react";
import { useStore } from "../../store/store.ts";
import { directs } from "../../sync/directs.ts";
import { actions } from "../../sync/runtime.ts";
import { Badge } from "../../ui/badge.tsx";
import { Button, Spinner } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Kbd } from "../../ui/kbd.tsx";
import { toast } from "../../ui/toast-store.ts";
import { namesWithYou, offersNoteToSelf } from "../directs/note-to-self.ts";
import { AgentBadge } from "../people/agent-badge.tsx";
import { UserAvatar } from "../people/user-avatar.tsx";
import { ROOM_KIND_ICON } from "../room/room-icon.ts";
import { memberCountLabel, useChannelDirectory } from "../rooms/channel-directory.ts";
import { GroupAvatars } from "../sidebar/group-avatars.tsx";
import { notificationLabel } from "../sidebar/sections.ts";
import { Highlighted } from "./highlighted.tsx";
import { matchScore, normalizeQuery } from "./match.ts";
import { openOverlay } from "./overlay-store.ts";
import {
  flattenSections,
  joinableSection,
  localItems,
  mergeItems,
  rankItems,
  readsUnread,
  remoteItems,
  type SwitcherItem,
  withoutRemovedThreads,
} from "./ranking.ts";
import { readRecents, recordRecent } from "./recents.ts";
import { useCatalogue } from "./use-catalogue.ts";
import "./switcher.css";
import { PrivateGlyph, usePrivateThread } from "../threads/private-thread.tsx";

interface SwitcherDialogProps {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

/** A thread's glyph: a lock once the thread is held and private (the catalogue doesn't say). */
function ThreadGlyph({ threadId }: { readonly threadId: number | null }) {
  const secret = usePrivateThread(threadId);

  return secret ? <PrivateGlyph size={16} /> : <Icon name="thread" size={16} />;
}

function ItemGlyph({ item }: { readonly item: SwitcherItem }) {
  if (item.kind === "person" && item.userId !== null) {
    return <UserAvatar userId={item.userId} size={20} presence decorative />;
  }

  if (item.kind === "thread") {
    return <ThreadGlyph threadId={item.threadId} />;
  }

  if (item.roomKind === "direct") {
    return <GroupAvatars ids={item.memberIds} size={20} />;
  }

  return <Icon name={ROOM_KIND_ICON[item.roomKind ?? "open"]} size={16} />;
}

function itemDetail(item: SwitcherItem): string | null {
  if (item.kind === "thread") {
    return item.detail === null ? "Thread" : `in #${item.detail}`;
  }

  if (item.kind === "joinable") {
    return item.detail;
  }

  return item.kind === "person" && item.roomId === null ? "New message" : null;
}

interface OptionProps {
  readonly item: SwitcherItem;
  readonly id: string;
  readonly active: boolean;
  readonly busy: boolean;
  readonly query: string;
  readonly onHover: () => void;
  readonly onChoose: () => void;
}

function Option({ item, id, active, busy, query, onHover, onChoose }: OptionProps) {
  const detail = itemDetail(item);

  return (
    // The input owns the keyboard (aria-activedescendant), so options take clicks only.
    // biome-ignore lint/a11y/useKeyWithClickEvents: the combobox input handles the keys.
    <div
      id={id}
      role="option"
      tabIndex={-1}
      aria-selected={active}
      className="switcher-option"
      data-active={active || undefined}
      data-unread={readsUnread(item) || undefined}
      data-muted={item.muted || undefined}
      onPointerMove={active ? undefined : onHover}
      onClick={onChoose}
    >
      <span className="switcher-glyph">
        <ItemGlyph item={item} />
      </span>
      <span className="switcher-label">
        <Highlighted text={item.label} query={query} />
      </span>
      {item.kind === "person" && item.userId !== null ? <AgentBadge userId={item.userId} /> : null}
      {detail === null ? null : <span className="switcher-detail">{detail}</span>}
      <span className="switcher-meta">
        {busy ? <Spinner label={item.kind === "joinable" ? "Joining" : "Opening"} /> : null}
        {!busy && item.count > 0 ? (
          <Badge count={item.count} label={notificationLabel(item.count)} />
        ) : null}
        {!busy && item.count === 0 && item.unread ? (
          <span className="switcher-unread-dot" aria-label="Unread" role="img" />
        ) : null}
        {item.kind === "joinable" && !busy ? <span className="switcher-join">Join</span> : null}
        <Kbd keys={["⏎"]} className="switcher-enter" />
      </span>
    </div>
  );
}

/**
 * The quick switcher (⌘K): type to jump to any room, person or recent thread. The sidebar's
 * rooms answer the first keystroke; the server's catalogue joins as soon as it arrives. With no
 * query it opens on your recent picks and unread conversations. ↑/↓ move, Enter opens, Esc
 * closes; picking someone you have no DM with yet opens one. Open channels you aren't in that
 * match come last, under "Channels you can join": picking one joins it and opens it.
 */
export default function SwitcherDialog({ open, onOpenChange }: SwitcherDialogProps) {
  const id = useId();
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const [recents, setRecents] = useState(readRecents);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [wasOpen, setWasOpen] = useState(open);
  const sidebar = useStore((state) => state.sidebar);
  const users = useStore((state) => state.users);
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  const notesOffered = useStore(offersNoteToSelf);
  const { catalogue, loading } = useCatalogue(open);
  const removedThreads = useStore((state) => state.removedThreads);

  if (open !== wasOpen) {
    setWasOpen(open);

    if (open) {
      setQuery("");
      setActiveIndex(0);
      setRecents(readRecents());
      setBusyKey(null);
    }
  }

  const items = mergeItems(
    localItems(sidebar, viewerId),
    catalogue === null ? [] : remoteItems(withoutRemovedThreads(catalogue, removedThreads)),
    namesWithYou(users, viewerId),
  );

  const trimmed = query.trim();
  const directory = useChannelDirectory(open && trimmed !== "", trimmed);
  const needle = normalizeQuery(trimmed);

  // The last answer stands in while the next is on its way, for as long as its names still match.
  const joinable = directory.channels.flatMap((channel) =>
    channel.joined ||
    trimmed === "" ||
    (directory.query !== trimmed && matchScore(channel.name, needle) === null)
      ? []
      : [
          {
            roomId: channel.roomId,
            name: channel.name,
            detail: memberCountLabel(channel.memberCount),
          },
        ],
  );

  const sections = [...rankItems(items, query, recents), ...joinableSection(joinable, items)];
  const flat = flattenSections(sections);
  const active = Math.min(activeIndex, Math.max(flat.length - 1, 0));
  const activeItem = flat[active];
  const optionId = (item: SwitcherItem) => `${id}-${item.key}`;

  useEffect(() => {
    if (activeItem === undefined) {
      return;
    }

    document.getElementById(`${id}-${activeItem.key}`)?.scrollIntoView({ block: "nearest" });
  }, [activeItem, id]);

  const goTo = (roomId: number, threadId: number | null) => {
    if (threadId === null) {
      void navigate({ to: "/r/$roomId", params: { roomId } });
    } else {
      void navigate({ to: "/r/$roomId/t/$threadId", params: { roomId, threadId } });
    }

    onOpenChange(false);
  };

  const choose = (item: SwitcherItem | undefined) => {
    if (item === undefined || busyKey !== null) {
      return;
    }

    recordRecent(item.kind === "joinable" ? `room:${item.roomId}` : item.key);

    // Your notes while the sidebar offers them open as the offer does, back in the sidebar.
    const reopensNotes = notesOffered && item.kind === "person" && item.userId === viewerId;

    if (item.kind === "joinable" && item.roomId !== null) {
      const roomId = item.roomId;

      setBusyKey(item.key);
      actions.joinOpenRoom(roomId).then(
        () => {
          setBusyKey(null);
          goTo(roomId, null);
        },
        (error: Error) => {
          setBusyKey(null);
          toast({
            title: `Couldn't join #${item.label}`,
            description: error.message,
            tone: "danger",
          });
        },
      );

      return;
    }

    if (item.roomId !== null && !reopensNotes) {
      goTo(item.roomId, item.threadId);

      return;
    }

    if (item.userId === null) {
      return;
    }

    setBusyKey(item.key);
    directs.create([item.userId]).then(
      (row) => {
        setBusyKey(null);
        goTo(row.room.id, null);
      },
      (error: Error) => {
        setBusyKey(null);
        toast({
          title: "Couldn't open the conversation",
          description: error.message,
          tone: "danger",
        });
      },
    );
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    const count = flat.length;

    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();

      if (count > 0) {
        const step = event.key === "ArrowDown" ? 1 : -1;

        setActiveIndex((active + step + count) % count);
      }
    } else if (event.key === "Enter" && !event.nativeEvent.isComposing) {
      event.preventDefault();
      choose(activeItem);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange} title="Jump to a conversation">
      <div className="switcher">
        <div className="switcher-search">
          <Icon name="search" size={18} className="switcher-search-icon" />
          <input
            className="switcher-input"
            type="text"
            role="combobox"
            aria-label="Search conversations, people and threads"
            aria-expanded="true"
            aria-controls={`${id}-list`}
            aria-autocomplete="list"
            aria-activedescendant={activeItem === undefined ? undefined : optionId(activeItem)}
            placeholder="Jump to a conversation, person or thread…"
            autoComplete="off"
            spellCheck={false}
            value={query}
            data-autofocus="always"
            onChange={(event) => {
              setQuery(event.target.value);
              setActiveIndex(0);
            }}
            onKeyDown={onKeyDown}
          />
          {loading && trimmed !== "" ? <Spinner label="Searching" /> : null}
        </div>
        <div id={`${id}-list`} role="listbox" aria-label="Results" className="switcher-results">
          {sections.map((section) => (
            // A listbox groups its options with role="group"; a fieldset would be a form part.
            // biome-ignore lint/a11y/useSemanticElements: ARIA listbox grouping.
            <div
              key={section.key}
              role="group"
              aria-labelledby={`${id}-${section.key}`}
              className="switcher-section"
            >
              <div id={`${id}-${section.key}`} className="switcher-section-title">
                {section.title}
              </div>
              {section.items.map((item) => (
                <Option
                  key={item.key}
                  item={item}
                  id={optionId(item)}
                  active={item === activeItem}
                  busy={busyKey === item.key}
                  query={query}
                  onHover={() => setActiveIndex(flat.indexOf(item))}
                  onChoose={() => choose(item)}
                />
              ))}
            </div>
          ))}
          {flat.length === 0 ? (
            <div className="switcher-empty">
              {trimmed === "" ? (
                "Nothing here yet."
              ) : (
                <>
                  <span className="switcher-empty-title">No matches for “{trimmed}”</span>
                  <span>Try a channel or a person's name.</span>
                </>
              )}
            </div>
          ) : null}
        </div>
        <footer className="switcher-footer">
          <span className="switcher-hints" aria-hidden="true">
            <span>
              <Kbd keys={["↑", "↓"]} /> to move
            </span>
            <span>
              <Kbd keys={["⏎"]} /> to open
            </span>
            <span>
              <Kbd keys={["Esc"]} /> to close
            </span>
          </span>
          <Button
            variant="ghost"
            size="sm"
            icon="hash"
            className="switcher-browse"
            onClick={() => openOverlay("browse-channels")}
          >
            Browse channels
          </Button>
        </footer>
      </div>
    </Dialog>
  );
}
