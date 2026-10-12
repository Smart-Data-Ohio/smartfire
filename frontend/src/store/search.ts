/**
 * Global search's client state, in its own small store beside the live one: the result pages of
 * the queries searched this session (keyed by the query with its whitespace collapsed, the form
 * the server echoes and recent searches store), and the viewer's recent searches. Hits are a
 * snapshot of the server's answer; a row reads the live store first when it holds a newer copy.
 * Pure reducers below; `searchMutations` is the only writer (the search actions in src/sync).
 */
import { useStore as useZustand } from "zustand";
import { createStore } from "zustand/vanilla";
import type { ConversationName } from "../gen/ConversationName.ts";
import type { RecentSearch } from "../gen/RecentSearch.ts";
import type { SearchChip } from "../gen/SearchChip.ts";
import type { SearchResults } from "../gen/SearchResults.ts";
import type { SearchSection } from "../gen/SearchSection.ts";
import type { LoadStatus, MessageDTO } from "./model.ts";

/** One query's results, every page loaded so far. */
export interface SearchResultList {
  /** The first page's status; a reload of a ready list stays `ready` and keeps its rows. */
  readonly status: LoadStatus;
  /** Why the first page failed, or `null`. */
  readonly error: string | null;
  readonly chips: readonly SearchChip[];
  /** The hits, newest first, each once. */
  readonly messages: readonly MessageDTO[];
  /** By `conversationKey(roomId, threadId)`. */
  readonly conversations: Readonly<Record<string, ConversationName>>;
  /** First page only; often empty. */
  readonly sections: readonly SearchSection[];
  /** The next (older) page's cursor; `null` when every match is loaded. */
  readonly nextCursor: string | null;
  readonly loadingMore: boolean;
  /** Why the last older page failed, or `null`. */
  readonly moreError: string | null;
  /** Bumped by each first-page load, so a superseded load's late reply is dropped. */
  readonly generation: number;
}

/** The viewer's recent searches, most recent first. */
export interface RecentSearchesState {
  readonly status: LoadStatus;
  readonly searches: readonly RecentSearch[];
}

export interface SearchState {
  /** By `searchKey(query)`. */
  readonly results: Readonly<Record<string, SearchResultList>>;
  /** The cached keys, most recently searched first (at most `MAX_CACHED_QUERIES`). */
  readonly order: readonly string[];
  readonly recents: RecentSearchesState;
}

/** How many queries' results stay cached; older ones load again when revisited. */
export const MAX_CACHED_QUERIES = 8;

export const initialSearchState: SearchState = {
  results: {},
  order: [],
  recents: { status: "idle", searches: [] },
};

export const emptyResultList: SearchResultList = {
  status: "idle",
  error: null,
  chips: [],
  messages: [],
  conversations: {},
  sections: [],
  nextCursor: null,
  loadingMore: false,
  moreError: null,
  generation: 0,
};

/** The query as the server reads it: runs of whitespace collapsed, trimmed (`display_query`). */
export function searchKey(query: string): string {
  return query.replace(/\s+/gu, " ").trim();
}

/** How `SearchResultList.conversations` is keyed: a room's root timeline or one of its threads. */
export function conversationKey(roomId: number, threadId: number | null): string {
  return `${roomId}:${threadId ?? ""}`;
}

function withList(
  state: SearchState,
  key: string,
  change: (list: SearchResultList) => SearchResultList,
): SearchState {
  const list = state.results[key];

  if (list === undefined) {
    return state;
  }

  return { ...state, results: { ...state.results, [key]: change(list) } };
}

/** Keeps the newest `MAX_CACHED_QUERIES` keys, `key` first; the rest leave the cache. */
function touch(state: SearchState, key: string): Pick<SearchState, "order" | "results"> {
  const order = [key, ...state.order.filter((held) => held !== key)];
  const kept = order.slice(0, MAX_CACHED_QUERIES);
  const dropped = new Set(order.slice(MAX_CACHED_QUERIES));

  if (dropped.size === 0) {
    return { order: kept, results: state.results };
  }

  return {
    order: kept,
    results: Object.fromEntries(
      Object.entries(state.results).filter(([held]) => !dropped.has(held)),
    ),
  };
}

/** A first-page load starts: a new list shows loading; a held one keeps its rows meanwhile. */
export function startSearch(state: SearchState, key: string): SearchState {
  const held = state.results[key] ?? emptyResultList;
  const { order, results } = touch(state, key);

  const list: SearchResultList = {
    ...held,
    status: held.status === "ready" ? "ready" : "loading",
    error: null,
    generation: held.generation + 1,
  };

  return { ...state, order, results: { ...results, [key]: list } };
}

function conversationsOf(results: SearchResults, held: SearchResultList["conversations"] = {}) {
  return Object.fromEntries([
    ...Object.entries(held),
    ...results.conversations.map(
      (name) => [conversationKey(name.roomId, name.threadId), name] as const,
    ),
  ]);
}

/**
 * `results` without what's in a thread `removed` says the viewer lost or saw deleted: its
 * replies, its name and its rows in the board-post and work-thread sections. A search sent before
 * that still lists them.
 */
export function withoutThreads(
  results: SearchResults,
  removed: (threadId: number) => boolean,
): SearchResults {
  const gone = (threadId: number | null) => threadId !== null && removed(threadId);

  return {
    ...results,
    messages: results.messages.filter((message) => !gone(message.threadId)),
    conversations: results.conversations.filter((name) => !gone(name.threadId)),
    sections: results.sections.map((section) =>
      section.kind === "events"
        ? section
        : { ...section, rows: section.rows.filter((row) => !gone(row.id)) },
    ),
  };
}

