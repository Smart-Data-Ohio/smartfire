/**
 * Linked email addresses (`/email_addresses/*`), kept per world so `reset()` starts them over. The
 * viewer starts with one linked address besides the primary, which lives on the settings page.
 * Adding answers alike for any valid address (one starting `limit` is rate limited) and leaves it
 * waiting on the link `mock-email-token`, which then confirms it (or `extra@smartdata.example` when
 * nothing waits). `mock-email-signed-out` confirms as a signed-out browser does, going to sign in;
 * `mock-email-taken` finds its address claimed by another account; any other link is invalid.
 */
import type { AuthResponse } from "../../src/gen/AuthResponse.ts";
import type { EmailAddressConfirmation } from "../../src/gen/EmailAddressConfirmation.ts";
import type { EmailAddressEntry } from "../../src/gen/EmailAddressEntry.ts";
import type { EmailAddressList } from "../../src/gen/EmailAddressList.ts";
import { type MockResponse, noContent, notFound, ok, rateLimited, validation } from "../http.ts";
import { stringField } from "../json.ts";
import type { World } from "../seed.ts";
import { firstId, type Route, route, type S2Context } from "./context.ts";

/** The link that confirms the address waiting on it. */
export const MOCK_EMAIL_TOKEN = "mock-email-token";

/** A link opened in a signed-out browser: confirming goes on to sign in. */
export const MOCK_EMAIL_SIGNED_OUT_TOKEN = "mock-email-signed-out";

/** A link whose address another account linked first. */
export const MOCK_EMAIL_TAKEN_TOKEN = "mock-email-taken";

export const INVALID_EMAIL_LINK =
  "This confirmation link is invalid or expired. Send a new link from your profile settings.";

export const EMAIL_TAKEN = "This email address is already linked to another account.";

export const EMAIL_RATE_LIMITED = "Too many requests. Try again in an hour.";

/** The address the seed links besides the primary. */
export const SEEDED_LINKED_ADDRESS = "riel.personal@example.test";

/** What the module needs of the settings page, where the primary address lives. */
export interface PrimaryAddress {
  readonly get: () => string | null;
  readonly set: (emailAddress: string | null) => void;
}

interface State {
  nextId: number;
  secondary: EmailAddressEntry[];
  /** The address the last request emailed a link for. */
  waiting: string | null;
}

export interface EmailAddressesModule {
  readonly routes: readonly Route[];
}

const answer = (status: number, json: AuthResponse | EmailAddressConfirmation): MockResponse => ({
  status,
  json,
});

const refusal = (status: number, field: string, message: string): MockResponse =>
  answer(status, { kind: "error", fieldErrors: { [field]: [message] } });

export function createEmailAddresses(
  ctx: S2Context,
  primary: PrimaryAddress,
  requireSudo: () => void,
): EmailAddressesModule {
  let state: State | null = null;
  let stateWorld: World | null = null;

  const current = (): State => {
    const world = ctx.world();

    if (state === null || stateWorld !== world) {
      state = {
        nextId: 2,
        secondary: [
          {
            id: 1,
            emailAddress: SEEDED_LINKED_ADDRESS,
            createdAt: new Date(ctx.now() - 86_400_000).toISOString(),
          },
        ],
        waiting: null,
      };
      stateWorld = world;
    }

    return state;
  };

  const list = (): EmailAddressList => ({
    primary: primary.get(),
    secondary: [...current().secondary],
    canAdd: true,
  });

  const owned = (id: number): EmailAddressEntry => {
    const entry = current().secondary.find((each) => each.id === id);

    if (entry === undefined) throw notFound("Email address not found");

    return entry;
  };

  const link = (emailAddress: string) => {
    const held = current();

    if (held.secondary.some((each) => each.emailAddress === emailAddress)) return;

    held.secondary.push({
      id: held.nextId++,
      emailAddress,
      createdAt: new Date(ctx.now()).toISOString(),
    });
  };

  return {
    routes: [
      route("GET", /^\/email_addresses$/, () => ok(list())),
      route("POST", /^\/email_addresses$/, ({ body }) => {
        requireSudo();

        const emailAddress = (stringField(body, "emailAddress") ?? "").trim();

        if (!/^[^@\s]+@[^@\s]+$/.test(emailAddress)) {
          throw validation("emailAddress", "isn't valid");
        }

        if (emailAddress.toLowerCase().startsWith("limit")) {
          throw rateLimited(EMAIL_RATE_LIMITED, 3600);
        }

        current().waiting = emailAddress;

        return ok({ kind: "confirmationSent", emailAddress });
      }),
      route("DELETE", /^\/email_addresses\/(\d+)$/, (request) => {
        requireSudo();

        const entry = owned(firstId(request));
        const held = current();

        held.secondary = held.secondary.filter((each) => each !== entry);

        return noContent();
      }),
      route("POST", /^\/email_addresses\/(\d+)\/primary$/, (request) => {
        requireSudo();

        const entry = owned(firstId(request));
        const held = current();
        const previous = primary.get();

        held.secondary = held.secondary.flatMap((each) =>
          each === entry
            ? previous === null
              ? []
              : [{ ...each, emailAddress: previous }]
            : [each],
        );
        primary.set(entry.emailAddress);

        return noContent();
      }),
      route("GET", /^\/email_addresses\/confirmations\/([^/]+)$/, ({ path }) => {
        const token = decodeURIComponent(path.split("/").pop() ?? "");

        switch (token) {
          case MOCK_EMAIL_TOKEN:
            return answer(200, {
              emailAddress: current().waiting ?? "extra@smartdata.example",
              requiresSignIn: false,
            });
          case MOCK_EMAIL_SIGNED_OUT_TOKEN:
            return answer(200, { emailAddress: "phone@smartdata.example", requiresSignIn: true });
          case MOCK_EMAIL_TAKEN_TOKEN:
            return answer(200, { emailAddress: "shared@smartdata.example", requiresSignIn: false });
          default:
            return refusal(400, "base", INVALID_EMAIL_LINK);
        }
      }),
      route("PUT", /^\/email_addresses\/confirmations\/([^/]+)$/, ({ path }) => {
        const token = decodeURIComponent(path.split("/").pop() ?? "");

        switch (token) {
          case MOCK_EMAIL_TOKEN: {
            const held = current();

            link(held.waiting ?? "extra@smartdata.example");
            held.waiting = null;

            return answer(200, { kind: "navigate", location: "/app/settings" });
          }

          case MOCK_EMAIL_SIGNED_OUT_TOKEN:
            link("phone@smartdata.example");

            return answer(200, { kind: "navigate", location: "/app/session/new" });
          case MOCK_EMAIL_TAKEN_TOKEN:
            return refusal(422, "emailAddress", EMAIL_TAKEN);
          default:
            return refusal(400, "base", INVALID_EMAIL_LINK);
        }
      }),
    ],
  };
}
