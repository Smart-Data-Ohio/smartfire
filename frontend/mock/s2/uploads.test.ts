import { describe, expect, it } from "vitest";
import type { DirectUpload } from "../../src/gen/DirectUpload.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { ThreadCreated } from "../../src/gen/ThreadCreated.ts";
import type { MockServer } from "../server.ts";
import { SEED_IDS } from "../server.ts";
import { onboardingMockupPng, utf8 } from "./assets.ts";
import { md5Base64 } from "./digest.ts";
import { errorOf, expectStatus, harness, messageBody, send } from "./testing.ts";

const { rooms, threads, messages } = SEED_IDS;

const GIF = new Uint8Array([
  0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x20, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xff, 0xff, 0xff,
  0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x02, 0x02, 0x44,
  0x01, 0x00, 0x3b,
]);

function declare(server: MockServer, filename: string, contentType: string, bytes: Uint8Array) {
  return expectStatus<DirectUpload>(
    server,
    "POST",
    "/api/v1/uploads",
    { filename, byteSize: bytes.length, checksum: md5Base64(bytes), contentType },
    200,
  );
}

function put(server: MockServer, upload: DirectUpload, contentType: string, bytes: Uint8Array) {
  return server.handleBinary({
    method: "PUT",
    path: upload.uploadUrl,
    headers: { "Content-Type": contentType },
    bytes,
  });
}

