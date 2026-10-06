import { expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { ApiConfig, endpointUrl } from "../api/client.ts";
import { actions } from "./runtime.ts";

it.effect("resolves API paths against the configured base", () =>
  Effect.gen(function* () {
    expect(yield* endpointUrl("/me")).toBe("https://example.test/api/v1/me");
  }).pipe(Effect.provide(ApiConfig.layer("https://example.test/api/v1"))),
);

it("runs actions on the shared runtime", async () => {
  expect(await actions.endpointUrl("/me")).toBe("/api/v1/me");
});
