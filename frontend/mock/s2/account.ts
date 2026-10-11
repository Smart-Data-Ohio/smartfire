/**
 * The classic profile's lower panels (S7): the rooms the viewer is in, two-step sign-in and the
 * sign-in link, kept per world so `reset()` starts them over. Room levels are read from the
 * world, so a change through `PUT /rooms/:id/involvement` shows here at once.
 *
 * Confirmation in the mock: `123456` or the mock password is accepted, a value starting `limit`
 * is rate limited, anything else is refused with the classic alert.
 *
 * Setup (`/two_factor/setup`) accepts `123456` too. Turning two-step sign-in off leaves the session
 * unverified, as the server's replacement session is: every other request then answers the
 * enrollment gate (403 `TwoFactorRequired`) until setup is confirmed or the browser signs out.
 * `/__mock/two-factor` puts the account in either state and expires the pending setup.
 */
import type { AccountSettings } from "../../src/gen/AccountSettings.ts";
import type { ApiError } from "../../src/gen/ApiError.ts";
import type { RoomMembershipRow } from "../../src/gen/RoomMembershipRow.ts";
import type { TwoFactorSettings } from "../../src/gen/TwoFactorSettings.ts";
import type { TwoFactorSetupResponse } from "../../src/gen/TwoFactorSetupResponse.ts";
import type { TwoFactorSetupState } from "../../src/gen/TwoFactorSetupState.ts";
import { conflict, HttpError, type MockResponse, ok, rateLimited, refused } from "../http.ts";
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

/** The sign-in link's QR code the mock shows: a stand-in drawing, not a scannable code. */
export const MOCK_TRANSFER_QR_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 3 3" width="240" height="240"><path d="M0 0h1v1H0zM2 0h1v1H2zM1 1h1v1H1zM0 2h1v1H0z"/></svg>';

const DISABLED_NOTICE = "Two-step sign-in is off. Set it up again to keep signing in.";

const FORGOT_ONE = "Device forgotten. It will ask for a code at next sign-in.";

const FORGOT_ALL = "All devices forgotten. Every browser will ask for a code at next sign-in.";

/** `two_factor/setups#create`'s alert for a wrong or expired code. */
export const SETUP_WRONG = "That code didn't work. Check your authenticator app and try again.";

/** Where the server sends an enrolled account's setup request: the profile. */
export const SETUP_PROFILE = "http://localhost/users/me/profile";

/** Where the server's sign-out sends the browser. */
export const SIGNED_OUT_LOCATION = "http://localhost/";

/** The enrollment gate's refusal, as `enforce_two_factor_for_restored_session` answers JSON. */
const SETUP_REQUIRED: ApiError = {
  _tag: "TwoFactorRequired",
  message: "Set up two-step sign-in to continue",
  requirement: { kind: "setup", location: "http://localhost/two_factor_setup" },
};

/** What an unverified session may still do: set up, or sign out. */
const GATE_EXEMPT = new Set(["GET /two_factor/setup", "POST /two_factor/setup", "DELETE /session"]);

/** A stand-in QR drawing for a setup key; the screen only ever shows it as an image. */
function setupQrSvg(secret: string): string {
  const cells = [...secret]
    .slice(0, 9)
    .map((char, index) =>
      char.charCodeAt(0) % 2 === 0 ? `M${index % 3} ${Math.floor(index / 3)}h1v1h-1z` : "",
    )
    .join("");

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 3 3" width="200" height="200"><path d="${cells}"/></svg>`;
}

interface State {
  twoFactor: TwoFactorSettings;
  /** How many times the backup codes were replaced, so each set differs. */
  generations: number;
  /** The session is unverified (turned off here, or set so): the enrollment gate is up. */
  required: boolean;
  /** This browser's pending setup key, once setup was read. */
  secret: string | null;
  /** The pending setup expired: the next confirmation is refused with a new key. */
  expired: boolean;
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
    required: false,
    secret: null,
    expired: false,
  };
}

