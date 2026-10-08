import { useNavigate } from "@tanstack/react-router";
import { type KeyboardEvent, useEffect, useId, useState } from "react";
import { useStore } from "../../store/store.ts";
import { directs } from "../../sync/directs.ts";
import { Badge } from "../../ui/badge.tsx";
import { Spinner } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Kbd } from "../../ui/kbd.tsx";
import { toast } from "../../ui/toast-store.ts";
import { AgentBadge } from "../people/agent-badge.tsx";
import { UserAvatar } from "../people/user-avatar.tsx";
import { ROOM_KIND_ICON } from "../room/room-icon.ts";
import { GroupAvatars } from "../sidebar/group-avatars.tsx";
import { matchRange, normalizeQuery } from "./match.ts";
import {
  flattenSections,
  localItems,
  mergeItems,
  rankItems,
  remoteItems,
  type SwitcherItem,
} from "./ranking.ts";
import { readRecents, recordRecent } from "./recents.ts";
import { useCatalogue } from "./use-catalogue.ts";
import "./switcher.css";

interface SwitcherDialogProps {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

function ItemGlyph({ item }: { readonly item: SwitcherItem }) {
  if (item.kind === "person" && item.userId !== null) {
    return <UserAvatar userId={item.userId} size={20} presence decorative />;
  }

  if (item.kind === "thread") {
    return <Icon name="thread" size={16} />;
  }

  if (item.roomKind === "direct") {
    return <GroupAvatars ids={item.memberIds} size={20} />;
  }

  return <Icon name={ROOM_KIND_ICON[item.roomKind ?? "open"]} size={16} />;
}

/** The label with the typed text marked, when it appears as one run (accents folded). */
function Highlighted({ text, query }: { readonly text: string; readonly query: string }) {
  const range = matchRange(text, normalizeQuery(query));

  if (range === null) {
    return text;
  }

  const [start, end] = range;

  return (
    <>
      {text.slice(0, start)}
      <mark className="switcher-mark">{text.slice(start, end)}</mark>
      {text.slice(end)}
    </>
  );
}

function itemDetail(item: SwitcherItem): string | null {
  if (item.kind === "thread") {
    return item.detail === null ? "Thread" : `in #${item.detail}`;
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
      data-unread={(item.unread && !item.muted) || undefined}
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
        {busy ? <Spinner label="Opening" /> : null}
        {!busy && item.count > 0 ? (
          <Badge count={item.count} label={`${item.count} unread`} />
        ) : null}
        {!busy && item.count === 0 && item.unread && !item.muted ? (
          <span className="switcher-unread-dot" aria-label="Unread" role="img" />
        ) : null}
        <Kbd keys={["⏎"]} className="switcher-enter" />
      </span>
    </div>
  );
}

/**
 * The quick switcher (⌘K): type to jump to any room, person or recent thread. The sidebar's
 * rooms answer the first keystroke; the server's catalogue joins as soon as it arrives. With no
 * query it opens on your recent picks and unread conversations. ↑/↓ move, Enter opens, Esc
 * closes; picking someone you have no DM with yet opens one.
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
  const { catalogue, loading } = useCatalogue(open);

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
    catalogue === null ? [] : remoteItems(catalogue),
    (userId) => users[userId]?.name,
  );

  const sections = rankItems(items, query, recents);
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

    recordRecent(item.key);

    if (item.roomId !== null) {
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

  const trimmed = query.trim();

  return (
    <Dialog open={open} onOpenChange={onOpenChange} title="Jump to a conversation" dirty={false}>
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
        <footer className="switcher-footer" aria-hidden="true">
          <span>
            <Kbd keys={["↑", "↓"]} /> to move
          </span>
          <span>
            <Kbd keys={["⏎"]} /> to open
          </span>
          <span>
            <Kbd keys={["Esc"]} /> to close
          </span>
        </footer>
      </div>
    </Dialog>
  );
}
