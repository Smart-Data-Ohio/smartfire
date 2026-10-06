import type { Schema } from "effect";

/**
 * The wire shape of a tolerant fallback, such as `UnknownMessageCard`: any `kind`, with or without
 * `data`. It's left out of the comparison (it accepts what the generated union doesn't, on
 * purpose), so a union with it pins against the generated union alone.
 */
export type ToleratedWire = { readonly kind: string; readonly data?: unknown };

type IsTolerated<T> = [T] extends [ToleratedWire]
  ? [ToleratedWire] extends [T]
    ? true
    : false
  : false;

/**
 * Effect Schema's arrays and records are readonly and ts-rs's aren't. Readonly-ness says nothing
 * about the wire, so both sides are compared deeply readonly.
 */
type Wire<T> = T extends unknown
  ? IsTolerated<T> extends true
    ? never
    : T extends ReadonlyArray<infer Item>
      ? ReadonlyArray<Wire<Item>>
      : T extends object
        ? { readonly [Key in keyof T]: Wire<T[Key]> }
        : T
  : never;

/**
 * `true` when the JSON a schema decodes (its `Encoded` side) and the type ts-rs generated from
 * the Rust struct are assignable to each other, else `false`. The decoded side differs on
 * purpose: it has branded ids and `DateTime.Utc` values.
 */
export type Pinned<S extends Schema.Top, Generated> = [Wire<Schema.Codec.Encoded<S>>] extends [
  Wire<Generated>,
]
  ? [Wire<Generated>] extends [Wire<Schema.Codec.Encoded<S>>]
    ? true
    : false
  : false;

/**
 * Fails to compile unless `T` is `true`. Each schema module exports
 * `type ...Pin = Assert<Pinned<typeof Schema, GeneratedType>>`, so `tsc` fails until the schema
 * follows a change to the Rust struct (`pnpm gen` regenerates `src/gen/`).
 */
export type Assert<T extends true> = T;
