import { type MouseEvent, type ReactNode, useEffect, useEffectEvent, useRef } from "react";
import type { MessageDTO } from "../../store/model.ts";
import { conversationKey } from "../../store/search.ts";
import type { SearchResultsView } from "../../store/search-hooks.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { chipLabel, messageCount } from "./format.ts";
import { ConversationHeading, HitRow } from "./hit-row.tsx";
import { SearchMatches } from "./search-matches.tsx";
import { SearchSections } from "./search-sections.tsx";

/** Consecutive hits in the same room or thread, under one heading. */
interface HitGroup {
  readonly key: string;
  readonly conversation: string;
  readonly hits: readonly MessageDTO[];
}

export function groupHits(messages: readonly MessageDTO[]): HitGroup[] {
  const groups: { key: string; conversation: string; hits: MessageDTO[] }[] = [];

  for (const message of messages) {
    const conversation = conversationKey(message.roomId, message.threadId);
    const last = groups.at(-1);

    if (last !== undefined && last.conversation === conversation) {
      last.hits.push(message);
    } else {
      groups.push({ key: `${conversation}@${message.id}`, conversation, hits: [message] });
    }
  }

  return groups;
}

function HitSkeletons() {
  return (
    <div className="search-skeletons">
      <Skeleton width={120} height={12} />
      {[0.92, 0.7, 0.85, 0.6, 0.78].map((width) => (
        <div key={width} className="search-skeleton-row">
          <Skeleton width={36} height={36} radius="md" />
          <div className="search-skeleton-text">
            <Skeleton width={180} height={12} />
            <Skeleton width={`${width * 100}%`} height={12} />
          </div>
        </div>
      ))}
    </div>
  );
}

/** A centred message for a state with nothing to list. */
export function SearchState({
  icon,
  title,
  children,
  tone,
}: {
  readonly icon: "search" | "alert";
  readonly title: string;
  readonly children?: ReactNode;
  readonly tone?: "danger";
}) {
  return (
    <div className="search-state enter-pop" data-tone={tone}>
      <span className="search-state-glyph">
        <Icon name={icon} size={22} />
      </span>
      <h2 className="search-state-title">{title}</h2>
      {children}
    </div>
  );
}

/**
 * Puts focus on the results' scroll region before the control that has it gives way (a state
 * swapping to skeletons), so it doesn't drop to the page; ↑/↓ go on from there.
 */
function holdFocusInResults(event: MouseEvent<HTMLElement>): void {
  event.currentTarget.closest<HTMLElement>(".search-scroll")?.focus({ preventScroll: true });
}

/** Focus fell to the page (its control went away). */
function focusDropped(): boolean {
  const active = document.activeElement;

  return active === null || active === document.body;
}

/**
 * Loads the next page when the bottom of the list scrolls into view, or from its button. The
 * button stays one element through loading and failing (busy, then "Try again"), so focus stays
 * on it; when no page is left it goes, and focus moves to the first message it loaded.
 */
function MoreSentinel({ results }: { readonly results: SearchResultsView }) {
  const ref = useRef<HTMLDivElement | null>(null);
  const resumeAt = useRef<number | null>(null);
  const armed = results.hasMore && !results.loadingMore && results.moreError === null;
  const onVisible = useEffectEvent(() => results.loadMore());

  useEffect(() => {
    const node = ref.current;

    if (node === null || !armed || !("IntersectionObserver" in window)) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          onVisible();
        }
      },
      { root: node.closest(".search-scroll"), rootMargin: "0px 0px 480px 0px" },
    );

    observer.observe(node);

    return () => observer.disconnect();
  }, [armed]);

  useEffect(() => {
    const from = resumeAt.current;

    if (from === null || results.loadingMore) {
      return;
    }

    resumeAt.current = null;

    if (!focusDropped()) {
      return;
    }

    const links = document.querySelectorAll<HTMLElement>(".search-scroll [data-search-hit-link]");

    (links[from] ?? links[links.length - 1])?.focus({ preventScroll: true });
  }, [results.loadingMore]);

  if (results.hasMore || results.moreError !== null) {
    return (
      <div ref={ref} className="search-more">
        {results.moreError === null ? null : <span>Couldn't load more results.</span>}
        <Button
          variant="link"
          size="sm"
          loading={results.loadingMore}
          loadingLabel="Loading more messages…"
          onClick={() => {
            resumeAt.current = results.messages.length;
            results.loadMore();
          }}
        >
          {results.moreError === null ? "Load more messages" : "Try again"}
        </Button>
      </div>
    );
  }

  return results.messages.length === 0 ? null : (
    <p className="search-more search-end">That's every message that matches.</p>
  );
}

