import {
  type KeyboardEvent,
  type ReactNode,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { VList, type VListHandle } from "virtua";
import { Button, Spinner } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import { type EmojiData, GROUP_LABELS, loadEmojiData } from "./data.ts";
import {
  layoutSections,
  moveActive,
  type PickerCell,
  type PickerRow,
  pickerSections,
  rowStartsOf,
  type SectionId,
} from "./picker-layout.ts";
import { type EmojiChoice, recordRecentEmoji, useRecentEmoji } from "./recent.ts";
import "./emoji-picker.css";

export interface EmojiPickerProps {
  /** Called with the chosen emoji or icon; it's already remembered as recent. */
  readonly onPick: (choice: EmojiChoice) => void;
  /** The workspace's brand and custom icons for the Custom tab (fetched once, on open). */
  readonly loadCustomIcons?: () => Promise<readonly EmojiChoice[]>;
}

const TABS: readonly { id: SectionId; label: string; glyph: string }[] = [
  { id: "recent", label: "Frequently used", glyph: "🕒" },
  ...Object.entries(GROUP_LABELS).map(([id, entry]) => ({
    // SAFETY: the keys of GROUP_LABELS are exactly the EmojiGroupId members.
    id: id as SectionId,
    label: entry.label,
    glyph: entry.glyph,
  })),
  { id: "custom", label: "Custom", glyph: "🧩" },
];

const LIST_HEIGHT = 288;

function useEmojiData(): EmojiData | null | "error" {
  const [data, setData] = useState<EmojiData | null | "error">(null);

  useEffect(() => {
    let live = true;

    loadEmojiData().then(
      (loaded) => {
        if (live) setData(loaded);
      },
      () => {
        if (live) setData("error");
      },
    );

    return () => {
      live = false;
    };
  }, []);

  return data;
}

function useCustomIcons(load: EmojiPickerProps["loadCustomIcons"]): readonly EmojiChoice[] {
  const [icons, setIcons] = useState<readonly EmojiChoice[]>([]);

  useEffect(() => {
    let live = true;

    load?.().then(
      (list) => {
        if (live) setIcons(list);
      },
      () => undefined,
    );

    return () => {
      live = false;
    };
  }, [load]);

  return icons;
}

function Glyph({ choice, size }: { readonly choice: EmojiChoice; readonly size: "cell" | "big" }) {
  return choice.imageUrl === null ? (
    <span className={`emoji-glyph emoji-glyph-${size}`}>{choice.content}</span>
  ) : (
    <img
      className={`emoji-image emoji-image-${size}`}
      src={choice.imageUrl}
      alt=""
      loading="lazy"
      draggable={false}
    />
  );
}

interface GridRowProps {
  readonly row: PickerRow;
  readonly activeIndex: number;
  readonly idFor: (index: number) => string;
  readonly onHover: (index: number) => void;
  readonly onPick: (cell: PickerCell) => void;
}

function GridRow({ row, activeIndex, idFor, onHover, onPick }: GridRowProps) {
  if (row.kind === "header") {
    return (
      <div className="emoji-picker-section" role="presentation">
        {row.label}
      </div>
    );
  }

  return (
    <div className="emoji-picker-row" role="presentation">
      {row.cells.map((cell) => (
        // biome-ignore lint/a11y/useKeyWithClickEvents: the search box drives the grid from the keyboard (aria-activedescendant)
        <div
          key={cell.index}
          id={idFor(cell.index)}
          role="option"
          tabIndex={-1}
          aria-selected={cell.index === activeIndex}
          aria-label={cell.choice.title}
          className="emoji-picker-cell"
          data-active={cell.index === activeIndex || undefined}
          onPointerMove={() => onHover(cell.index)}
          onClick={() => onPick(cell)}
        >
          <Glyph choice={cell.choice} size="cell" />
        </div>
      ))}
    </div>
  );
}

function Preview({ cell }: { readonly cell: PickerCell | undefined }) {
  return (
    <footer className="emoji-picker-preview" aria-hidden="true">
      {cell === undefined ? (
        <span className="emoji-picker-preview-empty">Pick an emoji</span>
      ) : (
        <>
          <Glyph choice={cell.choice} size="big" />
          <span className="emoji-picker-preview-text">
            <span className="emoji-picker-preview-title">{cell.choice.title}</span>
            {cell.shortcode === null ? null : (
              <span className="emoji-picker-preview-code">:{cell.shortcode}:</span>
            )}
          </span>
        </>
      )}
    </footer>
  );
}

function Body({ children }: { readonly children: ReactNode }) {
  return (
    <div className="emoji-picker-status" style={{ height: LIST_HEIGHT }}>
      {children}
    </div>
  );
}

/**
 * The emoji picker: a search box that drives the grid (arrows move, Enter picks, as in Slack and
 * Discord), category tabs that jump to their section and follow the scroll, a Frequently used
 * row from this browser's picks, the workspace's custom icons, and a preview footer. The grid is
 * virtualized, so ~1,900 emoji cost a screenful of nodes. Lazy-load it (`LazyEmojiPicker`).
 */
export default function EmojiPicker({ onPick, loadCustomIcons }: EmojiPickerProps) {
  const id = useId();
  const data = useEmojiData();
  const recent = useRecentEmoji();
  const custom = useCustomIcons(loadCustomIcons);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const [section, setSection] = useState<SectionId | null>(null);
  const listRef = useRef<VListHandle | null>(null);
  const keyboardMove = useRef(false);
  const inputRef = useRef<HTMLInputElement | null>(null);

  // Mounting late (the chunk loaded after the popover opened) still lands in the search box,
  // unless focus has already moved on to something else in the page.
  useEffect(() => {
    const input = inputRef.current;
    const active = document.activeElement;

    if (
      input !== null &&
      (active === null ||
        active === document.body ||
        active.contains(input) ||
        input.closest(".popover")?.contains(active) === true)
    ) {
      input.focus({ preventScroll: true });
    }
  }, []);

  const layout =
    data === null || data === "error"
      ? null
      : layoutSections(pickerSections(data, query, recent, custom));

  const cells = layout?.cells ?? [];
  const rows = layout?.rows ?? [];
  const active = cells[Math.min(activeIndex, cells.length - 1)];
  const idFor = (index: number) => `${id}-cell-${index}`;
  const visibleTabs = TABS.filter((tab) => layout?.sectionRows.has(tab.id) ?? false);
  const searching = query.trim() !== "";
  const currentSection = section ?? rows[0]?.section ?? null;

  // Keep the keyboard's cell on screen.
  useLayoutEffect(() => {
    if (!keyboardMove.current || layout === null) {
      return;
    }

    keyboardMove.current = false;

    const rowIndex = layout.rows.findIndex(
      (row) => row.kind === "cells" && row.cells.some((cell) => cell.index === activeIndex),
    );

    if (rowIndex >= 0) {
      listRef.current?.scrollToIndex(rowIndex, { align: "nearest" });
    }
  });

  const pick = (choice: EmojiChoice) => {
    recordRecentEmoji(choice);
    onPick(choice);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();

      if (active !== undefined) {
        pick(active.choice);
      }

      return;
    }

    const next = moveActive(activeIndex, event.key, cells.length, undefined, rowStartsOf(rows));

    if (next !== null) {
      event.preventDefault();
      keyboardMove.current = true;
      setActiveIndex(next);
    }
  };

  const jumpTo = (target: SectionId) => {
    const row = layout?.sectionRows.get(target);

    if (row === undefined) {
      return;
    }

    setSection(target);
    listRef.current?.scrollToIndex(row, { align: "start" });

    const first = layout?.rows[row + 1];

    if (first?.kind === "cells") {
      setActiveIndex(first.cells[0]?.index ?? 0);
    }
  };

  const onScroll = (offset: number) => {
    const list = listRef.current;

    if (list === null || searching) {
      return;
    }

    const row = rows[list.findItemIndex(offset + 1)];

    if (row !== undefined) {
      setSection(row.section);
    }
  };

  let body: ReactNode;

  if (data === null) {
    body = (
      <Body>
        <Spinner label="Loading emoji" />
      </Body>
    );
  } else if (data === "error") {
    body = <Body>Couldn't load emoji. Try again in a moment.</Body>;
  } else if (cells.length === 0) {
    body = (
      <Body>
        <span className="emoji-picker-empty-glyph" aria-hidden="true">
          🔍
        </span>
        No emoji match “{query.trim()}”
      </Body>
    );
  } else {
    body = (
      <VList
        ref={listRef}
        className="emoji-picker-grid"
        style={{ height: LIST_HEIGHT }}
        data={rows}
        bufferSize={160}
        onScroll={onScroll}
        id={`${id}-grid`}
        role="listbox"
        aria-label={searching ? "Search results" : "Emoji"}
      >
        {(row) => (
          <GridRow
            key={row.key}
            row={row}
            activeIndex={active?.index ?? -1}
            idFor={idFor}
            onHover={setActiveIndex}
            onPick={(cell) => pick(cell.choice)}
          />
        )}
      </VList>
    );
  }

  return (
    <div className="emoji-picker">
      <div className="emoji-picker-search">
        <Icon name="search" size={14} className="emoji-picker-search-icon" />
        <input
          ref={inputRef}
          className="input emoji-picker-input"
          type="search"
          role="combobox"
          aria-label="Search emoji"
          aria-expanded="true"
          aria-controls={`${id}-grid`}
          aria-activedescendant={active === undefined ? undefined : idFor(active.index)}
          aria-autocomplete="list"
          placeholder="Search emoji"
          autoComplete="off"
          spellCheck={false}
          value={query}
          data-autofocus
          onChange={(event) => {
            setQuery(event.target.value);
            setActiveIndex(0);
            setSection(null);
            listRef.current?.scrollToIndex(0);
          }}
          onKeyDown={onKeyDown}
        />
      </div>
      {searching ? null : (
        <div className="emoji-picker-tabs" role="toolbar" aria-label="Emoji categories">
          {visibleTabs.map((tab) => (
            <Tooltip key={tab.id} content={tab.label} describe={false} placement="bottom">
              <Button
                variant="icon"
                size="sm"
                className="emoji-picker-tab"
                aria-label={tab.label}
                aria-pressed={tab.id === currentSection}
                data-active={tab.id === currentSection || undefined}
                onClick={() => jumpTo(tab.id)}
              >
                <span aria-hidden="true">{tab.glyph}</span>
              </Button>
            </Tooltip>
          ))}
        </div>
      )}
      {body}
      <Preview cell={active} />
    </div>
  );
}