describe("direct uploads", () => {
  it("declares, uploads and attaches an image, reading its size", async () => {
    const { server } = harness();
    const upload = await declare(server, "pixel art.gif", "image/gif", GIF);

    expect(upload.uploadUrl).toMatch(/^\/rails\/active_storage\/disk\/[0-9a-f]+$/);
    expect(upload.signedId).toMatch(/--[0-9a-f]+$/);
    expect((await put(server, upload, "image/gif", GIF)).status).toBe(204);

    const message = await expectStatus<MessageDTO>(
      server,
      "POST",
      `/api/v1/rooms/${rooms.quiet}/messages`,
      messageBody("upload-1", "", { attachmentSignedId: upload.signedId }),
      201,
    );

    expect(message.attachment).toEqual({
      filename: "pixel art.gif",
      contentType: "image/gif",
      byteSize: GIF.length,
      width: 32,
      height: 16,
      preview: "image",
      url: `/rails/active_storage/blobs/redirect/${upload.signedId}/pixel%20art.gif`,
      downloadUrl: `/rails/active_storage/blobs/redirect/${upload.signedId}/pixel%20art.gif?disposition=attachment`,
      thumbnailUrl: expect.stringMatching(
        /^\/rails\/active_storage\/representations\/redirect\/.+\/pixel%20art\.gif$/,
      ),
    });

    const inline = await server.handleBinary({
      method: "GET",
      path: message.attachment?.url ?? "",
      bytes: null,
    });

    const download = await server.handleBinary({
      method: "GET",
      path: message.attachment?.downloadUrl ?? "",
      bytes: null,
    });

    const thumbnail = await server.handleBinary({
      method: "GET",
      path: message.attachment?.thumbnailUrl ?? "",
      bytes: null,
    });

    expect(inline).toMatchObject({ status: 200, contentType: "image/gif", bytes: GIF });
    expect(inline.headers["Content-Disposition"]).toMatch(/^inline;/);
    expect(download.headers["Content-Disposition"]).toMatch(
      /^attachment; filename="pixel art.gif"/,
    );
    expect(thumbnail.status).toBe(200);
  });

  it("refuses bytes that don't match what was declared", async () => {
    const { server } = harness();
    const upload = await declare(server, "a.gif", "image/gif", GIF);
    const wrongBytes = await put(server, upload, "image/gif", GIF.slice(1));
    const wrongType = await put(server, upload, "image/png", GIF);

    const tampered = GIF.slice();

    tampered[20] = 0x01;

    const wrongSum = await put(server, upload, "image/gif", tampered);

    const unknown = await server.handleBinary({
      method: "PUT",
      path: "/rails/active_storage/disk/nope",
      headers: { "Content-Type": "image/gif" },
      bytes: GIF,
    });

    expect([wrongBytes.status, wrongType.status, wrongSum.status, unknown.status]).toEqual([
      422, 422, 422, 404,
    ]);
  });

  it("expires the upload URL after five minutes", async () => {
    const { server, clock } = harness();
    const upload = await declare(server, "a.gif", "image/gif", GIF);

    clock.advance(5 * 60_000 + 1);
    expect((await put(server, upload, "image/gif", GIF)).status).toBe(404);
  });

  it("requires a finished upload and text unless there's a file", async () => {
    const { server } = harness();
    const upload = await declare(server, "a.gif", "image/gif", GIF);
    const path = `/api/v1/rooms/${rooms.quiet}/messages`;

    const unfinished = await send(
      server,
      "POST",
      path,
      messageBody("u-1", "", { attachmentSignedId: upload.signedId }),
    );

    const bogus = await send(
      server,
      "POST",
      path,
      messageBody("u-2", "hi", { attachmentSignedId: "nope" }),
    );

    const empty = await send(server, "POST", path, messageBody("u-3", ""));

    expect([unfinished.status, bogus.status, empty.status]).toEqual([422, 422, 422]);
    expect(errorOf(empty.json).tag).toBe("Validation");
    expect(
      (await send(server, "POST", "/api/v1/uploads", { filename: "", byteSize: 1, checksum: "x" }))
        .status,
    ).toBe(422);
  });

  it("attaches files to thread replies and new threads", async () => {
    const { server } = harness();
    const bytes = onboardingMockupPng();
    const first = await declare(server, "mockup.png", "image/png", bytes);
    const second = await declare(server, "notes.txt", "text/plain", utf8("notes"));

    await put(server, first, "image/png", bytes);
    await put(server, second, "text/plain", utf8("notes"));

    const reply = await expectStatus<MessageDTO>(
      server,
      "POST",
      `/api/v1/threads/${threads.generalActive}/messages`,
      messageBody("t-1", "", { attachmentSignedId: first.signedId }),
      201,
    );

    expect(reply.attachment).toMatchObject({ width: 1200, height: 750, preview: "image" });

    const created = await expectStatus<ThreadCreated>(
      server,
      "POST",
      `/api/v1/rooms/${rooms.general}/threads`,
      {
        parentMessageId: messages.generalSaved,
        name: "Notes",
        message: messageBody("t-2", "", { attachmentSignedId: second.signedId }),
      },
      201,
    );

    expect(created.message.attachment).toMatchObject({ filename: "notes.txt", preview: "file" });
  });

  it("holds and throttles upload PUTs", async () => {
    const { server, clock } = harness();
    const upload = await declare(server, "a.gif", "image/gif", GIF);
    let status = 0;

    server.holdUploads(true);

    const pending = put(server, upload, "image/gif", GIF).then((response) => {
      status = response.status;
    });

    await Promise.resolve();
    expect(server.pendingUploads()).toBe(1);
    expect(status).toBe(0);

    server.throttleUploads(2000);
    server.releaseUploads();
    await Promise.resolve();
    expect(status).toBe(0);

    clock.advance(2000);
    await pending;
    expect(status).toBe(204);
    expect(server.pendingUploads()).toBe(0);
  });

  it("serves seeded files and 404s unknown ones", async () => {
    const { server } = harness();

    const unknown = await server.handleBinary({
      method: "GET",
      path: "/rails/active_storage/blobs/redirect/nope/a.png",
      bytes: null,
    });

    const wrongMethod = await server.handleBinary({
      method: "DELETE",
      path: "/rails/active_storage/blobs/redirect/nope/a.png",
      bytes: null,
    });

    expect(unknown.status).toBe(404);
    expect(wrongMethod.status).toBe(405);
  });
});