interface SearchResultsProps {
  readonly results: SearchResultsView;
  /** The free-text words, for marking hits and matching people and channels. */
  readonly words: readonly string[];
  readonly now: number;
  /** Runs another query (a chip's removal from the empty state). */
  readonly onQuery: (query: string) => void;
}

/**
 * The results under the search field: skeletons while the first page loads, then the sections
 * (board posts, work threads, events), people and channels that match, and the messages grouped
 * by conversation, with older pages loading as you scroll. Empty and error states say what to
 * try next.
 */
export function SearchResults({ results, words, now, onQuery }: SearchResultsProps) {
  if (results.status === "error") {
    return (
      <SearchState icon="alert" title="Search isn't working right now" tone="danger">
        <p className="search-state-text">{results.error ?? "Something went wrong."}</p>
        <Button
          variant="secondary"
          size="sm"
          icon="rotate-ccw"
          onClick={(event) => {
            holdFocusInResults(event);
            results.reload();
          }}
        >
          Try again
        </Button>
      </SearchState>
    );
  }

  const loading = results.status !== "ready";
  const nothing = !loading && results.messages.length === 0 && results.sections.length === 0;

  if (nothing) {
    return (
      <>
        <SearchMatches words={words} />
        <SearchState icon="search" title={`No results for “${results.query}”`}>
          {results.chips.length > 0 ? (
            <>
              <p className="search-state-text">Try without a filter:</p>
              <div className="search-state-actions">
                {results.chips.map((chip) => (
                  <Button
                    key={`${chip.token}:${chip.removeQuery}`}
                    variant="pill"
                    size="sm"
                    icon="x"
                    onClick={(event) => {
                      holdFocusInResults(event);
                      onQuery(chip.removeQuery);
                    }}
                  >
                    {chipLabel(chip)}
                  </Button>
                ))}
              </div>
            </>
          ) : (
            <p className="search-state-text">
              Check the spelling, try fewer or different words, or search for a person's name.
            </p>
          )}
        </SearchState>
      </>
    );
  }

  return (
    <SkeletonReveal loading={loading} skeleton={<HitSkeletons />}>
      {loading ? null : (
        <>
          {results.error === null ? null : (
            <p className="search-stale">
              <Icon name="alert" size={14} /> Showing earlier results: {results.error}
            </p>
          )}
          <SearchSections
            sections={results.sections}
            conversations={results.conversations}
            now={now}
          />
          <SearchMatches words={words} />
          <section className="search-messages" aria-label="Messages">
            <h2 className="search-messages-heading">
              Messages
              {results.messages.length === 0 ? null : (
                <span className="search-messages-count">
                  {messageCount(results.messages.length, results.hasMore)}
                </span>
              )}
            </h2>
            {results.messages.length === 0 ? (
              <p className="search-none">No messages match.</p>
            ) : null}
            {groupHits(results.messages).map((group) => {
              const conversation = results.conversations[group.conversation];

              return (
                <div key={group.key} className="search-group">
                  <ConversationHeading name={conversation} />
                  {group.hits.map((hit) => (
                    <HitRow
                      key={hit.id}
                      hit={hit}
                      conversation={conversation}
                      terms={words}
                      now={now}
                    />
                  ))}
                </div>
              );
            })}
            <MoreSentinel results={results} />
          </section>
        </>
      )}
    </SkeletonReveal>
  );
}
