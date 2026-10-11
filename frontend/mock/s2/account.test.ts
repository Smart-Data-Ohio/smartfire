import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import {
  AccountSettings as AccountSettingsSchema,
  BackupCodes as BackupCodesSchema,
  TwoFactorChange as TwoFactorChangeSchema,
} from "../../src/api/schema/settings.ts";
import { TwoFactorSetupResponse } from "../../src/api/schema/two-factor-setup.ts";
import type { AccountSettings } from "../../src/gen/AccountSettings.ts";
import type { BackupCodes } from "../../src/gen/BackupCodes.ts";
import type { TwoFactorChange } from "../../src/gen/TwoFactorChange.ts";
import { field, type Json } from "../json.ts";
import {
  MOCK_TOTP,
  RATE_ALERT,
  REAUTH_ALERT,
  SETUP_PROFILE,
  SETUP_WRONG,
  SIGNED_OUT_LOCATION,
} from "./account.ts";
import { MOCK_PASSWORD } from "./settings.ts";
import { errorOf, expectStatus, get, harness } from "./testing.ts";

const ACCOUNT = "/api/v1/settings/account";

describe("the mock's account panels", () => {
  it("list the viewer's rooms by name, shared then direct, with live levels", async () => {
    const { server } = harness();

    const account = Schema.decodeUnknownSync(AccountSettingsSchema)(
      await get<AccountSettings>(server, ACCOUNT),
    );

    expect(account.sharedRooms.length).toBeGreaterThan(0);
    expect(account.directRooms.length).toBeGreaterThan(0);
    expect(account.sharedRooms.every((row) => !row.direct)).toBe(true);
    expect(account.directRooms.every((row) => row.direct)).toBe(true);

    const names = account.sharedRooms.map((row) => row.name.toLowerCase());

    expect(names).toEqual([...names].sort((a, b) => a.localeCompare(b)));

    const first = account.sharedRooms[0];

    if (first === undefined) throw new Error("no shared room");

    await expectStatus(
      server,
      "PUT",
      `/api/v1/rooms/${first.roomId}/involvement`,
      { involvement: "nothing" },
      200,
    );

    const after = await get<AccountSettings>(server, ACCOUNT);

    expect(after.sharedRooms.find((row) => row.roomId === first.roomId)?.involvement).toBe(
      "nothing",
    );
  });

  it("refuse a wrong confirmation with the classic alert and limit retries", async () => {
    const { server } = harness();
    const path = "/api/v1/settings/two_factor/backup_codes";

    const wrong = await expectStatus<Json>(server, "POST", path, { reauth: "nope" }, 422);

    expect(errorOf(wrong)).toMatchObject({ tag: "Validation", message: REAUTH_ALERT });

    const limited = await expectStatus<Json>(server, "POST", path, { reauth: "limit" }, 429);

    expect(errorOf(limited)).toMatchObject({ tag: "RateLimited", message: RATE_ALERT });
  });

  it("make new codes, forget browsers, and turn two-step sign-in off", async () => {
    const { server } = harness();

    const first = Schema.decodeUnknownSync(BackupCodesSchema)(
      await expectStatus<BackupCodes>(
        server,
        "POST",
        "/api/v1/settings/two_factor/backup_codes",
        { reauth: MOCK_TOTP },
        200,
      ),
    );

    const second = await expectStatus<BackupCodes>(
      server,
      "POST",
      "/api/v1/settings/two_factor/backup_codes",
      { reauth: MOCK_PASSWORD },
      200,
    );

    expect(first.codes).toHaveLength(10);
    expect(second.codes).not.toEqual(first.codes);

    const one = Schema.decodeUnknownSync(TwoFactorChangeSchema)(
      await expectStatus<TwoFactorChange>(
        server,
        "DELETE",
        "/api/v1/settings/two_factor/devices/1",
        { reauth: MOCK_TOTP },
        200,
      ),
    );

    expect(one.notice).toBe("Device forgotten. It will ask for a code at next sign-in.");
    expect(one.twoFactor.devices.map((device) => device.id)).toEqual([2]);

    const all = await expectStatus<TwoFactorChange>(
      server,
      "DELETE",
      "/api/v1/settings/two_factor/devices",
      { reauth: MOCK_TOTP },
      200,
    );

    expect(all.notice).toBe(
      "All devices forgotten. Every browser will ask for a code at next sign-in.",
    );
    expect(all.twoFactor.devices).toEqual([]);

    const off = await expectStatus<TwoFactorChange>(
      server,
      "DELETE",
      "/api/v1/settings/two_factor",
      { reauth: MOCK_TOTP },
      200,
    );

    expect(off.notice).toBe("Two-step sign-in is off. Set it up again to keep signing in.");
    expect(off.twoFactor.confirmedAt).toBeNull();

    // The replacement session is unverified: the enrollment gate answers until setup is done.
    const again = await expectStatus<Json>(
      server,
      "DELETE",
      "/api/v1/settings/two_factor",
      { reauth: MOCK_TOTP },
      403,
    );

    expect(errorOf(again)).toMatchObject({
      tag: "TwoFactorRequired",
      message: "Set up two-step sign-in to continue",
    });
  });

  it("refuse a change once two-step sign-in was turned off elsewhere", async () => {
    const { server } = harness();

    await expectStatus(server, "POST", "/__mock/two-factor", { enrolled: false }, 200);

    const stale = await expectStatus<Json>(
      server,
      "DELETE",
      "/api/v1/settings/two_factor",
      { reauth: MOCK_TOTP },
      409,
    );

    expect(errorOf(stale)).toMatchObject({
      tag: "Conflict",
      message: "Set up two-step sign-in first.",
    });
  });

  it("send a test push to the viewer's own devices only", async () => {
    const { server } = harness();

    await expectStatus(server, "POST", "/api/v1/settings/push_subscriptions/4/test", null, 204);
    await expectStatus(server, "POST", "/api/v1/settings/push_subscriptions/99/test", null, 404);
  });
});

