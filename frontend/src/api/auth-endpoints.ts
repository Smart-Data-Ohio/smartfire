import { Effect, Schema } from "effect";
import type { ChallengeSubmission } from "../gen/ChallengeSubmission.ts";
import type { FirstRunSubmission } from "../gen/FirstRunSubmission.ts";
import type { GoogleSignInStart } from "../gen/GoogleSignInStart.ts";
import type { PasswordSignIn } from "../gen/PasswordSignIn.ts";
import type { SignOut } from "../gen/SignOut.ts";
import type { TransferSignIn } from "../gen/TransferSignIn.ts";
import type { WorkspaceJoin } from "../gen/WorkspaceJoin.ts";
import { ApiClient } from "./client.ts";
import { AuthResponse, FirstRunState, JoinAnswer, SignedOutBoot } from "./schema/auth.ts";

const decodeAuth = Schema.decodeUnknownEffect(AuthResponse);

const decodeBoot = Schema.decodeUnknownEffect(SignedOutBoot);

const decodeFirstRun = Schema.decodeUnknownEffect(FirstRunState);

const decodeJoin = Schema.decodeUnknownEffect(JoinAnswer);

/** How a visitor joins: the workspace's join code, or a workspace invite's token. */
export type JoinTarget =
  | { readonly via: "code"; readonly code: string }
  | { readonly via: "invite"; readonly token: string };

const joinPath = (target: JoinTarget): string =>
  target.via === "code"
    ? `/join/${encodeURIComponent(target.code)}`
    : `/invite/${encodeURIComponent(target.token)}`;

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

/** `GET /first_run`: open (adopting its token) or, once set up, where to go instead. */
export const readFirstRun = Effect.fn("readFirstRun")(function* () {
  const client = yield* ApiClient;

  const state = yield* client.execute(
    { method: "GET", path: "/first_run", auth: true },
    decodeFirstRun,
  );

  if (state.kind === "pending") yield* client.setCsrfToken(state.csrfToken);

  return state;
});

/**
 * `POST /first_run`: the first administrator. With an avatar it goes as a multipart form, the
 * submission's JSON in `submission` beside the picture, as the server reads it.
 */
export const submitFirstRun = Effect.fn("submitFirstRun")(function* (
  body: FirstRunSubmission,
  avatar: File | null,
) {
  const client = yield* ApiClient;

  if (avatar === null) {
    return yield* client.execute(
      { method: "POST", path: "/first_run", body, auth: true },
      decodeAuth,
    );
  }

  const form = new FormData();

  form.append("submission", JSON.stringify(body));
  form.append("avatar", avatar);

  return yield* client.execute(
    { method: "POST", path: "/first_run", form, auth: true },
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

/** `GET /join/:code` or `/invite/:token`: the page's state, or where a signed-in visitor goes. */
export const readJoinPage = Effect.fn("readJoinPage")(function* (target: JoinTarget) {
  const client = yield* ApiClient;

  return yield* client.execute({ method: "GET", path: joinPath(target), auth: true }, decodeJoin);
});

/** `POST /join/:code` or `/invite/:token`: the fields, and the avatar as a file part. */
export const submitJoin = Effect.fn("submitJoin")(function* (
  target: JoinTarget,
  fields: WorkspaceJoin,
  avatar: File | null,
) {
  const client = yield* ApiClient;
  const form = new FormData();

  form.set("name", fields.name);
  form.set("emailAddress", fields.emailAddress);
  form.set("password", fields.password);

  if (avatar !== null) form.set("avatar", avatar);

  return yield* client.execute(
    { method: "POST", path: joinPath(target), form, auth: true },
    decodeJoin,
  );
});
