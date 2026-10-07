/**
 * Every search reply the mock sends decodes with the SPA's pinned wire schemas, so the mock
 * can't drift from the contract the client is built against.
 */
import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import { RecentSearchList, SearchResults } from "../../src/api/schema/search.ts";
import { harness, send } from "../s2/testing.ts";

const QUERIES = [
  "onboarding",
  "launch",
  "the",
  "from:@maya has:file",
  "has:image",
  "has:pin",
  "has:link docs",
  "is:thread invite",
  "in:#engineering rate limiter",
  "on:2026-10-05",
  "before:2026-09-26 after:2026-09-24",
  "has:bogus",
  "nothing matches this",
  "",
];

describe("search replies", () => {
  it("decode as SearchResults, every page of every query", async () => {
    const { server } = harness();
    const decode = Schema.decodeUnknownSync(SearchResults);

    for (const q of QUERIES) {
      let before: string | null = null;

      do {
        const query = new URLSearchParams(before === null ? { q } : { q, before });
        const response = await server.handle({ method: "GET", path: `/api/v1/search?${query}` });

        expect(response.status, q).toBe(200);

        const page = decode(response.json);

        before = page.nextCursor;
      } while (before !== null);
    }
  });

  it("decode as RecentSearchList when listed and recorded", async () => {
    const { server } = harness();
    const decode = Schema.decodeUnknownSync(RecentSearchList);
    const listed = await server.handle({ method: "GET", path: "/api/v1/search/recents" });

    expect(decode(listed.json).searches).toHaveLength(3);

    const recorded = await send(server, "POST", "/api/v1/search/recents", { query: "launch" });

    expect(decode(recorded.json).searches[0]?.query).toBe("launch");
  });
});
