import { describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { FakeApi } from "./testing.ts";
import { readTwoFactorSetup, submitTwoFactorSetup } from "./two-factor-setup-endpoints.ts";

const setup = {
  secret: "JBSWY3DPEHPK3PXP",
  manualKey: "JBSW Y3DP EHPK 3PXP",
  otpauthUri: "otpauth://totp/Smartfire:ada%40example.com?secret=JBSWY3DPEHPK3PXP",
  qrSvg: '<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0"/></svg>',
};

describe("two-factor setup contracts", () => {
  it.effect("reads the provisioning material and confirms to one-time recovery codes", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const ready = { kind: "ready", setup };

      const codes = {
        kind: "recoveryCodes",
        codes: ["abcd-1234-efgh", "ijkl-5678-mnop"],
        signedOut: 2,
        continueUrl: "http://campfire.test/app/settings?tab=security",
      };

      yield* fake.reply("GET /two_factor/setup", ready);
      yield* fake.reply("POST /two_factor/setup", codes);
      expect(yield* readTwoFactorSetup()).toEqual(ready);
      expect(yield* submitTwoFactorSetup({ code: "123 456" })).toEqual(codes);
      yield* fake.reply("GET /two_factor/setup", {
        kind: "navigate",
        location: "http://campfire.test/users/me/profile",
      });
      expect((yield* readTwoFactorSetup()).kind).toBe("navigate");
      expect(yield* fake.requests).toMatchObject([
        { method: "GET", path: "/two_factor/setup", auth: true },
        { method: "POST", path: "/two_factor/setup", auth: true, body: { code: "123 456" } },
        { method: "GET", path: "/two_factor/setup", auth: true },
      ]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keeps rejected codes with the setup state and rejects incomplete contracts", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      const error = {
        kind: "error",
        message: "That code didn't work. Check your authenticator app and try again.",
        setup,
      };

      yield* fake.reply("POST /two_factor/setup", error);
      expect(yield* submitTwoFactorSetup({ code: "wrong" })).toEqual(error);
      yield* fake.reply("POST /two_factor/setup", {
        ...error,
        message: "Too many attempts. Try again in a few minutes.",
      });
      expect((yield* submitTwoFactorSetup({ code: "123456" })).kind).toBe("error");

      for (const reply of [
        { kind: "ready", setup: { secret: setup.secret } },
        { kind: "recoveryCodes", codes: ["abcd-1234-efgh"] },
        { kind: "error", message: error.message },
      ]) {
        yield* fake.reply("GET /two_factor/setup", reply);
        expect((yield* readTwoFactorSetup().pipe(Effect.flip))._tag).toBe("ServerError");
      }
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
