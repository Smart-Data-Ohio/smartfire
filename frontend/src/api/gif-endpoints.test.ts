import { Effect } from "effect";
import { afterEach, describe, expect, it } from "vitest";
import { GIF_BYTES, GIF_FIXTURES } from "../../mock/fixtures/gifs.ts";
import { harness } from "../../mock/s2/testing.ts";
import { installMockNetwork, type MockNetwork } from "../test/mock-network.ts";
import { ApiClient } from "./client.ts";
import {
  gifAutocomplete,
  gifCategories,
  gifConfig,
  gifSuggestions,
  searchGifs,
  trendingGifs,
} from "./gif-endpoints.ts";

let network: MockNetwork | null = null;

afterEach(() => {
  network?.server.dispose();
  network?.restore();
  network = null;
});

describe("typed GIF API client", () => {
  it("requests and decodes discovery through the app HTTP client", async () => {
    const { server } = harness();

    network = installMockNetwork(server);

    const result = await Effect.runPromise(
      Effect.gen(function* () {
        return {
          config: yield* gifConfig(),
          trending: yield* trendingGifs(),
          search: yield* searchGifs("Hello Wave"),
          categories: yield* gifCategories(),
          suggestions: yield* gifSuggestions("hello"),
          autocomplete: yield* gifAutocomplete("he"),
        };
      }).pipe(Effect.provide(ApiClient.layerBrowser("/api/v1"))),
    );

    expect(result.config).toEqual({ enabled: true });
    expect(result.trending.items).toEqual(GIF_FIXTURES);
    expect(result.search.items).toEqual([GIF_FIXTURES[0]]);
    expect(result.categories.categories.map((category) => category.name)).toEqual([
      "Hello Wave",
      "Celebration Cheer",
    ]);
    expect(result.suggestions.suggestions).toEqual(["hello", "hello there"]);
    expect(result.autocomplete.suggestions).toEqual(["hello", "hello there", "cheer"]);
  });

  it("serves preview and persisted fixture URLs as bytes through mock fetch", async () => {
    const { server } = harness();

    network = installMockNetwork(server);
    const fixture = GIF_FIXTURES[0];

    expect(fixture).toBeDefined();
    const preview = await fetch(fixture?.previewUrl ?? "");

    expect(preview.status).toBe(200);
    expect(preview.headers.get("content-type")).toBe("image/gif");
    expect(new Uint8Array(await preview.arrayBuffer())).toEqual(GIF_BYTES);
    const saved = await fetch("/mock/files/gifs/hello-wave.gif");

    expect(saved.status).toBe(200);
    expect(new Uint8Array(await saved.arrayBuffer())).toEqual(GIF_BYTES);
  });
});
