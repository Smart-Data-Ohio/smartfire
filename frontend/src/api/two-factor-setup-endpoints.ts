import { Effect, Schema } from "effect";
import type { TwoFactorSetupSubmission } from "../gen/TwoFactorSetupSubmission.ts";
import { ApiClient } from "./client.ts";
import { TwoFactorSetupResponse } from "./schema/two-factor-setup.ts";

const decodeSetup = Schema.decodeUnknownEffect(TwoFactorSetupResponse);

export const readTwoFactorSetup = Effect.fn("readTwoFactorSetup")(function* () {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "GET", path: "/two_factor/setup", auth: true },
    decodeSetup,
  );
});

export const submitTwoFactorSetup = Effect.fn("submitTwoFactorSetup")(function* (
  body: TwoFactorSetupSubmission,
) {
  const client = yield* ApiClient;

  return yield* client.execute(
    { method: "POST", path: "/two_factor/setup", body, auth: true },
    decodeSetup,
  );
});
