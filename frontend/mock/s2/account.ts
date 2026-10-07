/**
 * The classic profile's lower panels (S7): the rooms the viewer is in, two-step sign-in and the
 * sign-in link, kept per world so `reset()` starts them over. Room levels are read from the
 * world, so a change through `PUT /rooms/:id/involvement` shows here at once.
 *
 * Confirmation in the mock: `123456` or the mock password is accepted, a value starting `limit`
 * is rate limited, anything else is refused with the classic alert.
 */
import type { AccountSettings } from "../../src/gen/AccountSettings.ts";
import type { RoomMembershipRow } from "../../src/gen/RoomMembershipRow.ts";
import type { TwoFactorSettings } from "../../src/gen/TwoFactorSettings.ts";
import { conflict, ok, rateLimited, refused } from "../http.ts";
import { type Json, stringField } from "../json.ts";
import { timestamp, type World } from "../seed.ts";
import { type Route, route, type S2Context } from "./context.ts";
import { MOCK_PASSWORD } from "./settings.ts";

const DAY_MS = 24 * 60 * 60_000;

/** The authenticator code the mock accepts. */
export const MOCK_TOTP = "123456";

/** The classic alert for a refused confirmation (an account with a password). */
export const REAUTH_ALERT = "Enter your authenticator code or password to continue.";

/** `two_factor::RATE_ALERT`. */
export const RATE_ALERT = "Too many attempts. Try again in a few minutes.";

/** The sign-in link the mock shows. */
export const MOCK_TRANSFER_URL = "http://localhost/session/transfers/mock-transfer-token";

const DISABLED_NOTICE = "Two-step sign-in is off. Set it up again to keep signing in.";

const FORGOT_ONE = "Device forgotten. It will ask for a code at next sign-in.";

const FORGOT_ALL = "All devices forgotten. Every browser will ask for a code at next sign-in.";

interface State {
  twoFactor: TwoFactorSettings;
  /** How many times the backup codes were replaced, so each set differs. */
  generations: number;
}

function initialState(now: number): State {
  return {
    twoFactor: {
      confirmedAt: timestamp(now - 90 * DAY_MS),
      google: false,
      hasPassword: true,
      devices: [
        {
          id: 1,
          description:
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15",
          ipAddress: "203.0.113.9",
          lastUsedAt: timestamp(now - 3 * 60 * 60_000),
        },
        {
          id: 2,
          description: "Mozilla/5.0 (X11; Linux x86_64; rv:142.0) Gecko/20100101 Firefox/142.0",
          ipAddress: null,
          lastUsedAt: timestamp(now - 4 * DAY_MS),
        },
      ],
    },
    generations: 0,
  };
}

/** The account module. */
export interface AccountModule {
  readonly routes: readonly Route[];
}

/** Creates the account module. */
export function createAccount(ctx: S2Context): AccountModule {
  let state: State | null = null;
  let stateWorld: World | null = null;

  const current = (): State => {
    const world = ctx.world();

    if (state === null || stateWorld !== world) {
      state = initialState(ctx.now());
      stateWorld = world;
    }

    return state;
  };

  /** Every room the viewer is in, by name as `Membership.with_ordered_room` lists them. */
  const rooms = (): readonly RoomMembershipRow[] =>
    [...ctx.world().rooms.values()]
      .map((record) => ({
        roomId: record.room.id,
        name: ctx.displayName(record),
        involvement: record.membership.involvement,
        direct: record.room.kind === "direct",
      }))
      .sort(
        (a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.roomId - b.roomId,
      );

  const account = (): AccountSettings => {
    const all = rooms();

    return {
      sharedRooms: all.filter((row) => !row.direct),
      directRooms: all.filter((row) => row.direct),
      twoFactor: current().twoFactor,
      transferUrl: MOCK_TRANSFER_URL,
    };
  };

  /** The classic `reauthenticated()` check, then two-step sign-in must be on. */
  const confirmed = (body: Json | undefined): State => {
    const held = current();
    const reauth = stringField(body, "reauth") ?? "";

    if (held.twoFactor.confirmedAt === null) throw conflict("Set up two-step sign-in first.");

    if (reauth.startsWith("limit")) {
      throw rateLimited(RATE_ALERT);
    }

    if (reauth !== MOCK_TOTP && reauth !== MOCK_PASSWORD) throw refused(REAUTH_ALERT);

    return held;
  };

  const changed = (held: State, notice: string) => ok({ notice, twoFactor: held.twoFactor });

  return {
    routes: [
      route("GET", /^\/settings\/account$/, () => ok(account())),
      route("POST", /^\/settings\/two_factor\/backup_codes$/, ({ body }) => {
        const held = confirmed(body);

        held.generations += 1;

        const codes = Array.from(
          { length: 10 },
          (_, index) => `${held.generations}${index}`.padStart(4, "0") + ctx.hex(6),
        );

        return ok({ codes });
      }),
      route("DELETE", /^\/settings\/two_factor$/, ({ body }) => {
        const held = confirmed(body);

        held.twoFactor = { ...held.twoFactor, confirmedAt: null, devices: [] };

        return changed(held, DISABLED_NOTICE);
      }),
      route("DELETE", /^\/settings\/two_factor\/devices\/(\d+)$/, ({ body, ids }) => {
        const held = confirmed(body);

        held.twoFactor = {
          ...held.twoFactor,
          devices: held.twoFactor.devices.filter((device) => device.id !== ids[0]),
        };

        return changed(held, FORGOT_ONE);
      }),
      route("DELETE", /^\/settings\/two_factor\/devices$/, ({ body }) => {
        const held = confirmed(body);

        held.twoFactor = { ...held.twoFactor, devices: [] };

        return changed(held, FORGOT_ALL);
      }),
    ],
  };
}
