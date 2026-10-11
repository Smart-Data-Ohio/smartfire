import { Effect, Schema } from "effect";
import type { ChallengeSubmission } from "../gen/ChallengeSubmission.ts";
import type { GoogleSignInStart } from "../gen/GoogleSignInStart.ts";
import type { PasswordSignIn } from "../gen/PasswordSignIn.ts";
import type { SignOut } from "../gen/SignOut.ts";
import type { TransferSignIn } from "../gen/TransferSignIn.ts";
import { ApiClient } from "./client.ts";
import { AuthResponse, SignedOutBoot } from "./schema/auth.ts";

const decodeAuth = Schema.decodeUnknownEffect(AuthResponse);

const decodeBoot = Schema.decodeUnknownEffect(SignedOutBoot);

export const signedOutBoot = Effect.fn("signedOutBoot")(function* () {
  const client = yield* ApiClient;

  const boot = yield* client.execute(
    { method: "GET", path: "/session/boot", auth: true },
    decodeBoot,
  );

  yield* client.setCsrfToken(boot.csrfToken);

  return boot;
});

export const passwordSignIn = Effect.fn("passwordSignIn")(function* (body: PasswordSignIn) {
  const client = yield* ApiClient;

  return yield* client.execute({ method: "POST", path: "/session", body, auth: true }, decodeAuth);
});

export const googleSignIn = Effect.fn("googleSignIn")(function* () {
  const client = yield* ApiClient;
  const body: GoogleSignInStart = {};

  return yield* client.execute(
    { method: "POST", path: "/session/google", body, auth: true },
    decodeAuth,
  );
});

export const readChallenge = Effect.fn("readChallenge")(function* () {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "GET", path: "/two_factor/challenge", auth: true },
    decodeAuth,
  );
});

export const submitChallenge = Effect.fn("submitChallenge")(function* (body: ChallengeSubmission) {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "POST", path: "/two_factor/challenge", body, auth: true },
    decodeAuth,
  );
});

export const consumeTransfer = Effect.fn("consumeTransfer")(function* (id: string) {
  const client = yield* ApiClient;
  const body: TransferSignIn = {};

  return yield* client.execute(
    { method: "PUT", path: `/session/transfers/${encodeURIComponent(id)}`, body, auth: true },
    decodeAuth,
  );
});

export const signOut = Effect.fn("signOut")(function* (body: SignOut) {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "DELETE", path: "/session", body, auth: true },
    decodeAuth,
  );
});
