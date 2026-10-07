import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { Validation } from "../api/errors.ts";
import { FakeApi, messageFixture, userFixture } from "../api/testing.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { SearchResults } from "../gen/SearchResults.ts";
import { searchMutations, searchStore } from "../store/search.ts";
import { mutations, store } from "../store/store.ts";
import * as search from "./search-actions.ts";

function results(messages: readonly MessageDTO[], nextCursor: string | null = null): SearchResults {
  return {
    query: "launch",
    chips: [],
    messages: [...messages],
    users: [userFixture(7, "Ada")],
    conversations: [
      {
        roomId: 1,
        threadId: null,
        roomKind: "open",
        roomName: "general",
        roomIconName: null,
        threadName: null,
      },
    ],
    nextCursor,
    sections: [],
  };
}

const refused = new Validation({
  message: "Enter a word to search for.",
  fields: { query: ["Enter a word to search for."] },
});

describe("search actions", () => {
  afterEach(() => {
    searchMutations.reset();
    mutations.reset();
  });

  it.effect("load a query's first page and keep its people", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      yield* fake.reply("GET /search", results([messageFixture(1, 1), messageFixture(2, 1)], "c1"));
      yield* search.run("  launch   plan ");

      const requests = yield* fake.requests;
      const list = searchStore.getState().results["launch plan"];

      expect(requests.at(-1)?.query).toEqual({ q: "launch plan" });
      expect(list?.status).toBe("ready");
      expect(list?.messages.map((message) => message.id)).toEqual([2, 1]);
      expect(store.getState().users[7]?.name).toBe("Ada");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("record a failed first page in the store instead of failing", () =>
    Effect.gen(function* () {
      yield* search.run("launch");

      const list = searchStore.getState().results.launch;

      expect(list?.status).toBe("error");
      expect(list?.error).toContain("No route");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("page through older matches with the cursor, once at a time", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      yield* fake.reply("GET /search", results([messageFixture(50, 1)], "c1"));
      yield* search.run("launch");
      yield* fake.reply("GET /search", results([messageFixture(10, 1)], null));
      yield* search.loadMore("launch");

      const requests = yield* fake.requests;
      const list = searchStore.getState().results.launch;

      expect(requests.at(-1)?.query).toEqual({ q: "launch", before: "c1" });
      expect(list?.messages.map((message) => message.id)).toEqual([50, 10]);
      expect(list?.nextCursor).toBeNull();

      yield* search.loadMore("launch");

      expect(yield* fake.requests).toHaveLength(2);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("say when an older page fails", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      yield* fake.reply("GET /search", results([messageFixture(50, 1)], "c1"));
      yield* search.run("launch");
      yield* fake.route("GET /search", () =>
        Effect.fail(
          new Validation({ message: "That page is gone.", fields: { before: ["is invalid"] } }),
        ),
      );
      yield* search.loadMore("launch");

      const list = searchStore.getState().results.launch;

      expect(list?.moreError).toBe("That page is gone.");
      expect(list?.loadingMore).toBe(false);
      expect(list?.messages).toHaveLength(1);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("load the recent searches", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const searches = [{ id: 1, query: "launch", searchedAt: "2026-10-06T09:00:00.000Z" }];

      yield* fake.reply("GET /search/recents", { searches });
      yield* search.loadRecents();

      expect(searchStore.getState().recents).toEqual({ status: "ready", searches });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("move a recorded search to the top at once, then take the server's list", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const older = { id: 1, query: "deploy", searchedAt: "2026-10-05T09:00:00.000Z" };
      const repeat = { id: 2, query: "launch", searchedAt: "2026-10-05T10:00:00.000Z" };

      searchMutations.setRecents([older, repeat]);

      let shownDuringRequest: readonly string[] = [];

      yield* fake.route("POST /search/recents", () => {
        shownDuringRequest = searchStore.getState().recents.searches.map((row) => row.query);

        return Effect.succeed({
          searches: [{ ...repeat, searchedAt: "2026-10-06T16:30:00.000Z" }, older],
        });
      });
      yield* search.record(" launch ");

      const requests = yield* fake.requests;

      expect(requests.at(-1)?.body).toEqual({ query: "launch" });
      expect(shownDuringRequest).toEqual(["launch", "deploy"]);
      expect(searchStore.getState().recents.searches[0]?.searchedAt).toBe(
        "2026-10-06T16:30:00.000Z",
      );
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("put the recents back when recording fails, and skip a blank query", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const held = [{ id: 1, query: "deploy", searchedAt: "2026-10-05T09:00:00.000Z" }];

      searchMutations.setRecents(held);
      yield* search.record("   ");

      expect(yield* fake.requests).toHaveLength(0);

      yield* fake.route("POST /search/recents", () => Effect.fail(refused));

      const outcome = yield* Effect.flip(search.record("launch"));

      expect(outcome.message).toBe("Enter a word to search for.");
      expect(searchStore.getState().recents.searches).toEqual(held);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep a newer server list when a refused change ends", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const held = [{ id: 1, query: "deploy", searchedAt: "2026-10-05T09:00:00.000Z" }];
      const newer = [{ id: 3, query: "budget", searchedAt: "2026-10-06T16:00:00.000Z" }, ...held];

      searchMutations.setRecents(held);
      yield* fake.route("DELETE /search/recents", () => {
        // A search in another pane brought the server's list meanwhile.
        searchMutations.setRecents(newer);

        return Effect.fail(refused);
      });
      yield* Effect.flip(search.clearRecents());

      expect(searchStore.getState().recents.searches).toEqual(newer);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("clear the recents at once and restore them if the server refuses", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const held = [{ id: 1, query: "deploy", searchedAt: "2026-10-05T09:00:00.000Z" }];

      searchMutations.setRecents(held);
      yield* fake.route("DELETE /search/recents", () => Effect.succeed(null));
      yield* search.clearRecents();

      expect(searchStore.getState().recents.searches).toEqual([]);

      searchMutations.setRecents(held);
      yield* fake.route("DELETE /search/recents", () => Effect.fail(refused));
      yield* Effect.flip(search.clearRecents());

      expect(searchStore.getState().recents.searches).toEqual(held);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
