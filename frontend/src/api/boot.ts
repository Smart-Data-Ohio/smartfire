import { Effect, Option, Schema } from "effect";
import type { Boot as BootData } from "../store/model.ts";
import { ApiClient } from "./client.ts";
import { boot } from "./endpoints.ts";
import { Boot } from "./schema/boot.ts";

const decodeInlineBoot = Schema.decodeUnknownOption(Schema.fromJsonString(Boot));

/** The JSON in `<script type="application/json" id="boot">`, when the Rust shell inlined it. */
function inlineBootText(): string | null {
  if (!("document" in globalThis)) {
    return null;
  }

  return document.getElementById("boot")?.textContent ?? null;
}

/**
 * The boot data: parsed from the page the Rust shell rendered, else (the Vite dev server's page
 * has none) fetched from `GET /api/v1/boot`, whose CSRF token the client then adopts.
 */
export const readBoot = Effect.fn("readBoot")(function* () {
  const inline = Option.flatMap(Option.fromNullishOr(inlineBootText()), decodeInlineBoot);

  if (Option.isSome(inline)) {
    const data: BootData = inline.value;

    return data;
  }

  const client = yield* ApiClient;
  const { csrfToken, ...data } = yield* boot();

  yield* client.setCsrfToken(csrfToken);

  return data;
});
