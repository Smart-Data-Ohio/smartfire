import { useNavigate } from "@tanstack/react-router";
import { type KeyboardEvent, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { playClearDissolve } from "../../motion/clear-dissolve.ts";
import { usePresence } from "../../motion/presence.ts";
import { useRecentSearches } from "../../store/search-hooks.ts";
import { directs } from "../../sync/directs.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Kbd } from "../../ui/kbd.tsx";
import { toast } from "../../ui/toast-store.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { ROOM_KIND_ICON } from "../room/room-icon.ts";
import {
  flattenTypeahead,
  type TypeaheadItem,
  type TypeaheadSection,
  typeaheadSections,
} from "./typeahead.ts";
import { useSuggestible } from "./use-suggestible.ts";
import "./search-box.css";

function ItemGlyph({ item }: { readonly item: TypeaheadItem }) {
  if (item.userId !== null) {
    return <UserAvatar userId={item.userId} size={20} decorative />;
  }

  if (item.roomKind !== null) {
    return <Icon name={ROOM_KIND_ICON[item.roomKind]} size={16} />;
  }

  return item.icon === null ? null : <Icon name={item.icon} size={16} />;
}

interface DropdownProps {
  readonly open: boolean;
  readonly listId: string;
  readonly sections: readonly TypeaheadSection[];
  readonly active: TypeaheadItem | undefined;
  readonly optionId: (item: TypeaheadItem) => string;
  readonly showClear: boolean;
  readonly onHover: (item: TypeaheadItem) => void;
  readonly onChoose: (item: TypeaheadItem) => void;
  readonly onClearRecents: () => void;
}

/**
 * The suggestions under the field: a listbox the input drives (focus stays in the input, which
 * points at the highlighted row with aria-activedescendant). It drops from the field's corner on
 * the menu-dropdown recipe and keeps its last rows on screen while it exits.
 */
function Dropdown({
  open,
  listId,
  sections,
  active,
  optionId,
  showClear,
  onHover,
  onChoose,
  onClearRecents,
}: DropdownProps) {
  const presence = usePresence<HTMLDivElement>(open);
  const [shown, setShown] = useState(sections);

  if (open && shown !== sections) {
    setShown(sections);
  }

  if (!presence.mounted) {
    return null;
  }

  const rows = open ? sections : shown;

  return (
    <div
      ref={presence.ref}
      className="search-dropdown t-dropdown"
      data-state={presence.state}
      data-origin="top-left"
    >
      <div id={listId} role="listbox" aria-label="Suggestions" className="search-dropdown-list">
        {rows.map((section) => (
          // biome-ignore lint/a11y/useSemanticElements: ARIA listbox grouping, as in the switcher.
          <div
            key={section.key}
            role="group"
            aria-labelledby={section.title === null ? undefined : `${listId}-${section.key}`}
            aria-label={section.title === null ? "Search" : undefined}
            className="search-dropdown-group"
          >
            {section.title === null ? null : (
              <div id={`${listId}-${section.key}`} className="search-dropdown-title">
                {section.title}
              </div>
            )}
            {section.items.map((item) => (
              // biome-ignore lint/a11y/useKeyWithClickEvents: the input owns the keyboard (aria-activedescendant); a click only picks
              <div
                key={item.key}
                id={optionId(item)}
                role="option"
                tabIndex={-1}
                aria-selected={item === active}
                className="search-option"
                data-kind={section.key}
                onPointerMove={item === active ? undefined : () => onHover(item)}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => onChoose(item)}
              >
                <span className="search-option-glyph">
                  <ItemGlyph item={item} />
                </span>
                <span className="search-option-label">
                  {section.key === "query" ? (
                    <>
                      <span className="search-option-faint">Search for </span>
                      <strong>{item.label}</strong>
                    </>
                  ) : (
                    item.label
                  )}
                </span>
                {item.detail === null ? null : (
                  <span className="search-option-detail">{item.detail}</span>
                )}
                <Kbd keys={["⏎"]} className="search-option-enter" />
              </div>
            ))}
          </div>
        ))}
      </div>
      <footer className="search-dropdown-footer">
        <span className="search-dropdown-keys" aria-hidden="true">
          <Kbd keys={["↑", "↓"]} /> to move <Kbd keys={["⏎"]} /> to search <Kbd keys={["Esc"]} /> to
          close
        </span>
        {showClear ? (
          <Button
            variant="link"
            size="sm"
            className="search-dropdown-clear"
            onMouseDown={(event) => event.preventDefault()}
            onClick={onClearRecents}
          >
            Clear recent searches
          </Button>
        ) : null}
      </footer>
    </div>
  );
}

