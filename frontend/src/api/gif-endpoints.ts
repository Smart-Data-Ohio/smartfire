import { Effect } from "effect";
import type { GifCategoryList } from "../gen/GifCategoryList.ts";
import type { GifConfig } from "../gen/GifConfig.ts";
import type { GifList } from "../gen/GifList.ts";
import type { GifSuggestions } from "../gen/GifSuggestions.ts";
import { call, get } from "./call.ts";
import * as schema from "./schema/gif.ts";
import { wire } from "./wire.ts";

export const gifConfig = Effect.fn("api.gifConfig")(function* () {
  return yield* call(get("/gifs/config"), wire<GifConfig>(schema.GifConfig));
});

export const searchGifs = Effect.fn("api.searchGifs")(function* (query: string, page = 1) {
  return yield* call(
    get("/gifs/search", { q: query, page: String(page) }),
    wire<GifList>(schema.GifList),
  );
});

export const trendingGifs = Effect.fn("api.trendingGifs")(function* (page = 1) {
  return yield* call(get("/gifs/trending", { page: String(page) }), wire<GifList>(schema.GifList));
});

export const gifCategories = Effect.fn("api.gifCategories")(function* () {
  return yield* call(get("/gifs/categories"), wire<GifCategoryList>(schema.GifCategoryList));
});

export const gifSuggestions = Effect.fn("api.gifSuggestions")(function* (query: string) {
  return yield* call(
    get("/gifs/suggestions", { q: query }),
    wire<GifSuggestions>(schema.GifSuggestions),
  );
});

export const gifAutocomplete = Effect.fn("api.gifAutocomplete")(function* (query: string) {
  return yield* call(
    get("/gifs/autocomplete", { q: query }),
    wire<GifSuggestions>(schema.GifSuggestions),
  );
});
