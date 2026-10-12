import {
  type KeyboardEvent,
  useEffect,
  useEffectEvent,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type { GifCategory } from "../../gen/GifCategory.ts";
import type { GifItem } from "../../gen/GifItem.ts";
import { EmojiImage } from "../../lib/emoji/emoji-image.tsx";
import { useReducedMotion } from "../../motion/reduced-motion.ts";
import { gifActions } from "../../sync/gif-actions.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Popover } from "../../ui/popover.tsx";
import { toast } from "../../ui/toast-store.ts";
import {
  distributeGifs,
  gifAspectRatio,
  loadGifConfig,
  moveGifFocus,
  useGifEnabled,
} from "./gif-store.ts";
import "./gif-picker.css";

/** How long typing settles before a search (and its autocomplete) goes out. */
export const GIF_SEARCH_DEBOUNCE_MS = 250;

interface GifPickerProps {
  /** Called with the chosen GIF; the caller sends it. A rejection keeps the picker open. */
  readonly onPick: (item: GifItem) => Promise<void>;
}

/** One page of GIFs for the settled query: trending for a blank one, a search otherwise. */
function fetchPage(query: string, page: number) {
  return query === "" ? gifActions.trending(page) : gifActions.search(query, page);
}

/**
 * The GIF grid, the sticker picker's sibling: a search box that drives it from the keyboard
 * (arrows move through the masonry columns, Enter sends), trending GIFs and category tiles to
 * start, debounced search with autocomplete terms and endless paging after that. A GIF rests on
 * its still until hovered, focused or active, and always under reduced motion (`EmojiImage`).
 * A pick is sent by the caller while the picker stays up, so a failed send can keep it open.
 */
export function GifPicker({ onPick }: GifPickerProps) {
  const id = useId();
  const reducedMotion = useReducedMotion();
  const [query, setQuery] = useState("");
  const [settled, setSettled] = useState("");
  const [items, setItems] = useState<readonly GifItem[]>([]);
  const [page, setPage] = useState(1);
  const [hasNext, setHasNext] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  const [moreFailed, setMoreFailed] = useState(false);
  const [categories, setCategories] = useState<readonly GifCategory[]>([]);
  const [suggestions, setSuggestions] = useState<readonly string[]>([]);
  const [activeSlug, setActiveSlug] = useState<string | null>(null);
  const [sendingSlug, setSendingSlug] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);
  const sequence = useRef(0);
  const keyboardMove = useRef(false);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const columns = distributeGifs(items);
  const active = items.find((item) => item.slug === activeSlug) ?? items[0];
  const idFor = (slug: string) => `${id}-gif-${slug}`;

  // Typing settles before it searches: one request for a run of keystrokes, not one a letter.
  useEffect(() => {
    const timer = setTimeout(() => setSettled(query.trim()), GIF_SEARCH_DEBOUNCE_MS);

    return () => clearTimeout(timer);
  }, [query]);

  // The category tiles for the opening view; a failure just leaves trending on its own.
  useEffect(() => {
    let live = true;

    gifActions.categories().then(
      (result) => {
        if (live) {
          setCategories(result.categories);
        }
      },
      () => undefined,
    );

    return () => {
      live = false;
    };
  }, []);

  // Terms under the field while searching; stale answers (a newer keystroke's) are dropped.
  useEffect(() => {
    if (settled === "") {
      setSuggestions([]);

      return;
    }

    let live = true;

    gifActions.autocomplete(settled).then(
      (result) => {
        if (live) {
          setSuggestions(result.suggestions.filter((term) => term !== settled));
        }
      },
      () => undefined,
    );

    return () => {
      live = false;
    };
  }, [settled]);

  // The first page for the settled query (trending when it is blank): the grid starts over.
  // biome-ignore lint/correctness/useExhaustiveDependencies: retryCount is the Retry button's retrigger, read for nothing else
  useEffect(() => {
    sequence.current += 1;

    const mine = sequence.current;

    setLoading(true);
    // A page still out for the old query is dropped by sequence, so its "loading more"
    // belongs to the old query too: clear it here, or paging stays stuck off.
    setLoadingMore(false);
    setFailed(null);
    setMoreFailed(false);
    setItems([]);
    setPage(1);
    setHasNext(false);
    setActiveSlug(null);
    fetchPage(settled, 1).then(
      (result) => {
        if (sequence.current !== mine) {
          return;
        }

        setItems(result.items);
        setPage(result.page);
        setHasNext(result.hasNext);
        setActiveSlug(result.items[0]?.slug ?? null);
        setLoading(false);
      },
      (error: Error) => {
        if (sequence.current !== mine) {
          return;
        }

        setFailed(error.message);
        setLoading(false);
      },
    );
  }, [settled, retryCount]);

  const loadMore = useEffectEvent(() => {
    const mine = sequence.current;
    const next = page + 1;

    setLoadingMore(true);
    setMoreFailed(false);
    fetchPage(settled, next).then(
      (result) => {
        if (sequence.current !== mine) {
          return;
        }

        setItems((held) => {
          const seen = new Set(held.map((item) => item.slug));

          return [...held, ...result.items.filter((item) => !seen.has(item.slug))];
        });
        setPage(result.page);
        setHasNext(result.hasNext);
        setLoadingMore(false);
      },
      () => {
        if (sequence.current !== mine) {
          return;
        }

        setMoreFailed(true);
        setLoadingMore(false);
      },
    );
  });

  const armed = hasNext && !loading && !loadingMore && !moreFailed;

  // Endless paging: the sentinel after the grid asks for the next page as it nears.
  useEffect(() => {
    const node = sentinelRef.current;

    if (node === null || !armed || !("IntersectionObserver" in window)) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          loadMore();
        }
      },
      { root: scrollRef.current, rootMargin: "0px 0px 240px 0px" },
    );

    observer.observe(node);

    return () => observer.disconnect();
  }, [armed]);

  // Keep the keyboard's GIF on screen.
  useLayoutEffect(() => {
    if (!keyboardMove.current || active === undefined) {
      return;
    }

    keyboardMove.current = false;
    scrollRef.current
      ?.querySelector(`[data-gif-slug="${active.slug}"]`)
      ?.scrollIntoView?.({ block: "nearest" });
  });

  const pick = async (item: GifItem) => {
    if (sendingSlug !== null) {
      return;
    }

    setSendingSlug(item.slug);

    try {
      await onPick(item);
    } catch (error) {
      toast({
        title: "Couldn't send that GIF",
        description:
          error instanceof Error ? error.message : "Check your connection and try again.",
        tone: "danger",
      });
    } finally {
      setSendingSlug(null);
    }
  };

  const searchFor = (term: string) => {
    setQuery(term);

    if (scrollRef.current !== null) {
      scrollRef.current.scrollTop = 0;
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    // An input method's keys are its own, as in the sticker picker.
    if (event.nativeEvent.isComposing) {
      event.stopPropagation();

      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();

      if (active !== undefined) {
        void pick(active);
      }

      return;
    }

    if (active === undefined) {
      return;
    }

    const next = moveGifFocus(columns, active.slug, event.key);

    if (next !== null) {
      event.preventDefault();
      keyboardMove.current = true;
      setActiveSlug(next);
    }
  };

  return (
    <div className="gif-picker">
      <div className="gif-picker-search">
        <Icon name="search" size={14} className="gif-picker-search-icon" />
        <input
          className="input gif-picker-input"
          type="search"
          role="combobox"
          aria-label="Search GIFs"
          aria-expanded="true"
          aria-controls={`${id}-grid`}
          aria-activedescendant={active === undefined ? undefined : idFor(active.slug)}
          aria-autocomplete="list"
          placeholder="Search GIFs"
          autoComplete="off"
          spellCheck={false}
          value={query}
          data-autofocus
          onChange={(event) => searchFor(event.target.value)}
          onKeyDown={onKeyDown}
        />
      </div>
      {suggestions.length === 0 ? null : (
        <div className="gif-picker-suggestions">
          {suggestions.map((term) => (
            <button
              key={term}
              type="button"
              className="gif-picker-chip"
              onClick={() => searchFor(term)}
            >
              {term}
            </button>
          ))}
        </div>
      )}
      <div ref={scrollRef} className="gif-picker-scroll">
        {loading ? (
          <div className="gif-picker-columns" aria-busy="true">
            <span className="visually-hidden" role="status">
              Loading GIFs…
            </span>
            {[0, 1].map((column) => (
              <div key={column} className="gif-picker-column" aria-hidden="true">
                {[0, 1, 2].map((cell) => (
                  <div
                    key={cell}
                    className="gif-picker-skeleton"
                    data-tall={cell === column || undefined}
                  />
                ))}
              </div>
            ))}
          </div>
        ) : failed !== null ? (
          <div className="gif-picker-status" role="alert">
            <p>Couldn't load GIFs. {failed}</p>
            <button
              type="button"
              className="gif-picker-retry"
              onClick={() => setRetryCount((count) => count + 1)}
            >
              Retry
            </button>
          </div>
        ) : (
          <>
            {settled !== "" || categories.length === 0 ? null : (
              <div className="gif-picker-categories">
                {categories.map((category) => (
                  <button
                    key={category.query}
                    type="button"
                    className="gif-picker-category"
                    onClick={() => searchFor(category.query)}
                  >
                    {category.previewUrl === null ? null : (
                      <img
                        className="gif-picker-category-image"
                        src={category.previewUrl}
                        alt=""
                        loading="lazy"
                        draggable={false}
                      />
                    )}
                    <span className="gif-picker-category-name">{category.name}</span>
                  </button>
                ))}
              </div>
            )}
            {items.length === 0 ? (
              <p className="gif-picker-status" role="status">
                No GIFs found
              </p>
            ) : (
              <div
                id={`${id}-grid`}
                className="gif-picker-columns"
                role="listbox"
                aria-label={settled === "" ? "Trending GIFs" : "Search results"}
              >
                {columns.map((column) => (
                  <div key={column[0]?.slug ?? "empty"} className="gif-picker-column">
                    {column.map((item) => (
                      // biome-ignore lint/a11y/useKeyWithClickEvents: the search box drives the grid from the keyboard (aria-activedescendant)
                      <div
                        key={item.slug}
                        id={idFor(item.slug)}
                        role="option"
                        tabIndex={-1}
                        aria-selected={item.slug === active?.slug}
                        aria-label={item.title}
                        aria-disabled={sendingSlug !== null || undefined}
                        className="gif-picker-cell"
                        data-gif-slug={item.slug}
                        data-active={item.slug === active?.slug || undefined}
                        onPointerMove={() => setActiveSlug(item.slug)}
                        onClick={() => void pick(item)}
                      >
                        {item.stillUrl === null && (reducedMotion || item.slug !== active?.slug) ? (
                          // No still to rest on: a titled placeholder, so the animated
                          // preview never plays at rest or under reduced motion. It
                          // animates only as the active GIF when motion is allowed.
                          <span
                            className="gif-picker-placeholder"
                            style={{ aspectRatio: gifAspectRatio(item) }}
                          >
                            <span className="gif-picker-placeholder-title">{item.title}</span>
                          </span>
                        ) : (
                          <EmojiImage
                            className="gif-picker-image"
                            src={item.previewUrl}
                            still={item.stillUrl}
                            resting={item.slug !== active?.slug}
                            alt={item.title}
                            title={item.attribution}
                            width={item.width > 0 ? item.width : undefined}
                            height={item.height > 0 ? item.height : undefined}
                            style={{ aspectRatio: gifAspectRatio(item) }}
                            loading="lazy"
                            draggable={false}
                          />
                        )}
                      </div>
                    ))}
                  </div>
                ))}
              </div>
            )}
            {loadingMore ? (
              <p className="gif-picker-more" role="status">
                Loading more GIFs…
              </p>
            ) : null}
            {moreFailed ? (
              <div className="gif-picker-more" role="alert">
                Couldn't load more GIFs.{" "}
                <button type="button" className="gif-picker-retry" onClick={() => loadMore()}>
                  Retry
                </button>
              </div>
            ) : null}
            {hasNext ? <div ref={sentinelRef} className="gif-picker-sentinel" /> : null}
          </>
        )}
      </div>
      <footer className="gif-picker-footer">
        {active === undefined ? null : (
          <span className="gif-picker-active-title">{active.title}</span>
        )}
        <span className="gif-picker-attribution">Powered by KLIPY</span>
      </footer>
    </div>
  );
}

/**
 * The composer's GIF button: shown once the server says GIFs are on (never when they are off),
 * it opens the picker above the toolbar (a bottom sheet on a touch phone), and a pick is sent
 * at once. The picker closes only once the send lands; a failure toasts and leaves it open.
 */
export function ComposerGifButton({ onPick }: GifPickerProps) {
  const enabled = useGifEnabled();

  useEffect(() => {
    void loadGifConfig().catch(() => undefined);
  }, []);

  if (!enabled) {
    return null;
  }

  return (
    <Popover
      label="Send a GIF"
      placement="top-start"
      trigger={(props) => (
        <IconButton
          {...props}
          icon="gif"
          label="GIF"
          size="sm"
          onMouseDown={(event) => event.preventDefault()}
        />
      )}
    >
      {(close) => (
        <GifPicker
          onPick={async (item) => {
            await onPick(item);
            close();
          }}
        />
      )}
    </Popover>
  );
}
