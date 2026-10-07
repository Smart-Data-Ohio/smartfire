import { describe, expect, it } from "vitest";
import { messageFixture } from "../api/testing.ts";
import type { ConversationName } from "../gen/ConversationName.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { SearchResults } from "../gen/SearchResults.ts";
import {
  applyFirstPage,
  applyMorePage,
  conversationKey,
  failFirstPage,
  failMore,
  initialSearchState,
  MAX_CACHED_QUERIES,
  type SearchState,
  searchKey,
  setRecents,
  setRecentsFailed,
  setRecentsLoading,
  startMore,
  startSearch,
} from "./search.ts";

function room(roomId: number, threadId: number | null = null): ConversationName {
  return {
    roomId,
    threadId,
    roomKind: "open",
    roomName: `room ${roomId}`,
    roomIconName: null,
    threadName: threadId === null ? null : `thread ${threadId}`,
  };
}

/** A page as the server sends it: oldest first. */
function page(
  messages: readonly MessageDTO[],
  nextCursor: string | null = null,
  change: Partial<SearchResults> = {},
): SearchResults {
  return {
    query: "launch",
    chips: [],
    messages: [...messages],
    users: [],
    conversations: [room(1)],
    nextCursor,
    sections: [],
    ...change,
  };
}

function loaded(key: string, results: SearchResults): SearchState {
  const started = startSearch(initialSearchState, key);

  return applyFirstPage(started, key, 1, results);
}

describe("searchKey", () => {
  it("collapses whitespace the way the server echoes the query", () => {
    expect(searchKey("  from:ada \t launch\n plan ")).toBe("from:ada launch plan");
    expect(searchKey("   ")).toBe("");
  });
});

describe("first page", () => {
  it("shows a new query as loading, then its hits newest first", () => {
    const started = startSearch(initialSearchState, "launch");

    expect(started.results.launch?.status).toBe("loading");
    expect(started.order).toEqual(["launch"]);

    const state = applyFirstPage(
      started,
      "launch",
      1,
      page([messageFixture(1, 1), messageFixture(2, 1)], "cursor-a", {
        chips: [
          {
            operator: "has",
            value: "file",
            token: "has:file",
            label: "has: file",
            removeQuery: "launch",
          },
        ],
      }),
    );

    const list = state.results.launch;

    expect(list?.status).toBe("ready");
    expect(list?.messages.map((message) => message.id)).toEqual([2, 1]);
    expect(list?.chips.map((chip) => chip.label)).toEqual(["has: file"]);
    expect(list?.nextCursor).toBe("cursor-a");
    expect(list?.conversations[conversationKey(1, null)]?.roomName).toBe("room 1");
  });

  it("drops a superseded load's late reply", () => {
    const first = startSearch(initialSearchState, "launch");
    const second = startSearch(first, "launch");
    const late = applyFirstPage(second, "launch", 1, page([messageFixture(1, 1)]));

    expect(late.results.launch?.status).toBe("loading");

    const current = applyFirstPage(late, "launch", 2, page([messageFixture(2, 1)]));

    expect(current.results.launch?.messages.map((message) => message.id)).toEqual([2]);
  });

  it("keeps a ready list's rows while it reloads and when the reload fails", () => {
    const ready = loaded("launch", page([messageFixture(1, 1)]));
    const reloading = startSearch(ready, "launch");

    expect(reloading.results.launch?.status).toBe("ready");
    expect(reloading.results.launch?.messages).toHaveLength(1);

    const failed = failFirstPage(reloading, "launch", 2, "Offline");

    expect(failed.results.launch?.status).toBe("ready");
    expect(failed.results.launch?.error).toBe("Offline");
    expect(failed.results.launch?.messages).toHaveLength(1);
  });

  it("marks a new list's failure as an error", () => {
    const started = startSearch(initialSearchState, "launch");
    const failed = failFirstPage(started, "launch", 1, "Offline");

    expect(failed.results.launch?.status).toBe("error");
    expect(failed.results.launch?.error).toBe("Offline");
  });

  it("caches the most recent queries only", () => {
    let state = initialSearchState;

    for (let index = 0; index <= MAX_CACHED_QUERIES; index += 1) {
      state = startSearch(state, `q${index}`);
    }

    expect(state.order).toHaveLength(MAX_CACHED_QUERIES);
    expect(state.order[0]).toBe(`q${MAX_CACHED_QUERIES}`);
    expect(state.results.q0).toBeUndefined();

    const revisited = startSearch(state, "q1");

    expect(revisited.order[0]).toBe("q1");
    expect(revisited.order).toHaveLength(MAX_CACHED_QUERIES);
  });
});

describe("older pages", () => {
  it("append older hits once each and merge their conversations", () => {
    const ready = loaded("launch", page([messageFixture(40, 1), messageFixture(41, 1)], "c1"));
    const loading = startMore(ready, "launch");

    expect(loading.results.launch?.loadingMore).toBe(true);

    const more = applyMorePage(
      loading,
      "launch",
      "c1",
      page([messageFixture(20, 2, { threadId: 5 }), messageFixture(40, 1)], null, {
        conversations: [room(2, 5)],
      }),
    );

    const list = more.results.launch;

    expect(list?.messages.map((message) => message.id)).toEqual([41, 40, 20]);
    expect(list?.nextCursor).toBeNull();
    expect(list?.loadingMore).toBe(false);
    expect(Object.keys(list?.conversations ?? {}).sort()).toEqual(["1:", "2:5"]);
  });

  it("ignore a page for a cursor the list no longer waits on", () => {
    const ready = loaded("launch", page([messageFixture(40, 1)], "c1"));
    const loading = startMore(ready, "launch");
    const stale = applyMorePage(loading, "launch", "other", page([messageFixture(1, 1)]));

    expect(stale.results.launch?.messages).toHaveLength(1);

    const notWaiting = applyMorePage(ready, "launch", "c1", page([messageFixture(1, 1)]));

    expect(notWaiting.results.launch?.messages).toHaveLength(1);
  });

  it("say why an older page failed and stop loading", () => {
    const ready = loaded("launch", page([messageFixture(40, 1)], "c1"));
    const failed = failMore(startMore(ready, "launch"), "launch", "Offline");

    expect(failed.results.launch?.loadingMore).toBe(false);
    expect(failed.results.launch?.moreError).toBe("Offline");

    const retrying = startMore(failed, "launch");

    expect(retrying.results.launch?.moreError).toBeNull();
  });

  it("leave an unknown query alone", () => {
    expect(startMore(initialSearchState, "nothing")).toBe(initialSearchState);
    expect(failMore(initialSearchState, "nothing", "x")).toBe(initialSearchState);
  });
});

describe("recent searches", () => {
  const searches = [{ id: 3, query: "launch", searchedAt: "2026-10-06T09:00:00.000Z" }];

  it("load, then hold the server's list", () => {
    const loading = setRecentsLoading(initialSearchState);

    expect(loading.recents.status).toBe("loading");

    const ready = setRecents(loading, searches);

    expect(ready.recents).toEqual({ status: "ready", searches });
  });

  it("keep a shown list when a reload fails, and say so when nothing was shown", () => {
    const ready = setRecents(initialSearchState, searches);

    expect(setRecentsFailed(ready).recents.status).toBe("ready");
    expect(setRecentsFailed(setRecentsLoading(initialSearchState)).recents.status).toBe("error");
  });
});
