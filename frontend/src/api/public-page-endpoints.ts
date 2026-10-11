import { Effect, Schema } from "effect";
import type { PublicPageName } from "../gen/PublicPageName.ts";
import { ApiClient } from "./client.ts";
import { PublicPage } from "./schema/public-pages.ts";

const decodePage = Schema.decodeUnknownEffect(PublicPage);

/** `GET /public_pages/:page`: About, Privacy or Terms, for anyone, signed in or not. */
export const readPublicPage = Effect.fn("readPublicPage")(function* (page: PublicPageName) {
  const client = yield* ApiClient;

  return yield* client.execute({ method: "GET", path: `/public_pages/${page}` }, decodePage);
});