/** The first page landed (ignored when a later load superseded the one it answers). */
export function applyFirstPage(
  state: SearchState,
  key: string,
  generation: number,
  results: SearchResults,
): SearchState {
  return withList(state, key, (list) =>
    list.generation !== generation
      ? list
      : {
          ...list,
          status: "ready",
          error: null,
          chips: results.chips,
          messages: [...results.messages].reverse(),
          conversations: conversationsOf(results),
          sections: results.sections,
          nextCursor: results.nextCursor,
          loadingMore: false,
          moreError: null,
        },
  );
}

/** The first page failed; a list that was showing rows keeps them and says why. */
export function failFirstPage(
  state: SearchState,
  key: string,
  generation: number,
  error: string,
): SearchState {
  return withList(state, key, (list) =>
    list.generation !== generation
      ? list
      : { ...list, status: list.status === "ready" ? "ready" : "error", error },
  );
}

/** An older page is on its way. */
export function startMore(state: SearchState, key: string): SearchState {
  return withList(state, key, (list) => ({ ...list, loadingMore: true, moreError: null }));
}

/**
 * An older page landed. It only applies while the list still waits on `cursor` (a first-page
 * reload in between started the list over), and a hit already shown isn't listed twice.
 */
export function applyMorePage(
  state: SearchState,
  key: string,
  cursor: string,
  results: SearchResults,
): SearchState {
  return withList(state, key, (list) => {
    if (list.nextCursor !== cursor || !list.loadingMore) {
      return list;
    }

    const shown = new Set(list.messages.map((message) => message.id));
    const older = [...results.messages].reverse().filter((message) => !shown.has(message.id));

    return {
      ...list,
      messages: [...list.messages, ...older],
      conversations: conversationsOf(results, list.conversations),
      nextCursor: results.nextCursor,
      loadingMore: false,
      moreError: null,
    };
  });
}

/** An older page failed: the list says so and offers to try again. */
export function failMore(state: SearchState, key: string, error: string): SearchState {
  return withList(state, key, (list) => ({ ...list, loadingMore: false, moreError: error }));
}

export function setRecentsLoading(state: SearchState): SearchState {
  return { ...state, recents: { ...state.recents, status: "loading" } };
}

/** The server's list (a load, or the answer to recording a search). */
export function setRecents(state: SearchState, searches: readonly RecentSearch[]): SearchState {
  return { ...state, recents: { status: "ready", searches } };
}

export function setRecentsFailed(state: SearchState): SearchState {
  return {
    ...state,
    recents: { ...state.recents, status: state.recents.status === "ready" ? "ready" : "error" },
  };
}

/** The search store. Components read it through `useSearchStore(selector)`. */
/**
 * The viewer lost access to a private thread: its replies, its name and its row in the board-post
 * and work-thread sections leave every cached query, as a fresh search wouldn't find them.
 */
export function forgetThread(state: SearchState, threadId: number): SearchState {
  let changed = false;

  const results = Object.fromEntries(
    Object.entries(state.results).map(([key, list]) => {
      const messages = list.messages.filter((message) => message.threadId !== threadId);

      const conversations = Object.fromEntries(
        Object.entries(list.conversations).filter(([, name]) => name.threadId !== threadId),
      );

      const sections = list.sections.map((section) =>
        section.kind === "events"
          ? section
          : { ...section, rows: section.rows.filter((row) => row.id !== threadId) },
      );

      const same =
        messages.length === list.messages.length &&
        Object.keys(conversations).length === Object.keys(list.conversations).length &&
        sections.every(
          (section, index) => section.rows.length === list.sections[index]?.rows.length,
        );

      if (same) {
        return [key, list] as const;
      }

      changed = true;

      return [key, { ...list, messages, conversations, sections }] as const;
    }),
  );

  return changed ? { ...state, results } : state;
}

export const searchStore = createStore<SearchState>()(() => initialSearchState);

export function useSearchStore<T>(selector: (state: SearchState) => T): T {
  return useZustand(searchStore, selector);
}

const apply = (change: (state: SearchState) => SearchState) => searchStore.setState(change, true);

/** Every write to the search store, one `setState` each. */
export const searchMutations = {
  startSearch: (key: string) => apply((state) => startSearch(state, key)),
  applyFirstPage: (key: string, generation: number, results: SearchResults) =>
    apply((state) => applyFirstPage(state, key, generation, results)),
  failFirstPage: (key: string, generation: number, error: string) =>
    apply((state) => failFirstPage(state, key, generation, error)),
  startMore: (key: string) => apply((state) => startMore(state, key)),
  applyMorePage: (key: string, cursor: string, results: SearchResults) =>
    apply((state) => applyMorePage(state, key, cursor, results)),
  failMore: (key: string, error: string) => apply((state) => failMore(state, key, error)),
  setRecentsLoading: () => apply(setRecentsLoading),
  setRecents: (searches: readonly RecentSearch[]) => apply((state) => setRecents(state, searches)),
  setRecentsFailed: () => apply(setRecentsFailed),
  forgetThread: (threadId: number) => apply((state) => forgetThread(state, threadId)),
  /** Back to empty (tests). */
  reset: () => apply(() => initialSearchState),
};
