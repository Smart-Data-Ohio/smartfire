import { Schema } from "effect";
import type { GifCategoryList as GeneratedGifCategoryList } from "../../gen/GifCategoryList.ts";
import type { GifConfig as GeneratedGifConfig } from "../../gen/GifConfig.ts";
import type { GifList as GeneratedGifList } from "../../gen/GifList.ts";
import type { GifSuggestions as GeneratedGifSuggestions } from "../../gen/GifSuggestions.ts";
import type { Assert, Pinned } from "./pin.ts";

export const GifConfig = Schema.Struct({ enabled: Schema.Boolean });

export type GifConfigPin = Assert<Pinned<typeof GifConfig, GeneratedGifConfig>>;

export const GifItem = Schema.Struct({
  slug: Schema.String,
  klipyId: Schema.String,
  title: Schema.String,
  attribution: Schema.String,
  previewUrl: Schema.String,
  stillUrl: Schema.NullOr(Schema.String),
  width: Schema.Int,
  height: Schema.Int,
});

export const GifList = Schema.Struct({
  items: Schema.Array(GifItem),
  page: Schema.Int,
  hasNext: Schema.Boolean,
});

export type GifListPin = Assert<Pinned<typeof GifList, GeneratedGifList>>;

export const GifCategoryList = Schema.Struct({
  categories: Schema.Array(
    Schema.Struct({
      name: Schema.String,
      query: Schema.String,
      previewUrl: Schema.NullOr(Schema.String),
    }),
  ),
});

export type GifCategoryListPin = Assert<Pinned<typeof GifCategoryList, GeneratedGifCategoryList>>;

export const GifSuggestions = Schema.Struct({ suggestions: Schema.Array(Schema.String) });

export type GifSuggestionsPin = Assert<Pinned<typeof GifSuggestions, GeneratedGifSuggestions>>;
