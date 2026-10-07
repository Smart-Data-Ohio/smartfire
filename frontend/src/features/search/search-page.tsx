import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { type KeyboardEvent, useEffect, useRef, useState } from "react";
import type { SearchChip } from "../../gen/SearchChip.ts";
import { shortcutKeys } from "../../lib/shortcuts.ts";
import { searchKey } from "../../store/search.ts";
import { useRecentSearches, useSearchResults } from "../../store/search-hooks.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { Kbd } from "../../ui/kbd.tsx";
import { toast } from "../../ui/toast-store.ts";
import { useAnnouncer } from "../destinations/live-region.tsx";
import { type RowParts, useKeepRowFocus } from "../destinations/row-focus.ts";
import { useNow } from "../threads/use-now.ts";
import { chipLabel, resultsAnnouncement } from "./format.ts";
import { appendToken, textWords } from "./query.ts";
import { SearchBox } from "./search-box.tsx";
import { visibleSearchInput } from "./search-hotkey.tsx";
import { isEditable } from "./search-keys.ts";
import { SearchResults, SearchState } from "./search-results.tsx";
import { OPERATOR_HINTS } from "./typeahead.ts";
import "./search.css";

const CHIP_ICON = {
  from: "at",
  in: "hash",
  has: "paperclip",
  is: "thread",
  before: "calendar-clock",
  after: "calendar-clock",
  on: "calendar-clock",
} as const satisfies Record<SearchChip["operator"], IconName>;

/** Filters one click adds; the others put their operator in the field to pick a value. */
const TOGGLES = [
  { token: "has:file", operator: "has", value: "file", label: "Files", icon: "file" },
  { token: "has:link", operator: "has", value: "link", label: "Links", icon: "link" },
  { token: "has:pin", operator: "has", value: "pin", label: "Pinned", icon: "pin" },
  { token: "is:thread", operator: "is", value: "true", label: "In threads", icon: "thread" },
] as const satisfies readonly {
  readonly token: string;
  readonly operator: SearchChip["operator"];
  readonly value: string;
  readonly label: string;
  readonly icon: IconName;
}[];

const PICKERS = [
  { token: "from:", label: "From", icon: "at" },
  { token: "in:", label: "In", icon: "hash" },
  { token: "after:", label: "Date", icon: "calendar-clock" },
] as const satisfies readonly {
  readonly token: string;
  readonly label: string;
  readonly icon: IconName;
}[];

interface FilterBarProps {
  readonly query: string;
  readonly chips: readonly SearchChip[];
  readonly onQuery: (query: string) => void;
  readonly onCompose: (value: string) => void;
  /** Says what a chip or pill just did, in the page's live region. */
  readonly announce: (text: string) => void;
}

/**
 * The filter bar's items for `useKeepRowFocus`: a chip leaves when it's removed and a pill when its
 * chip appears, and focus moves on to the item beside it rather than dropping to the page.
 */
const FILTER_PARTS: RowParts = {
  list: ".search-filters",
  row: "[data-filter-item]",
  open: "button",
  leaving: "[data-filter-leaving]",
};

/** The query's filters as removable chips, then pills that add the common ones. */
function FilterBar({ query, chips, onQuery, onCompose, announce }: FilterBarProps) {
  const barRef = useRef<HTMLDivElement | null>(null);

  useKeepRowFocus(barRef, FILTER_PARTS);

  const toggles = TOGGLES.filter(
    (toggle) =>
      !chips.some((chip) => chip.operator === toggle.operator && chip.value === toggle.value),
  );

  return (
    <div ref={barRef} className="search-filters" role="toolbar" aria-label="Filters">
      {chips.map((chip) => (
        <span
          key={`${chip.token}:${chip.removeQuery}`}
          className="search-chip enter-pop"
          data-filter-item
        >
          <Icon name={CHIP_ICON[chip.operator]} size={14} className="search-chip-icon" />
          <span className="search-chip-label">{chipLabel(chip)}</span>
          <IconButton
            icon="x"
            label={`Remove ${chipLabel(chip)}`}
            size="sm"
            className="search-chip-remove"
            tooltipPlacement="bottom"
            onClick={() => {
              announce(`Removed the ${chipLabel(chip)} filter`);
              onQuery(chip.removeQuery);
            }}
          />
        </span>
      ))}
      {chips.length === 0 ? null : <span className="search-filters-divider" aria-hidden="true" />}
      {PICKERS.map((picker) => (
        <span key={picker.token} className="search-filter-item" data-filter-item>
          <Button
            variant="pill"
            size="sm"
            icon={picker.icon}
            trailingIcon="chevron-down"
            onClick={() => onCompose(appendToken(query, picker.token))}
          >
            {picker.label}
          </Button>
        </span>
      ))}
      {toggles.map((toggle) => (
        <span key={toggle.token} className="search-filter-item" data-filter-item>
          <Button
            variant="pill"
            size="sm"
            icon={toggle.icon}
            onClick={() => {
              announce(`Added the ${toggle.label} filter`);
              onQuery(appendToken(query, toggle.token));
            }}
          >
            {toggle.label}
          </Button>
        </span>
      ))}
    </div>
  );
}

