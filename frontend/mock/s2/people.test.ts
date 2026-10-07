import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import {
  PeopleDirectory as PeopleDirectorySchema,
  PersonProfile as PersonProfileSchema,
} from "../../src/api/schema/people.ts";
import type { PeopleDirectory } from "../../src/gen/PeopleDirectory.ts";
import type { PersonProfile } from "../../src/gen/PersonProfile.ts";
import type { Json } from "../json.ts";
import { BOT_ID, DEACTIVATED_ID, USER_IDS, VIEWER_ID } from "../seed.ts";
import { errorOf, expectStatus, get, harness, send } from "./testing.ts";

describe("the mock's people pages", () => {
  it("lists everyone active but the viewer, starred first, then by name", async () => {
    const { server } = harness();

    await send(server, "PUT", `/api/v1/users/${USER_IDS.theo}/star`);

    const directory = await get<PeopleDirectory>(server, "/api/v1/people");

    Schema.decodeUnknownSync(PeopleDirectorySchema)(directory);

    const ids = directory.people.map((person) => person.userId);

    const starred = directory.people.filter((person) => person.starred).map((p) => p.userId);

    expect(starred).toContain(USER_IDS.theo);
    expect(ids.slice(0, starred.length)).toEqual(starred);
    expect(ids).not.toContain(VIEWER_ID);
    expect(ids).not.toContain(DEACTIVATED_ID);
    expect(directory.people.find((person) => person.userId === BOT_ID)?.agent).toBe(true);
    expect(directory.users.map((user) => user.id)).toEqual(ids);
  });

  it("shows an administrator a person's email, sign-in link and ban button", async () => {
    const { server } = harness();
    const profile = await get<PersonProfile>(server, `/api/v1/people/${USER_IDS.theo}`);

    Schema.decodeUnknownSync(PersonProfileSchema)(profile);
    expect(profile.emailAddress).toBe("theo@smartdata.example");
    expect(profile.transferUrl).not.toBeNull();
    expect(profile.transferQrSvg).not.toBeNull();
    expect(profile.canBan).toBe(true);
    expect(profile.dndAllowed).not.toBeNull();
    expect(profile.status?.presence).toBe("online");
  });

  it("offers no DND toggle or ban button on your own page", async () => {
    const { server } = harness();
    const profile = await get<PersonProfile>(server, `/api/v1/people/${VIEWER_ID}`);

    expect(profile.dndAllowed).toBeNull();
    expect(profile.canBan).toBe(false);
    expect(profile.transferUrl).not.toBeNull();
  });

  it("gives bots and deactivated people no status", async () => {
    const { server } = harness();
    const bot = await get<PersonProfile>(server, `/api/v1/people/${BOT_ID}`);
    const gone = await get<PersonProfile>(server, `/api/v1/people/${DEACTIVATED_ID}`);

    expect(bot.status).toBeNull();
    expect(bot.canBan).toBe(false);
    expect(gone.status).toBeNull();
    expect(gone.emailAddress).toBeNull();
  });

  it("bans and removes the ban, asking for the password once it has lapsed", async () => {
    const { server } = harness();
    const path = `/api/v1/people/${USER_IDS.sam}/ban`;

    const banned = await expectStatus<PersonProfile>(server, "POST", path, null, 200);

    expect(banned.user.status).toBe("banned");
    expect(banned.transferUrl).toBeNull();
    expect(banned.canBan).toBe(true);

    const directory = await get<PeopleDirectory>(server, "/api/v1/people");

    expect(directory.people.map((person) => person.userId)).not.toContain(USER_IDS.sam);

    await send(server, "POST", "/__mock/lapse-sudo", { on: true });

    const lapsed = await expectStatus<Json>(server, "DELETE", path, null, 403);

    expect(errorOf(lapsed).tag).toBe("SudoRequired");
    await send(server, "POST", "/__mock/lapse-sudo", { on: false });

    const restored = await expectStatus<PersonProfile>(server, "DELETE", path, null, 200);

    expect(restored.user.status).toBe("active");
  });

  it("keeps stars and DND exceptions apart, as their separate tables do", async () => {
    const { server } = harness();
    const profileOf = (id: number) => get<PersonProfile>(server, `/api/v1/people/${id}`);

    await send(server, "PUT", `/api/v1/users/${USER_IDS.sam}/star`);
    expect((await profileOf(USER_IDS.sam)).dndAllowed).toBe(false);

    await send(server, "POST", `/api/v1/settings/dnd_allowances/${USER_IDS.jonah}`);
    expect((await profileOf(USER_IDS.jonah)).dndAllowed).toBe(true);

    const directory = await get<PeopleDirectory>(server, "/api/v1/people");

    expect(directory.people.find((person) => person.userId === USER_IDS.jonah)?.starred).toBe(
      false,
    );
  });

  it("answers 404 for someone who doesn't exist", async () => {
    const { server } = harness();

    await expectStatus<Json>(server, "GET", "/api/v1/people/999", null, 404);
    await expectStatus<Json>(server, "POST", "/api/v1/people/999/ban", null, 404);
  });
});
