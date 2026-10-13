import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import { GifCategoryList, GifConfig, GifList, GifSuggestions } from "../../src/api/schema/gif.ts";
import { CreateMessage, MessageDTO as MessageSchema } from "../../src/api/schema/message.ts";
import type { GifList as GifListWire } from "../../src/gen/GifList.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { ThreadCreated } from "../../src/gen/ThreadCreated.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import { GIF_BYTES, GIF_FIXTURES, GIF_STILL_BYTES } from "../fixtures/gifs.ts";
import { createMockServer, SEED_IDS } from "../server.ts";
import { expectStatus, get, harness, messageBody, send } from "./testing.ts";

const roomId = SEED_IDS.rooms.lounge;

describe("GIF picker API fixtures", () => {
  it("decodes config, search, categories and terms without a provider key", async () => {
    const { server } = harness();

    expect(Schema.decodeUnknownSync(GifConfig)(await get(server, "/api/v1/gifs/config"))).toEqual({
      enabled: true,
    });
    const trending = await get<GifListWire>(server, "/api/v1/gifs/trending");

    Schema.decodeUnknownSync(GifList)(trending);
    expect(trending.items).toEqual(GIF_FIXTURES);
    expect((await get<GifListWire>(server, "/api/v1/gifs/search?q=hello")).items).toEqual([
      GIF_FIXTURES[0],
    ]);
    expect((await get<GifListWire>(server, "/api/v1/gifs/trending?page=2")).items).toEqual([]);
    Schema.decodeUnknownSync(GifCategoryList)(await get(server, "/api/v1/gifs/categories"));

    for (const path of ["suggestions", "autocomplete"]) {
      expect(
        Schema.decodeUnknownSync(GifSuggestions)(await get(server, `/api/v1/gifs/${path}?q=he`)),
      ).toEqual({ suggestions: ["hello", "hello there", "cheer"] });
    }

    for (const item of trending.items) {
      const preview = await server.handleBinary({
        method: "GET",
        path: item.previewUrl,
        bytes: null,
      });

      expect(preview.status).toBe(200);
      expect(preview.bytes).toEqual(GIF_BYTES);
      expect(preview.contentType).toBe("image/gif");

      if (item.stillUrl !== null) {
        expect(
          (await server.handleBinary({ method: "GET", path: item.stillUrl, bytes: null })).bytes,
        ).toEqual(GIF_STILL_BYTES);
      }
    }
  });

  it("stores the selected fixture as a local message file and replays sends", async () => {
    const { server } = harness();

    const body = messageBody("fixture-gif", "", {
      gifSlug: "hello-wave",
      attachmentSignedId: null,
    });

    Schema.decodeUnknownSync(CreateMessage)(body);

    const message = await expectStatus<MessageDTO>(
      server,
      "POST",
      `/api/v1/rooms/${roomId}/messages`,
      body,
      201,
    );

    Schema.decodeUnknownSync(MessageSchema)(message);
    const file = message.attachments?.[0];

    expect(file?.filename).toBe("Hello-Wave.gif");
    expect(file?.thumbnailUrl).toBe(file?.url);
    expect(file?.byteSize).toBe(GIF_BYTES.length);
    expect(file?.url).toBe("/mock/files/gifs/hello-wave.gif");
    expect(
      (await server.handleBinary({ method: "GET", path: file?.url ?? "", bytes: null })).bytes,
    ).toEqual(GIF_BYTES);

    const replay = await expectStatus<MessageDTO>(
      server,
      "POST",
      `/api/v1/rooms/${roomId}/messages`,
      body,
      200,
    );

    expect(replay.id).toBe(message.id);
    expect(
      (
        await send(
          server,
          "POST",
          `/api/v1/rooms/${roomId}/messages`,
          messageBody("bad-gif", "", {
            gifSlug: "missing",
          }),
        )
      ).status,
    ).toBe(422);
  });

  it("starts a thread with a GIF only with one spoiler flag for it", async () => {
    const { server } = harness();

    const page = await get<{ messages: MessageDTO[] }>(
      server,
      `/api/v1/rooms/${SEED_IDS.rooms.general}/messages`,
    );

    const parents = page.messages.filter((message) => message.thread === null).slice(0, 2);

    expect(parents).toHaveLength(2);

    const body = (parentMessageId: number, clientMessageId: string, spoilers?: boolean[]) => {
      const message =
        spoilers === undefined
          ? messageBody(clientMessageId, "", {
              gifSlug: "hello-wave",
              attachmentSignedId: null,
            })
          : messageBody(clientMessageId, "", {
              gifSlug: "hello-wave",
              attachmentSignedId: null,
              attachmentSpoilers: spoilers,
            });

      return { parentMessageId, name: null, message };
    };

    const path = `/api/v1/rooms/${SEED_IDS.rooms.general}/threads`;

    // The GIF counts as one file (`file_spoilers` in crates/api): no flag for it is refused.
    expect((await send(server, "POST", path, body(parents[0]?.id ?? 0, "bad", []))).status).toBe(
      422,
    );

    const created = await expectStatus<ThreadCreated>(
      server,
      "POST",
      path,
      body(parents[0]?.id ?? 0, "thread-gif", [false]),
      201,
    );

    const replies = await get<{ messages: MessageDTO[] }>(
      server,
      `/api/v1/threads/${created.detail.thread.id}/messages`,
    );

    expect(replies.messages[0]?.attachments?.[0]?.filename).toBe("Hello-Wave.gif");

    // Leaving the flags out is the other way the server allows: it fills them with `false`.
    expect(
      (await send(server, "POST", path, body(parents[1]?.id ?? 0, "thread-gif-omitted"))).status,
    ).toBe(201);
  });

  it("keeps a GIF-only board brief and its retry", async () => {
    const { server } = harness();
    const path = `/api/v1/rooms/${SEED_IDS.boards.roomId}/posts`;

    const body = {
      name: "GIF brief",
      status: "planned",
      tags: [],
      message: messageBody("board-gif", "", { gifSlug: "hello-wave" }),
    };

    const created = await expectStatus<ThreadDetail>(server, "POST", path, body, 201);

    const page = await get<{ messages: MessageDTO[] }>(
      server,
      `/api/v1/threads/${created.thread.id}/messages`,
    );

    expect(page.messages[0]?.attachments?.[0]?.filename).toBe("Hello-Wave.gif");
    const replay = await expectStatus<ThreadDetail>(server, "POST", path, body, 200);

    expect(replay.thread.id).toBe(created.thread.id);
  });

  it("offers a feature-off world and validates search inputs", async () => {
    const server = createMockServer({ gifsEnabled: false });

    expect(await get(server, "/api/v1/gifs/config")).toEqual({ enabled: false });

    for (const path of [
      "trending",
      "search?q=hello",
      "suggestions?q=hello",
      "autocomplete?q=he",
      "categories",
    ]) {
      expect((await server.handle({ method: "GET", path: `/api/v1/gifs/${path}` })).status).toBe(
        404,
      );
    }

    expect(
      (
        await send(
          server,
          "POST",
          `/api/v1/rooms/${roomId}/messages`,
          messageBody("off", "", {
            gifSlug: "hello-wave",
          }),
        )
      ).status,
    ).toBe(404);
    server.dispose();
    const { server: active } = harness();

    for (const path of ["search?q=", "suggestions?q=", "trending?page=0", "trending?page=10001"]) {
      expect((await active.handle({ method: "GET", path: `/api/v1/gifs/${path}` })).status).toBe(
        422,
      );
    }
  });
});
