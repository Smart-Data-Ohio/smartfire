/**
 * The signed-out auth contracts (slice 44): the public boot, password and Google sign-in, the
 * second-factor challenge and the sign-in link. The mock is always signed in for the rest of the
 * API, so a successful sign-in only has to answer where to go; the pending second step is kept
 * per world so `reset()` starts it over.
 *
 * Credentials in the mock: the mock password signs in `theo@smartdata.example` at once and sends
 * `riel@smartdata.example` (who has two-step sign-in) to the challenge. The authenticator code is
 * `123456` and the backup code `MOCK_BACKUP_CODE`. An email address or code starting `limit` is
 * rate limited. The sign-in link `mock-transfer-token` signs in, `mock-transfer-two-factor` goes
 * on to the challenge, and any other link is invalid.
 *
 * First run (slice 73) is open while `firstRunPending` is set; setting up closes it. A name
 * starting `fail` is refused as an unreadable submission, so the page's refusal shows, and
 * `firstRunUnavailable` fails the next check as an unreachable server would.
 */
import type { AuthResponse } from "../../src/gen/AuthResponse.ts";
import type { FirstRunState } from "../../src/gen/FirstRunState.ts";
import type { SignedOutBoot } from "../../src/gen/SignedOutBoot.ts";
import type { MockResponse } from "../http.ts";
import { type Json, parseJson, stringField } from "../json.ts";
import { MOCK_TOTP } from "./account.ts";
import type { AdminModule } from "./admin.ts";
import { type Route, route, type S2Context } from "./context.ts";
import { MOCK_PASSWORD } from "./settings.ts";

/** Signs in with the mock password, straight into the app. */
export const MOCK_PLAIN_EMAIL = "theo@smartdata.example";

/** Signs in with the mock password, then needs the second step. */
export const MOCK_TWO_FACTOR_EMAIL = "riel@smartdata.example";

/** The backup code the challenge accepts besides the authenticator code. */
export const MOCK_BACKUP_CODE = "a1b2c3d4e5";

/** The sign-in links. */
export const MOCK_TRANSFER_ID = "mock-transfer-token";

export const MOCK_TRANSFER_TWO_FACTOR_ID = "mock-transfer-two-factor";

/** Where a completed sign-in goes, as `return_to_after_authenticating` defaults it. */
export const SIGNED_IN_LOCATION = "/app/";

/** `sessions::REJECTION`: wrong credentials and the rate limit say the same. */
export const SIGN_IN_REJECTION = "Too many requests or unauthorized.";

export const WRONG_CODE =
  "That code didn't work. Check your authenticator app or try a backup code.";

export const CODE_RATE_ALERT = "Too many attempts. Try again in a few minutes.";

export const INVALID_TRANSFER = "This sign-in link is invalid or expired.";

/** `auth::decode`'s refusal of a submission it can't read. */
export const UNREADABLE_SUBMISSION = "The request body isn't valid.";

const GOOGLE_AUTHORIZE = "https://accounts.google.com/o/oauth2/v2/auth?client_id=mock";

interface State {
  /** Whether a first factor is waiting on its second step. */
  pending: boolean;
  google: boolean;
  firstRunPending: boolean;
  /** The last first-run submission as it arrived, for tests. */
  firstRun: Json;
  /** The next first-run check answers 503, as a server that can't be reached would. */
  firstRunUnavailable: boolean;
}

/** The sign-in module. */
export interface SignInModule {
  readonly routes: readonly Route[];
  /** `/__mock/sign-in`: Google configured, first run pending, a second step waiting. */
  configure(change: {
    readonly google: boolean | null;
    readonly firstRunPending: boolean | null;
    readonly pending: boolean | null;
    readonly firstRunUnavailable: boolean | null;
  }): void;
  /** The last first-run submission as it arrived (JSON, or the multipart form's fields). */
  firstRun(): Json;
}

const answer = (status: number, json: AuthResponse): MockResponse => ({ status, json });

const refusal = (status: number, field: string, message: string): MockResponse =>
  answer(status, { kind: "error", fieldErrors: { [field]: [message] } });

const CHALLENGE: AuthResponse = {
  kind: "secondFactorRequired",
  challenge: { methods: ["totp", "recoveryCode"], rememberDevice: true },
};

const SIGNED_IN: AuthResponse = { kind: "signedIn", location: SIGNED_IN_LOCATION };

const BACK_TO_SIGN_IN: AuthResponse = { kind: "navigate", location: "/session/new" };

/** Where first run sends a visitor once the workspace exists: home, as `prevent_repeats` does. */
const HOME: AuthResponse = { kind: "navigate", location: "/" };

/**
 * The first-run submission, from its JSON body or (with an avatar) from the multipart form's
 * `submission` field; `null` when it isn't one.
 */
function firstRunSubmission(body: Json | undefined): { readonly name: string } | null {
  const multipart = stringField(body, "submission");
  const submission = multipart === null ? body : parseJson(multipart);
  const name = stringField(submission, "name");

  if (
    name === null ||
    stringField(submission, "emailAddress") === null ||
    stringField(submission, "password") === null
  ) {
    return null;
  }

  return { name };
}

