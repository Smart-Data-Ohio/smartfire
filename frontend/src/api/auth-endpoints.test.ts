import { describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import {
  consumeTransfer,
  googleSignIn,
  passwordSignIn,
  readChallenge,
  signedOutBoot,
  signOut,
  submitChallenge,
} from "./auth-endpoints.ts";
import { FakeApi } from "./testing.ts";

const signedIn = { kind: "signedIn", location: "http://campfire.test/app/" };

const challenge = {
  kind: "secondFactorRequired",
  challenge: { methods: ["totp", "recoveryCode"], rememberDevice: true },
};

describe("signed-out auth contracts", () => {
  it.effect("uses typed JSON requests and decodes each next action", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply("POST /session", signedIn);
      yield* fake.reply("POST /session/google", {
        kind: "navigate",
        location: "https://accounts.google.com/authorize",
      });
      yield* fake.reply("GET /two_factor/challenge", challenge);
      yield* fake.reply("POST /two_factor/challenge", signedIn);
      yield* fake.reply("PUT /session/transfers/a%2Fb%3Ftoken", challenge);
      yield* fake.reply("DELETE /session", { kind: "navigate", location: "http://campfire.test/" });
      expect(
        yield* passwordSignIn({ emailAddress: "ada@example.com", password: "secret" }),
      ).toEqual(signedIn);
      expect((yield* googleSignIn()).kind).toBe("navigate");
      expect(yield* readChallenge()).toEqual(challenge);
      expect(yield* submitChallenge({ code: "123456", rememberDevice: true })).toEqual(signedIn);
      expect(yield* consumeTransfer("a/b?token")).toEqual(challenge);
      expect((yield* signOut({ pushSubscriptionEndpoint: null })).kind).toBe("navigate");
      expect(yield* fake.requests).toMatchObject([
        {
          method: "POST",
          path: "/session",
          body: { emailAddress: "ada@example.com", password: "secret" },
        },
        { method: "POST", path: "/session/google" },
        { method: "GET", path: "/two_factor/challenge" },
        {
          method: "POST",
          path: "/two_factor/challenge",
          body: { code: "123456", rememberDevice: true },
        },
        { method: "PUT", path: "/session/transfers/a%2Fb%3Ftoken" },
        { method: "DELETE", path: "/session", body: { pushSubscriptionEndpoint: null } },
      ]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("returns field errors and the public boot without navigation", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      const error = {
        kind: "error",
        fieldErrors: { base: ["Too many requests or unauthorized."] },
      };

      yield* fake.reply("POST /session", error);
      expect(
        yield* passwordSignIn({ emailAddress: "unknown@example.com", password: "wrong" }),
      ).toEqual(error);

      const boot = {
        kind: "signedOut",
        workspace: { name: "Smart Data", logoUrl: null, description: "Hello" },
        signInMethods: { password: true, google: true, googleDomains: ["smartdata.net"] },
        firstRunPending: false,
        helpContact: { name: "Ada", emailAddress: "ada@example.com" },
        version: "2.0.0",
        csrfToken: "public-token",
      };

      yield* fake.reply("GET /session/boot", boot);
      expect(yield* signedOutBoot()).toEqual(boot);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});

describe("auth response validation", () => {
  it.effect("rejects unknown next actions instead of inventing a navigation", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply("POST /session", { kind: "signedIn" });

      const result = yield* passwordSignIn({
        emailAddress: "ada@example.com",
        password: "secret",
      }).pipe(Effect.flip);

      expect(result._tag).toBe("ServerError");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
