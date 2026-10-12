import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import { ApiErrorResponse } from "../../src/api/errors.ts";
import { CreateMessage, MessageDTO as MessageSchema } from "../../src/api/schema/message.ts";
import {
  StickerCatalog as CatalogSchema,
  MessageSticker,
  UpdateSticker,
} from "../../src/api/schema/sticker.ts";
import type { DirectUpload } from "../../src/gen/DirectUpload.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { StickerCatalog } from "../../src/gen/StickerCatalog.ts";
import { field } from "../json.ts";
import { type MockServer, SEED_IDS } from "../server.ts";
import { md5Base64 } from "./digest.ts";
import { STICKER_APNG, STICKER_CAPACITY, STICKER_GIF, STICKER_PNG } from "./stickers.ts";
import { collect, expectStatus, get, harness, send } from "./testing.ts";

const { rooms } = SEED_IDS;

async function uploaded(server: MockServer, bytes: Uint8Array): Promise<string> {
  const upload = await expectStatus<DirectUpload>(
    server,
    "POST",
    "/api/v1/uploads",
    {
      filename: "disguised.txt",
      contentType: "text/plain",
      byteSize: bytes.length,
      checksum: md5Base64(bytes),
    },
    200,
  );

  const reply = await server.handleBinary({
    method: "PUT",
    path: upload.uploadUrl,
    headers: { "Content-Type": "text/plain" },
    bytes,
  });

  expect(reply.status).toBe(204);

  return upload.signedId;
}

