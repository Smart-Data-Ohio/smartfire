import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import {
  BotChange as BotChangeSchema,
  BotKey as BotKeySchema,
  BotList as BotListSchema,
  Bot as BotSchema,
  CredentialCreated as CredentialCreatedSchema,
  GrantList as GrantListSchema,
} from "../../src/api/schema/bots.ts";
import type { Bot } from "../../src/gen/Bot.ts";
import type { BotChange } from "../../src/gen/BotChange.ts";
import type { BotKey } from "../../src/gen/BotKey.ts";
import type { BotList } from "../../src/gen/BotList.ts";
import type { CredentialCreated } from "../../src/gen/CredentialCreated.ts";
import type { CredentialList } from "../../src/gen/CredentialList.ts";
import type { GrantList } from "../../src/gen/GrantList.ts";
import type { PersonalAgent } from "../../src/gen/PersonalAgent.ts";
import type { PersonalAgentCreated } from "../../src/gen/PersonalAgentCreated.ts";
import type { PersonalAgentList } from "../../src/gen/PersonalAgentList.ts";
import type { PersonalCredentialCreated } from "../../src/gen/PersonalCredentialCreated.ts";
import { field, type Json } from "../json.ts";
import { BOT_ID } from "../seed.ts";
import { errorOf, expectStatus, get, harness, send } from "./testing.ts";

