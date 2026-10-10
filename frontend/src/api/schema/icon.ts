import { Schema } from "effect";
import type { Icon as GeneratedIcon } from "../../gen/Icon.ts";
import type { IconKind as GeneratedIconKind } from "../../gen/IconKind.ts";
import type { Assert, Pinned } from "./pin.ts";

export const IconKind = Schema.Literals(["brand", "custom", "emoji"]);

export type IconKind = typeof IconKind.Type;

export type IconKindPin = Assert<Pinned<typeof IconKind, GeneratedIconKind>>;

/** What `:name:` expands to: an emoji `character`, or a brand/custom `imageUrl`. */
export const Icon = Schema.Struct({
  name: Schema.String,
  title: Schema.String,
  kind: IconKind,
  character: Schema.NullOr(Schema.String),
  imageUrl: Schema.NullOr(Schema.String),
  animated: Schema.Boolean,
  stillUrl: Schema.NullOr(Schema.String),
});

export type Icon = typeof Icon.Type;

export type IconPin = Assert<Pinned<typeof Icon, GeneratedIcon>>;
