import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import {
  AccountSettings as AccountSettingsSchema,
  BackupCodes as BackupCodesSchema,
  TwoFactorChange as TwoFactorChangeSchema,
} from "../../src/api/schema/settings.ts";
import type { AccountSettings } from "../../src/gen/AccountSettings.ts";
import type { BackupCodes } from "../../src/gen/BackupCodes.ts";
import type { TwoFactorChange } from "../../src/gen/TwoFactorChange.ts";
import type { Json } from "../json.ts";
import { MOCK_TOTP, RATE_ALERT, REAUTH_ALERT } from "./account.ts";
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

    const again = await expectStatus<Json>(
      server,
      "DELETE",
      "/api/v1/settings/two_factor",
      { reauth: MOCK_TOTP },
      409,
    );

    expect(errorOf(again)).toMatchObject({
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
