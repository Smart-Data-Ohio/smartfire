import { Effect, Schema } from "effect";
import type { SudoSubmission } from "../gen/SudoSubmission.ts";
import { ApiClient } from "./client.ts";
import { SudoResponse } from "./schema/sudo.ts";

const decodeSudo = Schema.decodeUnknownEffect(SudoResponse);

export const readSudo = Effect.fn("readSudo")(function* () {
  const client = yield* ApiClient;

  return yield* client.execute({ method: "GET", path: "/sudo", auth: true }, decodeSudo);
});

export const submitSudo = Effect.fn("submitSudo")(function* (body: SudoSubmission) {
  const client = yield* ApiClient;

  return yield* client.execute({ method: "POST", path: "/sudo", body, auth: true }, decodeSudo);
});

export const startSudoGoogle = Effect.fn("startSudoGoogle")(function* () {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "POST", path: "/sudo/google", body: {}, auth: true },
    decodeSudo,
  );
});

export const resumeSudo = Effect.fn("resumeSudo")(function* () {
  const client = yield* ApiClient;

  return yield* client.execute({ method: "GET", path: "/sudo/continue", auth: true }, decodeSudo);
});
