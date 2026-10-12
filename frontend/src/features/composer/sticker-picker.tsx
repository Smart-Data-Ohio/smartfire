import { type KeyboardEvent, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { Sticker } from "../../gen/Sticker.ts";
import { EmojiImage } from "../../lib/emoji/emoji-image.tsx";
import { moveActive } from "../../lib/emoji/picker-layout.ts";
import { useActionSheet } from "../../ui/action-sheet.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Popover } from "../../ui/popover.tsx";
import { loadStickerCatalog, searchStickers, useHeldStickers } from "./sticker-store.ts";
import "./sticker-picker.css";

/** Stickers per grid row, in a popover and in a phone's sheet alike. */
export const STICKER_COLUMNS = 4;

interface StickerPickerProps {
  /** Called with the chosen sticker; the caller sends it. */
  readonly onPick: (sticker: Sticker) => void;
}

/**
 * The sticker grid: a search box (name and tags) that drives it from the keyboard, as the emoji
 * picker's does (arrows move, Enter picks), and a footer naming the sticker under the pointer or
 * the keyboard. Animated stickers rest on their first frame until active, and always under
 * reduced motion. Opening it refetches the catalog, so another administrator's changes show.
 */
export function StickerPicker({ onPick }: StickerPickerProps) {
  const id = useId();
  const all = useHeldStickers();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const keyboardMove = useRef(false);
  const gridRef = useRef<HTMLDivElement | null>(null);
  const sheet = useActionSheet();
  const shown = searchStickers(all, query);
  const current = Math.min(activeIndex, shown.length - 1);
  const active = shown[current];
  const idFor = (index: number) => `${id}-sticker-${index}`;

  useEffect(() => {
    void loadStickerCatalog(true).catch(() => undefined);
  }, []);

  // Keep the keyboard's sticker on screen.
  useLayoutEffect(() => {
    if (!keyboardMove.current || active === undefined) {
      return;
    }

    keyboardMove.current = false;
    gridRef.current
      ?.querySelector(`[data-sticker-id="${active.id}"]`)
      ?.scrollIntoView?.({ block: "nearest" });
  });

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();

      if (active !== undefined) {
        onPick(active);
      }

      return;
    }

    const next = moveActive(current, event.key, shown.length, STICKER_COLUMNS);

    if (next !== null) {
      event.preventDefault();
      keyboardMove.current = true;
      setActiveIndex(next);
    }
  };

  return (
    <div className="sticker-picker">
      <div className="sticker-picker-search">
        <Icon name="search" size={14} className="sticker-picker-search-icon" />
        <input
          className="input sticker-picker-input"
          type="search"
          role="combobox"
          aria-label="Search stickers"
          aria-expanded="true"
          aria-controls={`${id}-grid`}
          aria-activedescendant={active === undefined ? undefined : idFor(current)}
          aria-autocomplete="list"
          placeholder="Search stickers"
          autoComplete="off"
          spellCheck={false}
          value={query}
          data-autofocus
          onChange={(event) => {
            setQuery(event.target.value);
            setActiveIndex(0);

            if (gridRef.current !== null) {
              gridRef.current.scrollTop = 0;
            }
          }}
          onKeyDown={onKeyDown}
        />
      </div>
      {shown.length === 0 ? (
        <p className="sticker-picker-status" role="status">
          {all.length === 0 ? "No stickers yet" : `No stickers match “${query.trim()}”`}
        </p>
      ) : (
        <div
          ref={gridRef}
          id={`${id}-grid`}
          className="sticker-picker-grid"
          role="listbox"
          aria-label={query.trim() === "" ? "Stickers" : "Search results"}
        >
          {shown.map((sticker, index) => (
            // biome-ignore lint/a11y/useKeyWithClickEvents: the search box drives the grid from the keyboard (aria-activedescendant)
            <div
              key={sticker.id}
              id={idFor(index)}
              role="option"
              tabIndex={-1}
              aria-selected={index === current}
              aria-label={sticker.name}
              className="sticker-picker-cell"
              data-sticker-id={sticker.id}
              data-active={index === current || undefined}
              onPointerMove={() => setActiveIndex(index)}
              onClick={() => onPick(sticker)}
            >
              <EmojiImage
                className="sticker-picker-image"
                src={sticker.imageUrl}
                still={sticker.stillUrl}
                resting={index !== current}
                width={72}
                height={72}
                loading="lazy"
                draggable={false}
              />
            </div>
          ))}
        </div>
      )}
      {sheet ? null : (
        <footer className="sticker-picker-preview" aria-hidden="true">
          {active === undefined ? null : (
            <>
              <span className="sticker-picker-preview-name">{active.name}</span>
              {active.description === "" ? null : (
                <span className="sticker-picker-preview-text">{active.description}</span>
              )}
            </>
          )}
        </footer>
      )}
    </div>
  );
}

/**
 * The composer's sticker button: shown once the workspace has stickers, it opens the picker above
 * the toolbar (a bottom sheet on a touch phone), and a pick is sent at once as its own message.
 */
export function ComposerStickerButton({ onPick }: StickerPickerProps) {
  const count = useHeldStickers().length;

  useEffect(() => {
    void loadStickerCatalog().catch(() => undefined);
  }, []);

  if (count === 0) {
    return null;
  }

  return (
    <Popover
      label="Send a sticker"
      placement="top-start"
      trigger={(props) => (
        <IconButton
          {...props}
          icon="sticker"
          label="Sticker"
          size="sm"
          onMouseDown={(event) => event.preventDefault()}
        />
      )}
    >
      {(close) => (
        <StickerPicker
          onPick={(sticker) => {
            // Close first: the popover hands focus back to its trigger, then the composer's text.
            close();
            onPick(sticker);
          }}
        />
      )}
    </Popover>
  );
}