/** The two-step state `/__mock/two-factor` sets; each field left out stays as it is. */
export interface TwoFactorControl {
  /** Two-step sign-in is on (off clears it and the remembered browsers). */
  readonly enrolled?: boolean | null;
  /** The enrollment gate is up: every other request answers `TwoFactorRequired`. */
  readonly required?: boolean | null;
  /** The pending setup has expired. */
  readonly expire?: boolean | null;
}

/** The account module. */
export interface AccountModule {
  readonly routes: readonly Route[];
  /** Throws the enrollment gate's refusal for a request an unverified session may not make. */
  readonly gate: (method: string, path: string) => void;
  readonly control: (change: TwoFactorControl) => void;
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
      transferQrSvg: MOCK_TRANSFER_QR_SVG,
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

  /** This browser's pending setup, made on first read and kept until confirmed or expired. */
  const setupState = (held: State): TwoFactorSetupState => {
    held.secret ??= ctx.hex(16).toUpperCase();

    const secret = held.secret;

    return {
      secret,
      manualKey: secret.match(/.{1,4}/g)?.join(" ") ?? secret,
      otpauthUri: `otpauth://totp/Smartfire:you%40example.com?secret=${secret}&issuer=Smartfire`,
      qrSvg: setupQrSvg(secret),
    };
  };

  const setupReply = (status: number, answer: TwoFactorSetupResponse): MockResponse =>
    ok(answer, status);

  const confirmSetup = (code: string): MockResponse => {
    const held = current();

    if (held.twoFactor.confirmedAt !== null) {
      return setupReply(200, { kind: "navigate", location: SETUP_PROFILE });
    }

    if (code.startsWith("limit")) {
      return setupReply(429, { kind: "error", message: RATE_ALERT, setup: setupState(held) });
    }

    if (held.expired) {
      held.expired = false;
      held.secret = null;

      return setupReply(422, { kind: "error", message: SETUP_WRONG, setup: setupState(held) });
    }

    if (code.replaceAll(" ", "") !== MOCK_TOTP) {
      return setupReply(422, { kind: "error", message: SETUP_WRONG, setup: setupState(held) });
    }

    held.generations += 1;
    held.twoFactor = { ...held.twoFactor, confirmedAt: timestamp(ctx.now()) };
    held.required = false;
    held.secret = null;

    const codes = Array.from(
      { length: 10 },
      (_, index) => `${held.generations}${index}`.padStart(4, "0") + ctx.hex(6),
    );

    return setupReply(200, {
      kind: "recoveryCodes",
      codes,
      signedOut: 1,
      continueUrl: "http://localhost/app/",
    });
  };

  const gate = (method: string, path: string): void => {
    if (current().required && !GATE_EXEMPT.has(`${method} ${path}`)) {
      throw new HttpError(403, SETUP_REQUIRED);
    }
  };

  const control = (change: TwoFactorControl): void => {
    const held = current();

    if (change.enrolled === false) {
      held.twoFactor = { ...held.twoFactor, confirmedAt: null, devices: [] };
    } else if (change.enrolled === true && held.twoFactor.confirmedAt === null) {
      held.twoFactor = { ...held.twoFactor, confirmedAt: timestamp(ctx.now()) };
    }

    if (change.required !== undefined && change.required !== null) held.required = change.required;

    if (change.expire === true) held.expired = true;
  };

  return {
    gate,
    control,
    routes: [
      route("GET", /^\/two_factor\/setup$/, () => {
        const held = current();

        if (held.twoFactor.confirmedAt !== null) {
          return setupReply(200, { kind: "navigate", location: SETUP_PROFILE });
        }

        return setupReply(200, { kind: "ready", setup: setupState(held) });
      }),
      route("POST", /^\/two_factor\/setup$/, ({ body }) =>
        confirmSetup(stringField(body, "code") ?? ""),
      ),
      route("DELETE", /^\/session$/, () => ok({ kind: "navigate", location: SIGNED_OUT_LOCATION })),
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
        // The server replaces the session with an unverified one: setup comes next.
        held.required = true;

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
