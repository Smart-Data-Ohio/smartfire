/**
 * What the signed-out pages (sign-in, the second-factor challenge, the sign-in link) call: plain
 * promises over the slice-44 auth contracts. Each answer is the server's next action; a failure
 * that never reached a decision (the network, an unreadable body) rejects with an `ActionError`.
 */

import { Option, Schema } from "effect";
import {
  consumeTransfer,
  googleSignIn,
  type JoinTarget,
  passwordSignIn,
  readChallenge,
  readJoinPage,
  signedOutBoot,
  submitChallenge,
  submitJoin,
} from "../api/auth-endpoints.ts";
import { type AuthResponse, type JoinAnswer, SignedOutBoot } from "../api/schema/auth.ts";
import type { WorkspaceJoin } from "../gen/WorkspaceJoin.ts";
import { runAction } from "./runtime.ts";

/** The public boot: the workspace's branding, the sign-in methods and the help line. */
export type SignedOutBootData = typeof SignedOutBoot.Type;

/** A sign-in operation's next action. */
export type AuthNext = typeof AuthResponse.Type;

/** A join endpoint's answer: the join page's state, or a next action. */
export type JoinNext = typeof JoinAnswer.Type;

export type { JoinTarget };

const decodeInline = Schema.decodeUnknownOption(Schema.fromJsonString(SignedOutBoot));

/**
 * The boot JSON the Rust shell inlined for a signed-out visitor, or `null` when the page has none
 * or holds the signed-in boot (a signed-in visitor at sign-in, or the Vite dev page).
 */
export function inlineSignedOutBoot(): SignedOutBootData | null {
  const text = document.getElementById("boot")?.textContent;

  if (text === undefined || text === null || text === "") {
    return null;
  }

  return Option.getOrNull(decodeInline(text));
}

export const auth = {
  /** `GET /session/boot`; the client adopts its CSRF token. */
  boot: (): Promise<SignedOutBootData> => runAction(signedOutBoot()),
  signIn: (emailAddress: string, password: string): Promise<AuthNext> =>
    runAction(passwordSignIn({ emailAddress, password })),
  /** Starts Google sign-in; the answer navigates to Google's authorization page. */
  google: (): Promise<AuthNext> => runAction(googleSignIn()),
  /** The pending challenge, or where to go when there is none. */
  challenge: (): Promise<AuthNext> => runAction(readChallenge()),
  verify: (code: string, rememberDevice: boolean): Promise<AuthNext> =>
    runAction(submitChallenge({ code, rememberDevice })),
  transfer: (id: string): Promise<AuthNext> => runAction(consumeTransfer(id)),
  /** The join or invite page: the form, a dead invite's reason, or where to go instead. */
  joinPage: (target: JoinTarget): Promise<JoinNext> => runAction(readJoinPage(target)),
  /** Creates the account and signs it in, or says why not. */
  join: (target: JoinTarget, fields: WorkspaceJoin, avatar: File | null): Promise<JoinNext> =>
    runAction(submitJoin(target, fields, avatar)),
};