describe("sticker contracts and mock routes", () => {
  it("seeds static, GIF and APNG media with usable private original and still URLs", async () => {
    const { server } = harness();
    const catalog = await get<StickerCatalog>(server, "/api/v1/stickers");

    Schema.decodeUnknownSync(CatalogSchema)(catalog);
    expect(catalog.capacity).toBe(60);
    expect(catalog.stickers).toHaveLength(3);

    for (const sticker of catalog.stickers) {
      const original = await server.handleBinary({
        method: "GET",
        path: sticker.imageUrl,
        bytes: null,
      });

      const still = await server.handleBinary({
        method: "GET",
        path: sticker.stillUrl,
        bytes: null,
      });

      expect(original.status).toBe(200);
      expect(still.status).toBe(200);
      expect(still.contentType).toBe("image/png");
      expect(still.bytes).toEqual(STICKER_PNG);
    }

    Schema.decodeUnknownSync(CreateMessage)({
      clientMessageId: "sticker-only",
      markdownSource: "",
      stickerId: 2,
      replyToMessageId: null,
      replyNotifyAuthor: null,
      attachmentSignedId: null,
    });
    Schema.decodeUnknownSync(UpdateSticker)({ name: "Wave", description: "Hello", tags: "👋" });
    expect(() => Schema.decodeUnknownSync(MessageSticker)({ kind: "deleted", id: 2 })).toThrow();
  });

  it("posts one sticker with or without text, validates ids, and publishes deleted placeholders", async () => {
    const { server } = harness();
    const live = collect(server, [`room:${rooms.lounge}`]);
    const path = `/api/v1/rooms/${rooms.lounge}/messages`;

    const invalid = await send(server, "POST", path, {
      clientMessageId: "missing",
      stickerId: 9999,
    });

    expect(invalid.status).toBe(422);
    expect(field(field(invalid.json, "error"), "fields")).toEqual({ stickerId: ["must exist"] });

    for (const markdown of ["", "Hello!"]) {
      const message = await expectStatus<MessageDTO>(
        server,
        "POST",
        path,
        { clientMessageId: `sticker-${markdown}`, markdownSource: markdown, stickerId: 2 },
        201,
      );

      Schema.decodeUnknownSync(MessageSchema)(message);
      expect(message.sticker).toMatchObject({
        kind: "available",
        id: 2,
        name: "Dance",
        animated: true,
      });
    }

    expect(live.some((event) => event.type === "message.created")).toBe(true);
    await expectStatus(
      server,
      "PATCH",
      "/api/v1/admin/stickers/2",
      { name: "Waving", description: "Hi", tags: "👋" },
      200,
    );
    await expectStatus(server, "DELETE", "/api/v1/admin/stickers/2", null, 200);

    const again = await expectStatus<MessageDTO>(
      server,
      "POST",
      path,
      { clientMessageId: "sticker-", stickerId: 2 },
      200,
    );

    expect(again.sticker).toEqual({ kind: "deleted", name: "Waving" });
    Schema.decodeUnknownSync(MessageSchema)(again);

    const edited = await expectStatus<MessageDTO>(
      server,
      "PATCH",
      `/api/v1/messages/${again.id}`,
      { markdownSource: "" },
      200,
    );

    expect(edited.sticker).toEqual(again.sticker);
    expect(
      live.some(
        (event) => event.type === "message.updated" && event.data.sticker?.kind === "deleted",
      ),
    ).toBe(true);
    expect(
      (await server.handleBinary({ method: "GET", path: "/stickers/2", bytes: null })).status,
    ).toBe(404);
  });

  it("creates byte-detected formats, enforces names, dimensions, size, cap and admin permissions", async () => {
    const { server } = harness();

    for (const [name, bytes, animated] of [
      ["Static upload", STICKER_PNG, false],
      ["GIF upload", STICKER_GIF, true],
      ["APNG upload", STICKER_APNG, true],
    ] as const) {
      const signedId = await uploaded(server, bytes);

      const added = await expectStatus<StickerCatalog>(
        server,
        "POST",
        "/api/v1/admin/stickers",
        { name, description: "Hi", tags: "👋", signedId },
        201,
      );

      expect(added.stickers.find((sticker) => sticker.name === name)?.animated).toBe(animated);
    }

    const signedId = await uploaded(server, STICKER_PNG);

    for (const name of ["x", "a".repeat(31), "wave"]) {
      expect(
        (await send(server, "POST", "/api/v1/admin/stickers", { name, signedId })).status,
      ).toBe(422);
    }

    const oversized = new Uint8Array(512 * 1024 + 1);
    const bigId = await uploaded(server, oversized);

    expect(
      (await send(server, "POST", "/api/v1/admin/stickers", { name: "Big", signedId: bigId }))
        .status,
    ).toBe(422);
    const wide = STICKER_PNG.slice();

    new DataView(wide.buffer).setUint32(16, 321);
    const wideId = await uploaded(server, wide);

    expect(
      (await send(server, "POST", "/api/v1/admin/stickers", { name: "Wide", signedId: wideId }))
        .status,
    ).toBe(422);

    for (let index = 6; index < STICKER_CAPACITY; index += 1) {
      await expectStatus(
        server,
        "POST",
        "/api/v1/admin/stickers",
        { name: `Sticker ${index}`, signedId },
        201,
      );
    }

    const refused = await send(server, "POST", "/api/v1/admin/stickers", {
      name: "Overflow",
      signedId,
    });

    expect(refused.status).toBe(422);
    Schema.decodeUnknownSync(ApiErrorResponse)(refused.json);
    expect(field(field(refused.json, "error"), "_tag")).toBe("StickerLimitReached");

    await expectStatus(server, "DELETE", "/api/v1/admin/stickers/1", null, 200);
    await expectStatus(
      server,
      "POST",
      "/api/v1/admin/stickers",
      { name: "Replacement", signedId },
      201,
    );
    server.reset();
    expect((await get<StickerCatalog>(server, "/api/v1/stickers")).stickers).toHaveLength(3);
    await server.handle({ method: "POST", path: "/__mock/viewer-role", body: { role: "member" } });
    expect(
      (await send(server, "POST", "/api/v1/admin/stickers", { name: "Member", signedId })).status,
    ).toBe(403);
    expect((await get<StickerCatalog>(server, "/api/v1/stickers")).capacity).toBe(60);
  });
});