export interface SearchBoxProps {
  readonly value: string;
  readonly onValueChange: (value: string) => void;
  /** Runs a search: the field's text, a recent search or a finished suggestion. */
  readonly onSearch: (query: string) => void;
  /** `page`: the search page's wide field. `header`: the room header's compact one. */
  readonly variant: "page" | "header";
  readonly placeholder: string;
  readonly autoFocus?: boolean;
  /** ↓ with the suggestions closed (the page moves into its results). */
  readonly onArrowOut?: () => void;
}

/**
 * The search field, Slack's: a combobox whose suggestions (recent searches, filters, people and
 * channels) follow what's typed. ↑/↓ move through them, Enter runs the highlighted one (or the
 * typed text), Esc closes them, and the × clears with the input-clear-dissolve recipe.
 */
export function SearchBox({
  value,
  onValueChange,
  onSearch,
  variant,
  placeholder,
  autoFocus = false,
  onArrowOut,
}: SearchBoxProps) {
  const id = useId();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [now] = useState(() => Date.now());
  const recents = useRecentSearches();
  const items = useSuggestible(open);
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const ownInput = useRef<HTMLInputElement | null>(null);
  const mirrorRef = useRef<HTMLDivElement | null>(null);
  const placeholderRef = useRef<HTMLDivElement | null>(null);
  const glowRef = useRef<HTMLDivElement | null>(null);
  const stopClear = useRef<(() => void) | null>(null);

  const sections = typeaheadSections(value, {
    recents: recents.searches,
    items,
    now,
  });

  const flat = flattenTypeahead(sections);
  const active = activeIndex < 0 ? undefined : flat[Math.min(activeIndex, flat.length - 1)];
  const listId = `${id}-list`;
  // By position: an item's key can hold the typed text (spaces and all), which isn't an id.
  const optionId = (item: TypeaheadItem) => `${id}-option-${flat.indexOf(item)}`;
  // The page lists recent searches and examples itself while its field is empty.
  const idle = variant === "page" && value.trim() === "";
  const showing = open && !idle && flat.length > 0;
  const showClear = value.trim() === "" && recents.searches.length > 0;

  const activeId = active === undefined ? undefined : optionId(active);

  useLayoutEffect(() => {
    if (activeId !== undefined) {
      document.getElementById(activeId)?.scrollIntoView({ block: "nearest" });
    }
  }, [activeId]);

  useEffect(() => () => stopClear.current?.(), []);

  // The search page exists to type into. Focus after the frame the route's view switches in (a
  // phone shows the page in place of the list, which is hidden until then), not in the commit.
  useEffect(() => {
    if (!autoFocus) {
      return;
    }

    const frame = requestAnimationFrame(() => ownInput.current?.focus({ preventScroll: true }));

    return () => cancelAnimationFrame(frame);
  }, [autoFocus]);

  const close = () => {
    setOpen(false);
    setActiveIndex(-1);
  };

  const openRoom = (roomId: number) => {
    close();
    ownInput.current?.blur();
    void navigate({ to: "/r/$roomId", params: { roomId } });
  };

  const choose = (item: TypeaheadItem) => {
    const { action } = item;

    if (action.kind === "search") {
      close();
      onSearch(action.query);

      return;
    }

    if (action.kind === "complete") {
      onValueChange(action.value);
      setActiveIndex(-1);
      setOpen(true);
      ownInput.current?.focus();

      return;
    }

    if (action.roomId !== null) {
      openRoom(action.roomId);

      return;
    }

    if (action.userId === null) {
      return;
    }

    directs.create([action.userId]).then(
      (row) => openRoom(row.room.id),
      (error: Error) =>
        toast({
          title: "Couldn't open the conversation",
          description: error.message,
          tone: "danger",
        }),
    );
  };

  const clear = () => {
    const wrap = wrapRef.current;
    const input = ownInput.current;
    const mirror = mirrorRef.current;
    const fallback = placeholderRef.current;
    const glow = glowRef.current;
    const held = value;

    onValueChange("");
    setActiveIndex(-1);
    input?.focus();
    stopClear.current?.();

    if (wrap !== null && input !== null && mirror !== null && fallback !== null && glow !== null) {
      stopClear.current = playClearDissolve(
        { wrap, input, mirror, placeholder: fallback, glow },
        held,
      );
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    const count = flat.length;

    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      if (!showing) {
        if (event.key === "ArrowDown" && onArrowOut !== undefined && value.trim() !== "") {
          event.preventDefault();
          onArrowOut();

          return;
        }

        event.preventDefault();
        setOpen(true);

        return;
      }

      event.preventDefault();

      if (count > 0) {
        const step = event.key === "ArrowDown" ? 1 : -1;
        const from = activeIndex < 0 ? (step === 1 ? -1 : count) : activeIndex;

        setActiveIndex((from + step + count) % count);
      }

      return;
    }

    if (event.key === "Enter" && !event.nativeEvent.isComposing) {
      event.preventDefault();

      if (showing && active !== undefined) {
        choose(active);

        return;
      }

      if (value.trim() !== "") {
        close();
        onSearch(value);
      }

      return;
    }

    if (event.key === "Escape") {
      if (showing) {
        event.preventDefault();
        event.stopPropagation();
        close();

        return;
      }

      if (variant === "header") {
        event.currentTarget.blur();
      }

      return;
    }

    if (event.key === "Tab") {
      close();
    }
  };

  const hasValue = value !== "";

  return (
    <div className="search-box" data-variant={variant} data-open={showing || undefined}>
      <div ref={wrapRef} className={`search-field t-clear${hasValue ? " has-value" : ""}`}>
        <Icon name="search" size={variant === "page" ? 18 : 15} className="search-field-icon" />
        <input
          ref={ownInput}
          className="search-input"
          type="search"
          role="combobox"
          aria-label="Search messages"
          aria-expanded={showing}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={showing ? activeId : undefined}
          aria-keyshortcuts="Meta+Shift+F Control+Shift+F /"
          autoComplete="off"
          spellCheck={false}
          enterKeyHint="search"
          data-search-input
          value={value}
          onFocus={() => setOpen(true)}
          onBlur={close}
          onChange={(event) => {
            onValueChange(event.target.value);
            setActiveIndex(-1);
            setOpen(true);
          }}
          onKeyDown={onKeyDown}
        />
        <div ref={mirrorRef} className="t-clear-mirror search-field-text" aria-hidden="true" />
        <div
          ref={placeholderRef}
          className="t-clear-placeholder search-field-text"
          aria-hidden="true"
        >
          {placeholder}
        </div>
        <div ref={glowRef} className="t-clear-glow" aria-hidden="true" />
        {hasValue ? (
          <IconButton
            icon="x"
            label="Clear search"
            size="sm"
            className="search-clear"
            tooltipPlacement="bottom"
            onMouseDown={(event) => event.preventDefault()}
            onClick={clear}
          />
        ) : null}
      </div>
      <Dropdown
        open={showing}
        listId={listId}
        sections={sections}
        active={active}
        optionId={optionId}
        showClear={showClear}
        onHover={(item) => setActiveIndex(flat.indexOf(item))}
        onChoose={choose}
        onClearRecents={() => {
          // The button leaves with the recents; keep focus in the field rather than on the body.
          ownInput.current?.focus({ preventScroll: true });
          actions.search.clearRecents().catch((error: Error) =>
            toast({
              title: "Couldn't clear your recent searches",
              description: error.message,
              tone: "danger",
            }),
          );
        }}
      />
    </div>
  );
}
