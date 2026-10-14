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
import { field, type Json } from "../json.ts";
import { BOT_ID } from "../seed.ts";
import { errorOf, expectStatus, get, harness, send } from "./testing.ts";

describe("the mock's bot pages", () => {
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
    expect(detail).toMatchObject({ supportsBotKey: false, canAdminister: true });
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