/** Creates the sign-in module. */
export function createSignIn(
  ctx: S2Context,
  admin: Pick<AdminModule, "branding" | "description">,
  csrfToken: () => string,
): SignInModule {
  let state: State | null = null;
  let stateWorld: ReturnType<S2Context["world"]> | null = null;

  const current = (): State => {
    const world = ctx.world();

    if (state === null || stateWorld !== world) {
      state = {
        pending: false,
        google: false,
        firstRunPending: false,
        firstRun: null,
        firstRunUnavailable: false,
      };
      stateWorld = world;
    }

    return state;
  };

  const boot = (): SignedOutBoot => {
    const held = current();
    const branding = admin.branding();

    return {
      kind: "signedOut",
      workspace: {
        name: branding.name,
        logoUrl: branding.logoUrl,
        description: admin.description(),
      },
      signInMethods: {
        password: true,
        google: held.google,
        googleDomains: held.google ? ["smartdata.example"] : [],
      },
      firstRunPending: held.firstRunPending,
      helpContact: { name: "Riel", emailAddress: MOCK_TWO_FACTOR_EMAIL },
      version: "2.0.0-mock",
      csrfToken: csrfToken(),
    };
  };

  return {
    firstRun: () => current().firstRun,
    configure(change) {
      const held = current();

      held.google = change.google ?? held.google;
      held.firstRunPending = change.firstRunPending ?? held.firstRunPending;
      held.pending = change.pending ?? held.pending;
      held.firstRunUnavailable = change.firstRunUnavailable ?? held.firstRunUnavailable;
    },
    routes: [
      route("GET", /^\/session\/boot$/, () => ({ status: 200, json: boot() })),
      route("POST", /^\/session$/, ({ body }) => {
        const email = (stringField(body, "emailAddress") ?? "").trim().toLowerCase();
        const password = stringField(body, "password") ?? "";

        if (email.startsWith("limit")) return refusal(429, "base", SIGN_IN_REJECTION);

        if (password !== MOCK_PASSWORD) return refusal(401, "base", SIGN_IN_REJECTION);

        if (email === MOCK_PLAIN_EMAIL) return answer(200, SIGNED_IN);

        if (email === MOCK_TWO_FACTOR_EMAIL) {
          current().pending = true;

          return answer(200, CHALLENGE);
        }

        return refusal(401, "base", SIGN_IN_REJECTION);
      }),
      route("POST", /^\/session\/google$/, () =>
        current().google
          ? answer(200, { kind: "navigate", location: GOOGLE_AUTHORIZE })
          : answer(404, { kind: "error", fieldErrors: {} }),
      ),
      route("GET", /^\/two_factor\/challenge$/, () =>
        answer(200, current().pending ? CHALLENGE : BACK_TO_SIGN_IN),
      ),
      route("POST", /^\/two_factor\/challenge$/, ({ body }) => {
        const held = current();
        const code = (stringField(body, "code") ?? "").replaceAll(" ", "");

        if (code.startsWith("limit")) return refusal(429, "code", CODE_RATE_ALERT);

        if (!held.pending) return answer(200, BACK_TO_SIGN_IN);

        if (code !== MOCK_TOTP && code !== MOCK_BACKUP_CODE) {
          return refusal(422, "code", WRONG_CODE);
        }

        held.pending = false;

        return answer(200, SIGNED_IN);
      }),
      route("PUT", new RegExp(`^/session/transfers/${MOCK_TRANSFER_ID}$`), () =>
        answer(200, SIGNED_IN),
      ),
      route("PUT", new RegExp(`^/session/transfers/${MOCK_TRANSFER_TWO_FACTOR_ID}$`), () => {
        current().pending = true;

        return answer(200, CHALLENGE);
      }),
      route("PUT", /^\/session\/transfers\/[^/]+$/, () => refusal(400, "base", INVALID_TRANSFER)),
      route("GET", /^\/first_run$/, () => {
        const held = current();

        if (held.firstRunUnavailable) {
          held.firstRunUnavailable = false;

          return { status: 503, json: { error: { message: "Service Unavailable" } } };
        }

        const state: FirstRunState = held.firstRunPending
          ? { kind: "pending", csrfToken: csrfToken() }
          : { kind: "navigate", location: "/" };

        return { status: 200, json: state };
      }),
      route("POST", /^\/first_run$/, ({ body }) => {
        const held = current();

        if (!held.firstRunPending) return answer(200, HOME);

        held.firstRun = body ?? null;

        const submission = firstRunSubmission(body);

        if (submission === null || submission.name.startsWith("fail")) {
          return refusal(422, "base", UNREADABLE_SUBMISSION);
        }

        held.firstRunPending = false;

        // The real server answers home, which takes a new administrator to two-step setup; the
        // mock has neither, so it answers the app it always serves signed in.
        return answer(200, SIGNED_IN);
      }),
    ],
  };
}
