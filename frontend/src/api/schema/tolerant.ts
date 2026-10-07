/**
 * Tolerant literals: a closed set of values that the server may grow. A value this build doesn't
 * know decodes to `"unknown"` instead of failing the whole object (and every list holding it).
 * Encodes back to what it decoded from, except that an unknown value encodes as `"unknown"`.
 */
import { Predicate, Schema, SchemaGetter } from "effect";
import type { ToleratedString } from "./pin.ts";

function isToleratedString(input: unknown): input is ToleratedString {
  return Predicate.isString(input);
}

/** Any string, typed as `ToleratedString` so the pins skip it. */
const AnyString = Schema.declare(isToleratedString);

/** `Schema.Literals(known)`, plus `"unknown"` for any other string. */
export function tolerantLiterals<const Known extends readonly [string, ...string[]]>(known: Known) {
  const isKnown = (raw: string): raw is Known[number] => known.some((value) => value === raw);
  const Known = Schema.Literals(known);

  return Schema.Union([Known, AnyString]).pipe(
    Schema.decodeTo(Schema.Union([Known, Schema.Literal("unknown")]), {
      decode: SchemaGetter.transform((raw) => (isKnown(raw) ? raw : ("unknown" as const))),
      encode: SchemaGetter.transform((value) => value),
    }),
  );
}
