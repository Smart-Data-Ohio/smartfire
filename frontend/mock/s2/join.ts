/**
 * Joining the workspace (slice 46): the join code's page and the workspace invites' pages, and
 * signing up through either. The mock is always signed in for the rest of the API, so a signup
 * only has to answer where to go; who joined is kept per world so `reset()` starts it over.
 *
 * In the mock the join code is `MOCK_JOIN_CODE` (any other code is wrong). The invite
 * `MOCK_INVITE` has room for everyone, `MOCK_LAST_INVITE` for one more person, and the
 * `MOCK_DEAD_INVITES` are expired, used up and revoked; any other token is unknown. An address
 * that already has an account (the sign-in module's two) goes to sign-in.
 */
import type { AuthResponse } from "../../src/gen/AuthResponse.ts";
import type { InviteRefusal } from "../../src/gen/InviteRefusal.ts";
import type { JoinPage } from "../../src/gen/JoinPage.ts";
import type { MockResponse } from "../http.ts";
import { field, isRecord, type Json, stringField } from "../json.ts";
import type { AdminModule } from "./admin.ts";
import { type Route, route, type S2Context } from "./context.ts";
import { MOCK_PLAIN_EMAIL, MOCK_TWO_FACTOR_EMAIL, SIGNED_IN_LOCATION } from "./sign-in.ts";

export const MOCK_JOIN_CODE = "mock-join-code";

export const MOCK_INVITE = "mock-invite";

/** An invite with one use left. */
export const MOCK_LAST_INVITE = "mock-invite-last";

/** The dead invites, by why. */
export const MOCK_DEAD_INVITES = {
  expired: "mock-invite-expired",
  exhausted: "mock-invite-used",
  revoked: "mock-invite-revoked",
} as const;

/** `users::refusal_reason`, as the retained page says each. */
export const INVITE_REASONS: Readonly<Record<InviteRefusal, string>> = {
  expired: "It has expired.",
  exhausted: "All its uses have been taken.",
  revoked: "It has been revoked.",
  unknown: "The invite could not be found.",
};

/** Someone who joined, and the avatar they sent, if any. */
export interface MockJoined {
  readonly name: string;
  readonly emailAddress: string;
  readonly avatar: string | null;
}

/** The join module. */
export interface JoinModule {
  readonly routes: readonly Route[];
  /** Everyone who joined in this world, oldest first. */
  joined(): readonly MockJoined[];
}

interface State {
  joined: MockJoined[];
  lastInviteUsed: boolean;
}

const answer = (status: number, json: AuthResponse | JoinPage): MockResponse => ({
  status,
  json,
});

const EXISTING = new Set([MOCK_PLAIN_EMAIL, MOCK_TWO_FACTOR_EMAIL]);

/** The uploaded file's name, as the mock transports hand a multipart file part over. */
function fileName(value: Json | undefined): string | null {
  return isRecord(value) ? stringField(value, "filename") : null;
}

/** Creates the join module. */
export function createJoin(
  ctx: S2Context,
  admin: Pick<AdminModule, "branding" | "description">,
): JoinModule {
  let state: State | null = null;
  let stateWorld: ReturnType<S2Context["world"]> | null = null;

  const current = (): State => {
    const world = ctx.world();

    if (state === null || stateWorld !== world) {
      state = { joined: [], lastInviteUsed: false };
      stateWorld = world;
    }

    return state;
  };

  const workspace = (description: string) => {
    const branding = admin.branding();

    return { name: branding.name, logoUrl: branding.logoUrl, description };
  };

  const helpContact = { name: "Riel", emailAddress: MOCK_TWO_FACTOR_EMAIL };

  const page = (): JoinPage => ({
    kind: "join",
    workspace: workspace(admin.description()),
    helpContact,
  });

  const refusalOf = (token: string): InviteRefusal | null => {
    if (token === MOCK_INVITE) return null;

    if (token === MOCK_LAST_INVITE) return current().lastInviteUsed ? "exhausted" : null;

    if (token === MOCK_DEAD_INVITES.expired) return "expired";

    if (token === MOCK_DEAD_INVITES.exhausted) return "exhausted";

    if (token === MOCK_DEAD_INVITES.revoked) return "revoked";

    return "unknown";
  };

  const dead = (refusal: InviteRefusal): MockResponse =>
    answer(refusal === "unknown" ? 404 : 410, {
      kind: "inviteInvalid",
      workspace: workspace(""),
      helpContact,
      refusal,
      reason: INVITE_REASONS[refusal],
    });

  const wrongCode = answer(404, { kind: "error", fieldErrors: {} });

  const signUp = (body: Json | undefined, used: () => void): MockResponse => {
    const name = stringField(body, "name");
    const emailAddress = stringField(body, "emailAddress");
    const password = stringField(body, "password");

    if (name === null || emailAddress === null || password === null) {
      return answer(422, {
        kind: "error",
        fieldErrors: { base: ["The request body isn't valid."] },
      });
    }

    if (EXISTING.has(emailAddress.trim().toLowerCase())) {
      return answer(200, {
        kind: "navigate",
        location: `/session/new?email_address=${encodeURIComponent(emailAddress)}`,
      });
    }

    used();
    current().joined.push({ name, emailAddress, avatar: fileName(field(body, "avatar")) });

    return answer(200, { kind: "signedIn", location: SIGNED_IN_LOCATION });
  };

  const INVITE_PATH = /^\/invites\/([^/]+)$/;

  return {
    joined: () => current().joined,
    routes: [
      route("GET", new RegExp(`^/join/${MOCK_JOIN_CODE}$`), () => answer(200, page())),
      route("POST", new RegExp(`^/join/${MOCK_JOIN_CODE}$`), ({ body }) =>
        signUp(body, () => undefined),
      ),
      route("GET", /^\/join\/[^/]+$/, () => wrongCode),
      route("POST", /^\/join\/[^/]+$/, () => wrongCode),
      ...[MOCK_INVITE, MOCK_LAST_INVITE, ...Object.values(MOCK_DEAD_INVITES)].flatMap((known) => [
        route("GET", new RegExp(`^/invites/${known}$`), () => {
          const refusal = refusalOf(known);

          return refusal === null ? answer(200, page()) : dead(refusal);
        }),
        route("POST", new RegExp(`^/invites/${known}$`), ({ body }) => {
          const refusal = refusalOf(known);

          if (refusal !== null) return dead(refusal);

          return signUp(body, () => {
            if (known === MOCK_LAST_INVITE) current().lastInviteUsed = true;
          });
        }),
      ]),
      route("GET", INVITE_PATH, () => dead("unknown")),
      route("POST", INVITE_PATH, () => dead("unknown")),
    ],
  };
}