const SETUP = "/api/v1/two_factor/setup";

describe("the mock's two-step setup", () => {
  it("gates an unverified session to setup and sign-out, then confirms to one-time codes", async () => {
    const { server } = harness();

    await expectStatus(
      server,
      "POST",
      "/__mock/two-factor",
      { enrolled: false, required: true },
      200,
    );

    const gated = await expectStatus<Json>(server, "GET", ACCOUNT, null, 403);

    expect(errorOf(gated).tag).toBe("TwoFactorRequired");
    expect(field(field(gated, "error"), "requirement")).toEqual({
      kind: "setup",
      location: "/app/two_factor/setup",
    });

    const ready = Schema.decodeUnknownSync(TwoFactorSetupResponse)(await get<Json>(server, SETUP));

    if (ready.kind !== "ready") throw new Error(`expected ready, got ${ready.kind}`);

    expect(ready.setup.manualKey.replaceAll(" ", "")).toBe(ready.setup.secret);
    expect(ready.setup.qrSvg.startsWith("<svg")).toBe(true);
    // Visits reuse the pending key.
    expect(await get<Json>(server, SETUP)).toEqual(ready);

    const wrong = await expectStatus<Json>(server, "POST", SETUP, { code: "000000" }, 422);

    expect(wrong).toEqual({ kind: "error", message: SETUP_WRONG, setup: ready.setup });

    const limited = await expectStatus<Json>(server, "POST", SETUP, { code: "limit" }, 429);

    expect(limited).toMatchObject({ kind: "error", message: RATE_ALERT });

    const codes = await expectStatus<Json>(server, "POST", SETUP, { code: "123 456" }, 200);

    expect(codes).toMatchObject({ kind: "recoveryCodes", signedOut: 1 });
    expect(field(codes, "codes")).toHaveLength(10);
    await get(server, ACCOUNT);
    // The codes are shown once: setup now navigates to the profile.
    expect(await get<Json>(server, SETUP)).toEqual({ kind: "navigate", location: SETUP_PROFILE });
  });

  it("renews an expired setup with a new key and lets a gated session sign out", async () => {
    const { server } = harness();

    await expectStatus(
      server,
      "POST",
      "/__mock/two-factor",
      { enrolled: false, required: true },
      200,
    );

    const first = await get<Json>(server, SETUP);

    await expectStatus(server, "POST", "/__mock/two-factor", { expire: true }, 200);

    const expired = await expectStatus<Json>(server, "POST", SETUP, { code: MOCK_TOTP }, 422);

    expect(expired).toMatchObject({ kind: "error", message: SETUP_WRONG });
    expect(field(field(expired, "setup"), "secret")).not.toBe(
      field(field(first, "setup"), "secret"),
    );

    expect(await expectStatus<Json>(server, "DELETE", "/api/v1/session", null, 200)).toEqual({
      kind: "navigate",
      location: SIGNED_OUT_LOCATION,
    });
  });
});
