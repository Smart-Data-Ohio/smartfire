import { expect, it } from "vitest";
import type { DirectUpload } from "../../src/gen/DirectUpload.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { ScheduledMessage } from "../../src/gen/ScheduledMessage.ts";
import type { ScheduledMessageList } from "../../src/gen/ScheduledMessageList.ts";
import { utf8 } from "../s2/assets.ts";
import { md5Base64 } from "../s2/digest.ts";
import { expectStatus, get, harness, NOW } from "../s2/testing.ts";
import { type MockServer, SEED_IDS } from "../server.ts";

const ROOM = SEED_IDS.rooms.quiet;

async function upload(server: MockServer, filename: string) {
  const bytes = utf8("hello");

  const file = await expectStatus<DirectUpload>(
    server,
    "POST",
    "/api/v1/uploads",
    {
      filename,
      byteSize: bytes.length,
      checksum: md5Base64(bytes),
      contentType: "text/plain",
    },
    200,
  );

  const response = await server.handleBinary({
    method: "PUT",
    path: file.uploadUrl,
    bytes,
    headers: { "Content-Type": "text/plain" },
  });

  expect(response.status).toBe(204);

  return file.signedId;
}

function schedule(server: MockServer, ids: string[]) {
  return expectStatus<ScheduledMessage>(
    server,
    "POST",
    `/api/v1/rooms/${ROOM}/scheduled_messages`,
    {
      markdownSource: "",
      sendAt: new Date(NOW + 3600000).toISOString(),
      attachmentSignedIds: ids,
    },
    201,
  );
}

it("edits the grouped files, purges removals, and dispatches a single reply", async () => {
  const { server } = harness();
  const first = await upload(server, "first.txt");
  const second = await upload(server, "second.txt");
  const third = await upload(server, "third.txt");
  const target = server.post(ROOM, SEED_IDS.users.maya, "Please send the files");
  const row = await schedule(server, [first, second]);
  const path = `/api/v1/scheduled_messages/${row.id}`;
  await expectStatus(
    server,
    "PATCH",
    path,
    {
      attachmentSignedIds: [second, third],
      replyToMessageId: target.id,
    },
    200,
  );

  const removed = await server.handleBinary({
    method: "GET",
    path: row.attachments[0]?.attachment.url ?? "",
    bytes: null,
  });

  expect(removed.status).toBe(404);
  const sent = await expectStatus<ScheduledMessage>(server, "POST", `${path}/send_now`, {}, 200);
  const page = await get<MessagePage>(server, `/api/v1/rooms/${ROOM}/messages`);
  const messages = page.messages.filter((message) => message.id === sent.sentMessageId);
  expect(messages).toHaveLength(1);
  expect(messages[0]?.replyToMessageId).toBe(target.id);
  expect(messages[0]?.attachments?.map((file) => file.filename)).toEqual([
    "second.txt",
    "third.txt",
  ]);
});

it("revalidates the current file limit before dispatch and drops the whole group", async () => {
  const { server, clock } = harness();

  const row = await schedule(server, [
    await upload(server, "first.txt"),
    await upload(server, "second.txt"),
  ]);

  await expectStatus(server, "PATCH", "/api/v1/admin/workspace", { uploadLimitBytes: 4 }, 200);
  clock.advance(3600001);
  const past = await get<ScheduledMessageList>(server, "/api/v1/scheduled_messages?status=past");
  expect(past.scheduledMessages.find((message) => message.id === row.id)).toMatchObject({
    state: "dropped",
    sentMessageId: null,
    dropReason: expect.stringContaining("upload limit"),
  });
  const page = await get<MessagePage>(server, `/api/v1/rooms/${ROOM}/messages`);
  expect(
    page.messages.some((message) =>
      message.attachments?.some((file) => file.filename === "first.txt"),
    ),
  ).toBe(false);
});

it("rejects reusing a scheduled upload and purges files on cancellation", async () => {
  const { server } = harness();
  const signedId = await upload(server, "private.txt");
  const row = await schedule(server, [signedId]);

  await expectStatus(
    server,
    "POST",
    `/api/v1/rooms/${ROOM}/messages`,
    {
      clientMessageId: "reuse-private-upload",
      markdownSource: "Another",
      attachmentSignedIds: [signedId],
    },
    422,
  );
  await expectStatus(
    server,
    "POST",
    `/api/v1/rooms/${ROOM}/scheduled_messages`,
    {
      markdownSource: "Another",
      sendAt: new Date(NOW + 3600000).toISOString(),
      attachmentSignedIds: [signedId],
    },
    422,
  );
  await expectStatus(server, "DELETE", `/api/v1/scheduled_messages/${row.id}`, {}, 204);

  const file = await server.handleBinary({
    method: "GET",
    path: row.attachments[0]?.attachment.url ?? "",
    bytes: null,
  });

  expect(file.status).toBe(404);
});