describe("the mock's bot pages", () => {
  it("shares personal credentials, grants and lifecycle across admin and owner routes", async () => {
    const { server } = harness();
    const choices = await get<PersonalAgentList>(server, "/api/v1/me/agents");
    const roomId = choices.rooms[0]?.id;
    expect(roomId).toBeDefined();

    const created = await expectStatus<PersonalAgentCreated>(
      server,
      "POST",
      "/api/v1/me/agents",
      {
        name: "Personal helper",
        roomIds: [roomId ?? 0],
        capabilities: ["read_messages"],
      },
      201,
    );

    const ownerPath = `/api/v1/me/agents/${created.agent.id}`;
    const adminPath = `/api/v1/admin/bots/${created.agent.userId}`;
    const original = created.agent.credentials[0];
    expect(original).toBeDefined();
    const credentials = await get<CredentialList>(server, `${adminPath}/credentials`);
    expect(credentials.credentials).toMatchObject([{ id: original?.id, state: "active" }]);
    await expectStatus<CredentialList>(
      server,
      "DELETE",
      `${adminPath}/credentials/${original?.id}`,
      null,
      200,
    );
    expect((await get<PersonalAgent>(server, ownerPath)).credentials[0]?.state).toBe("revoked");
    expect(
      (await send(server, "POST", `${ownerPath}/credentials/${original?.id}/rotate`, {})).status,
    ).toBe(404);

    const issued = await expectStatus<CredentialCreated>(
      server,
      "POST",
      `${adminPath}/credentials`,
      {
        name: "Admin token",
        expiresAt: null,
      },
      200,
    );

    const credentialId = issued.credentials.credentials[0]?.id;
    const ownerIssued = await get<PersonalAgent>(server, ownerPath);
    expect(ownerIssued.credentials[0]).toMatchObject({
      id: credentialId,
      name: "Admin token",
      state: "active",
    });

    const rotated = await expectStatus<PersonalCredentialCreated>(
      server,
      "POST",
      `${ownerPath}/credentials/${credentialId}/rotate`,
      {},
      201,
    );

    const adminRotated = await get<CredentialList>(server, `${adminPath}/credentials`);
    expect(adminRotated.credentials.find((c) => c.id === credentialId)?.state).toBe("revoked");
    expect(adminRotated.credentials[0]?.id).toBe(rotated.credential.id);
    await expectStatus<CredentialList>(
      server,
      "DELETE",
      `${adminPath}/credentials/${rotated.credential.id}`,
      null,
      200,
    );
    expect((await get<PersonalAgent>(server, ownerPath)).credentials[0]?.state).toBe("revoked");
    expect(
      (await send(server, "POST", `${ownerPath}/credentials/${rotated.credential.id}/rotate`, {}))
        .status,
    ).toBe(404);

    const initialGrants = await get<GrantList>(server, `${adminPath}/grants`);
    expect(initialGrants).toMatchObject({
      legacy: false,
      grants: [{ capability: "read_messages", status: "active" }],
    });

    const granted = await expectStatus<GrantList>(
      server,
      "POST",
      `${adminPath}/grants`,
      { capability: "react", roomId: roomId ?? 0 },
      200,
    );

    const react = granted.grants.find((g) => g.capability === "react");
    expect((await get<PersonalAgent>(server, ownerPath)).capabilities).toEqual([
      "react",
      "read_messages",
    ]);

    const again = await expectStatus<GrantList>(
      server,
      "POST",
      `${adminPath}/grants`,
      { capability: "react", roomId: roomId ?? 0 },
      200,
    );

    expect(again.grants).toHaveLength(2);
    await expectStatus<GrantList>(server, "DELETE", `${adminPath}/grants/${react?.id}`, null, 200);
    expect((await get<PersonalAgent>(server, ownerPath)).capabilities).toEqual(["read_messages"]);
    await expectStatus<PersonalAgent>(
      server,
      "PUT",
      `${ownerPath}/access`,
      { roomIds: [roomId ?? 0], capabilities: ["post_messages"] },
      200,
    );
    const replaced = await get<GrantList>(server, `${adminPath}/grants`);
    expect(replaced.grants.filter((g) => !g.revoked).map((g) => g.capability)).toEqual([
      "post_messages",
    ]);
    expect(replaced.grants.find((g) => g.capability === "read_messages")?.status).toBe("revoked");

    const edited = await expectStatus<BotChange>(
      server,
      "PATCH",
      adminPath,
      {
        name: "Renamed",
        iconName: "claude",
        agent: {
          provider: "Provider",
          runtime: "Runtime",
          description: "Updated",
          dailyMessageCap: "23",
          dailyBoardPostCap: "4",
          dailyExternalActionCap: "2",
        },
      },
      200,
    );

    expect(edited.bot.agent).toMatchObject({
      dailyMessageCap: 23,
      dailyBoardPostCap: 4,
      dailyExternalActionCap: 2,
    });
    expect(await get<PersonalAgent>(server, ownerPath)).toMatchObject({
      name: "Renamed",
      iconName: "claude",
      provider: "Provider",
      runtime: "Runtime",
      description: "Updated",
      dailyMessageCap: 23,
    });
    await expectStatus<PersonalAgent>(
      server,
      "PATCH",
      ownerPath,
      { name: "Owner rename", provider: "Owner provider", dailyMessageCap: 31 },
      200,
    );
    expect(await get<Bot>(server, adminPath)).toMatchObject({
      name: "Owner rename",
      agent: { provider: "Owner provider", dailyMessageCap: 31, dailyBoardPostCap: 4 },
    });
    expect(
      (await get<BotList>(server, "/api/v1/admin/bots")).bots.find(
        (b) => b.id === created.agent.userId,
      )?.name,
    ).toBe("Owner rename");

    const suspended = await expectStatus<BotChange>(
      server,
      "POST",
      `${adminPath}/kill_switch`,
      null,
      200,
    );

    expect(suspended.bot.agent?.suspended).toBe(true);
    expect(await get<PersonalAgent>(server, ownerPath)).toMatchObject({
      suspended: true,
      capabilities: [],
    });
    expect(
      (await get<GrantList>(server, `${adminPath}/grants`)).grants.find((g) => !g.revoked)?.status,
    ).toBe("inactive");
    await expectStatus<Json>(server, "DELETE", adminPath, null, 200);
    expect((await send(server, "GET", ownerPath)).status).toBe(404);
    expect((await get<PersonalAgentList>(server, "/api/v1/me/agents")).agents).toEqual([]);
    expect(
      (await get<BotList>(server, "/api/v1/admin/bots")).bots.some(
        (b) => b.id === created.agent.userId,
      ),
    ).toBe(false);
    expect((await send(server, "GET", adminPath)).status).toBe(404);
    server.reset();
    expect((await send(server, "GET", adminPath)).status).toBe(404);
    expect((await get<PersonalAgentList>(server, "/api/v1/me/agents")).agents).toEqual([]);
    expect((await get<Bot>(server, `/api/v1/admin/bots/${BOT_ID}`)).agent?.suspended).toBe(false);

    const fresh = await expectStatus<PersonalAgentCreated>(
      server,
      "POST",
      "/api/v1/me/agents",
      {
        name: "Fresh helper",
        roomIds: [],
        capabilities: [],
      },
      201,
    );

    expect(fresh.agent.credentials).toHaveLength(1);
    expect(fresh.agent.credentials[0]).toMatchObject({
      name: "Primary token",
      state: "active",
      connection: null,
    });
    expect(
      (await get<CredentialList>(server, `/api/v1/admin/bots/${fresh.agent.userId}/credentials`))
        .credentials,
    ).toHaveLength(1);
  });

  it("keeps personal policy and OAuth revocation in the shared admin record", async () => {
    const { server } = harness();

    const created = await expectStatus<PersonalAgentCreated>(
      server,
      "POST",
      "/api/v1/me/agents",
      {
        name: "Connected helper",
        roomIds: [],
        capabilities: [],
      },
      201,
    );

    const ownerPath = `/api/v1/me/agents/${created.agent.id}`;
    const adminPath = `/api/v1/admin/bots/${created.agent.userId}`;
    await expectStatus<Json>(
      server,
      "PATCH",
      adminPath,
      { name: "Must not save", webhookUrl: "https://agents.example/webhook" },
      422,
    );
    expect((await get<Bot>(server, adminPath)).name).toBe("Connected helper");

    for (const grant of [
      { capability: "react", roomId: null },
      { capability: "external_action", roomId: null },
      { capability: "react", roomId: 999999 },
    ]) {
      await expectStatus<Json>(server, "POST", `${adminPath}/grants`, grant, 422);
    }

    await expectStatus<Json>(
      server,
      "PUT",
      `${adminPath}/github_connection`,
      { accessToken: "" },
      422,
    );

    const linked = await expectStatus<BotChange>(
      server,
      "PUT",
      `${adminPath}/github_connection`,
      { accessToken: "mock-github-token" },
      200,
    );

    const signed = await expectStatus<BotChange>(
      server,
      "POST",
      `${adminPath}/webhook_secret`,
      null,
      200,
    );

    expect(signed.bot.signingSecret).toMatch(/^whsec_/);
    const reloaded = await get<Bot>(server, adminPath);
    expect(reloaded).toMatchObject({
      webhookUrl: null,
      github: linked.bot.github,
      signingSecret: signed.bot.signingSecret,
    });
    await expectStatus<BotChange>(server, "DELETE", `${adminPath}/github_connection`, null, 200);
    expect((await get<Bot>(server, adminPath)).github).toBeNull();
    await expectStatus<Json>(
      server,
      "POST",
      "/__mock/oauth-connection",
      { agentId: created.agent.id, clientName: "Connected app" },
      200,
    );
    const connected = (await get<PersonalAgent>(server, ownerPath)).credentials[0];
    expect(connected?.kind).toBe("oauth");
    await expectStatus<CredentialList>(
      server,
      "DELETE",
      `${adminPath}/credentials/${connected?.id}`,
      null,
      200,
    );
    await expectStatus<Json>(
      server,
      "POST",
      "/__mock/oauth-reuse",
      { credentialId: connected?.id ?? 0 },
      200,
    );
    expect((await get<PersonalAgent>(server, ownerPath)).credentials[0]).toMatchObject({
      state: "revoked",
      connection: { revokedReason: "tokenReused" },
    });
    expect(
      (await get<CredentialList>(server, `${adminPath}/credentials`)).credentials[0]?.state,
    ).toBe("revoked");
    await expectStatus<Json>(server, "DELETE", ownerPath, null, 200);
    expect((await send(server, "GET", adminPath)).status).toBe(404);
    expect((await send(server, "GET", `${adminPath}/credentials`)).status).toBe(404);
    server.reset();
    expect((await get<PersonalAgentList>(server, "/api/v1/me/agents")).agents).toEqual([]);
  });

  it("keeps revoked personal grant history and frees removed agents' creation slots", async () => {
    const { server } = harness();

    const room = await expectStatus<Json>(
      server,
      "POST",
      "/api/v1/rooms",
      {
        type: "closed",
        clientRoomId: "c248bb46-045a-440d-bd4a-876e8cbd1f16",
        name: "Agent room",
        iconName: null,
        userIds: [1, 2],
      },
      201,
    );

    const roomId = field(field(room, "room"), "id");
    await expectStatus<Json>(
      server,
      "PATCH",
      "/api/v1/admin/workspace",
      { memberAgentLimit: 1 },
      200,
    );
    await expectStatus<Json>(server, "POST", "/__mock/viewer-role", { role: "member" }, 200);

    const created = await expectStatus<PersonalAgentCreated>(
      server,
      "POST",
      "/api/v1/me/agents",
      {
        name: "Member helper",
        roomIds: [roomId ?? 0],
        capabilities: ["read_messages"],
      },
      201,
    );

    await expectStatus<Json>(server, "POST", "/__mock/viewer-role", { role: "administrator" }, 200);
    const adminPath = `/api/v1/admin/bots/${created.agent.userId}`;
    const ownerPath = `/api/v1/me/agents/${created.agent.id}`;

    for (const capability of ["external_action", "fizzy", "dm_anyone"]) {
      await expectStatus<Json>(
        server,
        "POST",
        `${adminPath}/grants`,
        { capability, roomId: roomId ?? 0 },
        422,
      );
    }

    await expectStatus<Json>(
      server,
      "PATCH",
      `/api/v1/rooms/${roomId}`,
      { type: "closed", userIds: [2] },
      200,
    );
    expect(await get<PersonalAgent>(server, ownerPath)).toMatchObject({
      roomIds: [],
      capabilities: [],
    });
    const history = await get<GrantList>(server, `${adminPath}/grants`);
    expect(history.grants).toMatchObject([
      { capability: "read_messages", revoked: true, status: "revoked" },
    ]);
    expect(history.rooms).toEqual([]);
    await expectStatus<Json>(
      server,
      "POST",
      `${adminPath}/grants`,
      { capability: "react", roomId: roomId ?? 0 },
      422,
    );
    await expectStatus<Json>(server, "DELETE", adminPath, null, 200);
    await expectStatus<Json>(server, "POST", "/__mock/viewer-role", { role: "member" }, 200);

    const replacement = await expectStatus<PersonalAgentCreated>(
      server,
      "POST",
      "/api/v1/me/agents",
      {
        name: "Replacement helper",
        roomIds: [],
        capabilities: [],
      },
      201,
    );

    expect(replacement.agent.credentials).toHaveLength(1);
    expect(replacement.agent.id).not.toBe(created.agent.id);
  });

  it("revokes personal grants when a room is deleted", async () => {
    const { server } = harness();
    const choices = await get<PersonalAgentList>(server, "/api/v1/me/agents");
    const roomId = choices.rooms[0]?.id;
    expect(roomId).toBeDefined();

    const created = await expectStatus<PersonalAgentCreated>(
      server,
      "POST",
      "/api/v1/me/agents",
      { name: "Room helper", roomIds: [roomId ?? 0], capabilities: ["read_messages"] },
      201,
    );

    await expectStatus<Json>(server, "DELETE", `/api/v1/rooms/${roomId}`, null, 200);
    expect(await get<PersonalAgent>(server, `/api/v1/me/agents/${created.agent.id}`)).toMatchObject(
      {
        roomIds: [],
        capabilities: [],
      },
    );

    const history = await get<GrantList>(
      server,
      `/api/v1/admin/bots/${created.agent.userId}/grants`,
    );

    expect(history.grants).toMatchObject([
      { capability: "read_messages", roomName: "Deleted room", revoked: true, status: "revoked" },
    ]);
  });

  it("keeps personal agents manageable while refusing bot keys", async () => {
    const { server } = harness();

    const created = await expectStatus<Json>(
      server,
      "POST",
      "/api/v1/me/agents",
      {
        name: "Personal helper",
        roomIds: [],
        capabilities: [],
      },
      201,
    );

    const userId = field(field(created, "agent"), "userId");
    const list = await get<BotList>(server, "/api/v1/admin/bots");
    const personal = list.bots.find((bot) => bot.id === userId);
    expect(personal).toMatchObject({ supportsBotKey: false });
    const detail = await get<Bot>(server, `/api/v1/admin/bots/${userId}`);
    Schema.decodeUnknownSync(BotSchema)(detail);
    expect(detail).toMatchObject({
      supportsBotKey: false,
      canAdminister: true,
      avatarAttached: false,
    });
    expect(detail.agent).not.toBeNull();

    const refused = await expectStatus<Json>(
      server,
      "PUT",
      `/api/v1/admin/bots/${userId}/key`,
      null,
      422,
    );

    expect(errorOf(refused).message).toBe("Bot key is not available to personal agents");
  });

  it("answer the contract's shapes", async () => {
    const { server } = harness();
    const list = await get<BotList>(server, "/api/v1/admin/bots");

    Schema.decodeUnknownSync(BotListSchema)(list);
    expect(list.bots.map((bot) => bot.id)).toContain(BOT_ID);

    Schema.decodeUnknownSync(BotSchema)(await get<Bot>(server, `/api/v1/admin/bots/${BOT_ID}`));
    Schema.decodeUnknownSync(GrantListSchema)(
      await get<GrantList>(server, `/api/v1/admin/bots/${BOT_ID}/grants`),
    );
  });

  it("make a bot and show its key", async () => {
    const { server } = harness();

    const key = await expectStatus<BotKey>(
      server,
      "POST",
      "/api/v1/admin/bots",
      { name: "Robo", iconName: null, webhookUrl: null, avatar: null },
      200,
    );

    Schema.decodeUnknownSync(BotKeySchema)(key);
    expect(key.key.startsWith(`${key.id}-`)).toBe(true);

    const refused = await expectStatus<Json>(
      server,
      "POST",
      "/api/v1/admin/bots",
      { name: " ", iconName: null, webhookUrl: null, avatar: null },
      422,
    );

    expect(Object.keys(field(field(refused, "error"), "fields") ?? {})).toEqual(["name"]);
  });

  it("refuse a bad budget by its field, as the classic form does", async () => {
    const { server } = harness();

    const refused = await expectStatus<Json>(
      server,
      "PATCH",
      `/api/v1/admin/bots/${BOT_ID}`,
      { agent: { dailyMessageCap: "lots" } },
      422,
    );

    expect(errorOf(refused).tag).toBe("Validation");

    const change = await expectStatus<BotChange>(
      server,
      "PATCH",
      `/api/v1/admin/bots/${BOT_ID}`,
      { name: "Helper", agent: { dailyMessageCap: "5", dailyBoardPostCap: "" } },
      200,
    );

    Schema.decodeUnknownSync(BotChangeSchema)(change);
    expect(change.bot.name).toBe("Helper");
    expect(change.bot.agent?.dailyMessageCap).toBe(5);
  });

  it("issue and revoke a credential, giving a legacy bot its agent", async () => {
    const { server } = harness();

    const legacy = (await get<BotList>(server, "/api/v1/admin/bots")).bots.find(
      (bot) => bot.name === "Deploy Bot",
    );

    expect(legacy).toBeDefined();

    const path = `/api/v1/admin/bots/${legacy?.id}`;

    expect((await get<Bot>(server, path)).agent).toBeNull();

    const created = await expectStatus<CredentialCreated>(
      server,
      "POST",
      `${path}/credentials`,
      { name: "ci", expiresAt: null },
      200,
    );

    Schema.decodeUnknownSync(CredentialCreatedSchema)(created);
    expect((await get<Bot>(server, path)).agent).not.toBeNull();

    const revoked = await expectStatus<CredentialList>(
      server,
      "DELETE",
      `${path}/credentials/${created.credentials.credentials[0]?.id}`,
      null,
      200,
    );

    expect(revoked.credentials[0]?.state).toBe("revoked");
  });

  it("grant once, and refuse a webhook secret to a bot with no webhook", async () => {
    const { server } = harness();
    const path = `/api/v1/admin/bots/${BOT_ID}/grants`;

    await send(server, "POST", path, { capability: "react", roomId: null });

    const again = await expectStatus<GrantList>(
      server,
      "POST",
      path,
      { capability: "react", roomId: null },
      200,
    );

    expect(again.grants).toHaveLength(1);
    expect(again.legacy).toBe(false);

    const legacy = (await get<BotList>(server, "/api/v1/admin/bots")).bots.find(
      (bot) => bot.name === "Deploy Bot",
    );

    const refused = await expectStatus<Json>(
      server,
      "POST",
      `/api/v1/admin/bots/${legacy?.id}/webhook_secret`,
      null,
      422,
    );

    expect(errorOf(refused).message).toBe("Set a webhook URL before generating a signing secret.");
  });
});
