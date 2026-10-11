import { describe, expect, it } from "vitest";
import type { ChannelWebhook } from "../../src/gen/ChannelWebhook.ts";
import type { ChannelWebhookCredential } from "../../src/gen/ChannelWebhookCredential.ts";
import type { ChannelWebhookList } from "../../src/gen/ChannelWebhookList.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import { errorOf, expectStatus, get, harness, send } from "../s2/testing.ts";
import { SEED_IDS } from "../server.ts";

const { rooms, boards } = SEED_IDS;

describe("channel webhook mock", () => {
  it("creates, renames, tests, regenerates and deletes, showing the token only when issued", async () => {
    const { server } = harness();
    const path = `/api/v1/rooms/${rooms.launchPlanning}/webhooks`;

    expect((await get<ChannelWebhookList>(server, path)).webhooks).toEqual([]);

    const created = await expectStatus<ChannelWebhookCredential>(
      server,
      "POST",
      path,
      { name: " Deployments ", avatarSignedId: null },
      201,
    );

    expect(created.webhook.name).toBe("Deployments");
    expect(created.url).toBe(
      `http://127.0.0.1/api/webhooks/${created.webhook.id}/${created.token}`,
    );

    const listed = await get<ChannelWebhookList>(server, path);

    expect(listed.webhooks).toEqual([created.webhook]);
    expect(JSON.stringify(listed)).not.toContain(created.token);

    const blank = await send(server, "POST", path, { name: "  ", avatarSignedId: null });

    expect(blank.status).toBe(422);

    const hook = `${path}/${created.webhook.id}`;
    const renamed = await expectStatus<ChannelWebhook>(server, "PATCH", hook, { name: "CI" }, 200);

    expect(renamed.name).toBe("CI");

    const message = await expectStatus<MessageDTO>(server, "POST", `${hook}/test`, null, 200);

    expect(message.webhook).toEqual({ name: "CI", username: null, avatarUrl: null });
    expect(message.markdownSource).toBe("This is a test webhook message.");

    const regenerated = await expectStatus<ChannelWebhookCredential>(
      server,
      "POST",
      `${hook}/regenerate_token`,
      null,
      200,
    );

    expect(regenerated.token).not.toBe(created.token);
    expect((await get<ChannelWebhookList>(server, path)).webhooks[0]?.lastUsedAt).toBe(
      message.createdAt,
    );

    await expectStatus(server, "DELETE", hook, null, 204);
    expect((await get<ChannelWebhookList>(server, path)).webhooks).toEqual([]);
    expect((await send(server, "DELETE", hook)).status).toBe(404);
  });

  it("refuses a board's test post, a direct room, and anyone who can't manage the room", async () => {
    const { server } = harness();
    const board = `/api/v1/rooms/${boards.roomId}/webhooks`;

    const created = await expectStatus<ChannelWebhookCredential>(
      server,
      "POST",
      board,
      { name: "Board hook", avatarSignedId: null },
      201,
    );

    const refusedPost = await send(server, "POST", `${board}/${created.webhook.id}/test`);

    expect(refusedPost.status).toBe(422);
    expect((await send(server, "GET", `/api/v1/rooms/${rooms.dmMaya}/webhooks`)).status).toBe(404);

    await send(server, "POST", "/__mock/viewer-role", { role: "member" });
    const refused = await send(server, "GET", `/api/v1/rooms/${rooms.launchPlanning}/webhooks`);

    expect(refused.status).toBe(403);
    expect(errorOf(refused.json).message).toBe("Not allowed");
  });
});