const EXAMPLES = [
  { query: "from:@maya", operator: "from" },
  { query: "in:#launch-planning", operator: "in" },
  { query: "has:file", operator: "has" },
  { query: "is:thread", operator: "is" },
  { query: "after:2026-10-01", operator: "after" },
] as const;

/** Nothing searched yet: recent searches to run again, and how to narrow a search. */
function SearchHome({
  onQuery,
  onCompose,
  announce,
}: Pick<FilterBarProps, "onQuery" | "onCompose" | "announce">) {
  const recents = useRecentSearches();

  return (
    <div className="search-home enter-fade">
      <SearchState icon="search" title="Search messages, files and conversations">
        <p className="search-state-text">
          Press <Kbd keys={shortcutKeys("search")} /> or <Kbd keys={["/"]} /> anywhere to come back
          here.
        </p>
      </SearchState>
      {recents.searches.length === 0 ? null : (
        <section className="search-home-section" aria-label="Recent searches">
          <div className="search-home-heading">
            <h2 className="search-section-heading">Recent searches</h2>
            <Button
              variant="link"
              size="sm"
              onClick={() => {
                // The section goes with its recents: focus waits in the field instead.
                visibleSearchInput()?.focus({ preventScroll: true });
                announce("Cleared your recent searches");
                actions.search.clearRecents().catch((error: Error) =>
                  toast({
                    title: "Couldn't clear your recent searches",
                    description: error.message,
                    tone: "danger",
                  }),
                );
              }}
            >
              Clear
            </Button>
          </div>
          <ul className="search-home-list">
            {recents.searches.map((search) => (
              <li key={search.id}>
                <Button
                  variant="ghost"
                  icon="clock"
                  className="search-home-row"
                  data-search-nav
                  onClick={() => onQuery(search.query)}
                >
                  {search.query}
                </Button>
              </li>
            ))}
          </ul>
        </section>
      )}
      <section className="search-home-section" aria-label="Narrow your search">
        <h2 className="search-section-heading">Narrow your search</h2>
        <ul className="search-home-list">
          {EXAMPLES.map((example) => (
            <li key={example.query}>
              <Button
                variant="ghost"
                icon={OPERATOR_HINTS[example.operator].icon}
                className="search-home-row"
                data-search-nav
                onClick={() => onCompose(`${example.query} `)}
              >
                <code className="search-home-code">{example.query}</code>
                <span className="search-home-hint">{OPERATOR_HINTS[example.operator].label}</span>
              </Button>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}

/** Moves focus through the results' links with ↑/↓ (or k/j); ↑ from the first goes back up. */
function moveFocus(event: KeyboardEvent<HTMLElement>): void {
  const step =
    event.key === "ArrowDown" || event.key === "j"
      ? 1
      : event.key === "ArrowUp" || event.key === "k"
        ? -1
        : 0;

  if (step === 0 || event.metaKey || event.ctrlKey || event.altKey || isEditable(event.target)) {
    return;
  }

  const stops = [
    ...event.currentTarget.querySelectorAll<HTMLElement>(
      "[data-search-nav], [data-search-hit-link]",
    ),
  ];

  const active = document.activeElement;

  const index = active instanceof HTMLElement ? stops.indexOf(active) : -1;

  event.preventDefault();

  if (index + step < 0) {
    visibleSearchInput()?.focus();

    return;
  }

  const next = stops[index === -1 ? 0 : Math.min(index + step, stops.length - 1)];

  next?.focus();
  next?.scrollIntoView({ block: "nearest" });
}

/**
 * `/app/search?q=`: global search, Slack's results page. The field (with its typeahead) and the
 * filter chips stay pinned at the top; the results scroll under them. A phone shows it full
 * screen with a back button. Every query shown is remembered in your recent searches.
 */
export function SearchPage() {
  const navigate = useNavigate();
  const { q = "" } = useSearch({ from: "/shell/search" });
  const query = searchKey(q);
  const [draft, setDraft] = useState(query);
  const [shown, setShown] = useState(query);
  const now = useNow();
  const results = useSearchResults(query);
  const { announce, region } = useAnnouncer();
  const [readyChips, setReadyChips] = useState<readonly SearchChip[]>([]);

  if (shown !== query) {
    setShown(query);
    setDraft(query);
  }

  if (results.status === "ready" && results.chips !== readyChips) {
    setReadyChips(results.chips);
  }

  // While a changed query loads, its chips are the last ones shown that it still holds, so the
  // bar doesn't empty and refill (and focus has a chip beside it to land on).
  const chips =
    results.status === "ready"
      ? results.chips
      : readyChips.filter((chip) => query.includes(chip.token));

  const said = resultsAnnouncement(results);
  const saidFor = `${results.generation}:${said}`;
  const lastSaid = useRef("");

  useEffect(() => {
    if (saidFor !== lastSaid.current) {
      lastSaid.current = saidFor;

      if (said !== "") {
        announce(said);
      }
    }
  });

  useEffect(() => {
    if (query !== "") {
      void actions.search.record(query).catch(() => undefined);
    }
  }, [query]);

  const run = (next: string) => {
    const key = searchKey(next);

    setDraft(key);

    if (key === query) {
      results.reload();

      return;
    }

    void navigate({ to: "/search", search: key === "" ? {} : { q: key } });

    // The filter bar goes with the last filter: the field takes focus.
    if (key === "") {
      requestAnimationFrame(() => visibleSearchInput()?.focus());
    }
  };

  const compose = (value: string) => {
    setDraft(value);
    requestAnimationFrame(() => {
      const field = visibleSearchInput();

      field?.focus();
      field?.setSelectionRange(value.length, value.length);
    });
  };

  return (
    <section className="search-page" aria-labelledby="search-page-title">
      <h1 id="search-page-title" className="visually-hidden">
        Search
      </h1>
      <header className="search-header">
        <Link to="/" className="room-back search-back" aria-label="Back to conversations">
          <Icon name="chevron-left" size={20} />
        </Link>
        <SearchBox
          variant="page"
          value={draft}
          onValueChange={setDraft}
          onSearch={run}
          placeholder="Search messages, people and channels"
          autoFocus={query === ""}
          onArrowOut={() =>
            document
              .querySelector<HTMLElement>(
                ".search-scroll [data-search-nav], .search-scroll [data-search-hit-link]",
              )
              ?.focus()
          }
        />
        <IconButton
          icon="x"
          label="Close search"
          className="search-close"
          tooltipPlacement="bottom"
          onClick={() => void navigate({ to: "/" })}
        />
      </header>
      {query === "" ? null : (
        <FilterBar
          query={query}
          chips={chips}
          onQuery={run}
          onCompose={compose}
          announce={announce}
        />
      )}
      {/* The results list takes ↑/↓ (and j/k) between its links; it's a scroll region, not a widget. */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: arrow keys only move focus between the links inside */}
      <div className="search-scroll" tabIndex={-1} onKeyDown={moveFocus}>
        <div className="search-content">
          {query === "" ? (
            <SearchHome onQuery={run} onCompose={compose} announce={announce} />
          ) : (
            <SearchResults results={results} words={textWords(query)} now={now} onQuery={run} />
          )}
        </div>
      </div>
      {region}
    </section>
  );
}
