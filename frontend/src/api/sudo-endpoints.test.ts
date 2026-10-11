import { describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { readSudo, resumeSudo, startSudoGoogle, submitSudo } from "./sudo-endpoints.ts";
import { FakeApi } from "./testing.ts";

const retry = { method: "PATCH", path: "/api/v1/admin/custom_styles", returnTo: "/app/admin" };

describe("sudo contracts", () => {
  it.effect("reads methods, confirms either credential, and resumes Google", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      const state = {
        kind: "ready",
        reauthentication: { methods: ["password", "totp", "google"], retry },
      };

      const confirmed = { kind: "confirmed", retry };

      yield* fake.reply("GET /sudo", state);
      yield* fake.reply("POST /sudo", confirmed);
      yield* fake.reply("POST /sudo/google", {
        kind: "navigate",
        location: "https://accounts.google.com/authorize",
      });
      yield* fake.reply("GET /sudo/continue", confirmed);
      expect(yield* readSudo()).toEqual(state);
      expect(yield* submitSudo({ kind: "password", password: "secret" })).toEqual(confirmed);
      expect(yield* submitSudo({ kind: "totp", code: "123456" })).toEqual(confirmed);
      expect((yield* startSudoGoogle()).kind).toBe("navigate");
      expect(yield* resumeSudo()).toEqual(confirmed);
      expect(yield* fake.requests).toMatchObject([
        { method: "GET", path: "/sudo", auth: true },
        {
          method: "POST",
          path: "/sudo",
          body: { kind: "password", password: "secret" },
          auth: true,
        },
        { method: "POST", path: "/sudo", body: { kind: "totp", code: "123456" }, auth: true },
        { method: "POST", path: "/sudo/google", body: {}, auth: true },
        { method: "GET", path: "/sudo/continue", auth: true },
      ]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keeps credential refusals as data and rejects malformed responses", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      const rejected = { kind: "error", message: "Confirmation failed. Try again." };
      yield* fake.reply("POST /sudo", rejected);
      expect(yield* submitSudo({ kind: "password", password: "wrong" })).toEqual(rejected);
      yield* fake.reply("GET /sudo", {
        kind: "ready",
        reauthentication: { methods: ["recoveryCode"], retry: null },
      });
      expect((yield* readSudo().pipe(Effect.flip))._tag).toBe("ServerError");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
