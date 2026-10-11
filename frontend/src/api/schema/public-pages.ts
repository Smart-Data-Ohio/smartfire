import { Schema } from "effect";
import type { PublicPage as WirePage } from "../../gen/PublicPage.ts";
import type { Assert, Pinned } from "./pin.ts";

export const PublicPageName = Schema.Literals(["about", "privacy", "terms"]);

export const PublicPage = Schema.Struct({
  page: PublicPageName,
  title: Schema.String,
  description: Schema.String,
  policy: Schema.Struct({
    operatorName: Schema.NullOr(Schema.String),
    contactEmail: Schema.NullOr(Schema.String),
    effectiveDate: Schema.String,
  }),
  html: Schema.String,
});

export type PublicPagePin = Assert<Pinned<typeof PublicPage, WirePage>>;
