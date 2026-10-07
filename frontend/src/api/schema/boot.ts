import { Schema } from "effect";
import type { Boot as ModelBoot } from "../../store/model.ts";
import { TextSize, Theme } from "./me.ts";
import type { Assert, Pinned } from "./pin.ts";

/**
 * The boot JSON (`crates/spa`'s `Boot`): what the Rust shell inlines as
 * `<script type="application/json" id="boot">`. Hand-written until it moves into api_types; pinned
 * to the store's `Boot`. Plain values only, so decoding returns the wire value unchanged.
 */
export const Boot = Schema.Struct({
  user: Schema.Struct({ id: Schema.Int, name: Schema.String, avatarUrl: Schema.String }),
  account: Schema.Struct({ name: Schema.NullOr(Schema.String) }),
  theme: Theme,
  textSize: TextSize,
  cableUrl: Schema.String,
  serviceWorkerUrl: Schema.NullOr(Schema.String),
  version: Schema.String,
  revision: Schema.NullOr(Schema.String),
});

export type BootPin = Assert<Pinned<typeof Boot, ModelBoot>>;

/** `GET /api/v1/boot` (sign-in required): the boot JSON plus a fresh CSRF token, without a reload. */
export const BootReply = Schema.Struct({ ...Boot.fields, csrfToken: Schema.String });

export type BootReply = typeof BootReply.Type;
