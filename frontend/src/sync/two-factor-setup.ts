/**
 * What the two-step setup screen calls: plain promises over the setup contracts
 * (`/api/v1/two_factor/setup`) and the session's sign-out, all on the app's shared API client.
 * A wrong or expired code is an answer (`kind: "error"`), not a rejection; failures reject with
 * an `ActionError`.
 */

import { signOut } from "../api/auth-endpoints.ts";
import type { AuthResponse } from "../api/schema/auth.ts";
import type { TwoFactorSetupResponse } from "../api/schema/two-factor-setup.ts";
import {
  onTwoFactorRequirement,
  type TwoFactorRequirementListener,
} from "../api/two-factor-gate.ts";
import { readTwoFactorSetup, submitTwoFactorSetup } from "../api/two-factor-setup-endpoints.ts";
import { runAction } from "./runtime.ts";

/** A setup answer: `ready`, `error` (with the setup to retry), `recoveryCodes` or `navigate`. */
export type SetupAnswer = typeof TwoFactorSetupResponse.Type;

/** The provisioning material a `ready` or `error` answer carries. */
export type SetupState = Extract<SetupAnswer, { readonly kind: "ready" }>["setup"];

/** The sign-out's answer: where to go next. */
export type SignOutAnswer = typeof AuthResponse.Type;

export const twoFactorSetup = {
  /** Starts or resumes this browser's setup: its key and QR code, or where to go instead. */
  read: (): Promise<SetupAnswer> => runAction(readTwoFactorSetup()),
  /** Confirms the first code: the one-time backup codes, or the refusal with the setup to retry. */
  confirm: (code: string): Promise<SetupAnswer> => runAction(submitTwoFactorSetup({ code })),
  /** Signs this browser out (allowed before setup is done), dropping its push subscription too. */
  signOut: (pushSubscriptionEndpoint: string | null): Promise<SignOutAnswer> =>
    runAction(signOut({ pushSubscriptionEndpoint })),
  /** Leaves the screen with a full page load, so the app starts over on the verified session. */
  leave: (url: string): void => {
    window.location.assign(url);
  },
  /** Hears the API's enrollment gate; returns the unsubscribe. */
  onRequired: (listener: TwoFactorRequirementListener): (() => void) =>
    onTwoFactorRequirement(listener),
};
