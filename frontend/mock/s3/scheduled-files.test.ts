import { expect, it } from "vitest";
import type { DirectUpload } from "../../src/gen/DirectUpload.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { Poll } from "../../src/gen/Poll.ts";
import type { ScheduledMessage } from "../../src/gen/ScheduledMessage.ts";
import type { ScheduledMessageList } from "../../src/gen/ScheduledMessageList.ts";
import { onboardingMockupPng, utf8 } from "../s2/assets.ts";
import { md5Base64 } from "../s2/digest.ts";
import { expectStatus, get, harness, NOW, send } from "../s2/testing.ts";
import { tinyWav } from "../s5/soundboard.ts";
import { type MockServer, SEED_IDS } from "../server.ts";

const ROOM = SEED_IDS.rooms.quiet;

async function upload(
  server: MockServer,
  filename: string,
  bytes = utf8("hello"),
  contentType = "text/plain",
) {
  const file = await expectStatus<DirectUpload>(
    server,
    "POST",
    "/api/v1/uploads",
    {
      filename,
      byteSize: bytes.length,
      checksum: md5Base64(bytes),
      contentType,
    },
    200,
  );

  const response = await server.handleBinary({
    method: "PUT",
    path: file.uploadUrl,
    bytes,
    headers: { "Content-Type": contentType },
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

it("rejects rescheduling a sent signed id without purging its published download", async () => {
  const { server } = harness();
  const signedId = await upload(server, "published.txt");
  const first = await schedule(server, [signedId]);
  const path = `/api/v1/scheduled_messages/${first.id}`;
  const sent = await expectStatus<ScheduledMessage>(server, "POST", `${path}/send_now`, {}, 200);
  const downloadUrl = sent.attachments[0]?.attachment.downloadUrl ?? "";
  const download = () => server.handleBinary({ method: "GET", path: downloadUrl, bytes: null });

  expect((await download()).status).toBe(200);

  const reuse = await send(server, "POST", `/api/v1/rooms/${ROOM}/scheduled_messages`, {
    markdownSource: "",
    sendAt: new Date(NOW + 3600000).toISOString(),
    attachmentSignedIds: [signedId],
  });

  if (reuse.status === 201) {
    const pending = await get<ScheduledMessageList>(server, "/api/v1/scheduled_messages");

    const reused = pending.scheduledMessages.find((message) =>
      message.attachments.some((file) => file.signedId === signedId),
    );

    if (reused === undefined) throw new Error("reused scheduled message is missing");
    await expectStatus(server, "DELETE", `/api/v1/scheduled_messages/${reused.id}`, {}, 204);
  }

  expect((await download()).status).toBe(200);
  expect(reuse.status).toBe(422);
  await expectStatus(server, "DELETE", path, {}, 404);
  expect((await download()).status).toBe(200);
});

it("rejects a sent signed id while editing another scheduled draft", async () => {
  const { server } = harness();
  const signedId = await upload(server, "published.txt");
  const first = await schedule(server, [signedId]);
  await expectStatus(server, "POST", `/api/v1/scheduled_messages/${first.id}/send_now`, {}, 200);
  const second = await schedule(server, [await upload(server, "pending.txt")]);

  await expectStatus(
    server,
    "PATCH",
    `/api/v1/scheduled_messages/${second.id}`,
    {
      attachmentSignedIds: [signedId],
    },
    422,
  );
  await expectStatus(server, "DELETE", `/api/v1/scheduled_messages/${second.id}`, {}, 204);

  const download = await server.handleBinary({
    method: "GET",
    path: first.attachments[0]?.attachment.downloadUrl ?? "",
    bytes: null,
  });

  expect(download.status).toBe(200);
});

it.each([
  ["avatar", "PUT", "/api/v1/settings/avatar"],
  ["profile banner", "PUT", "/api/v1/settings/banner"],
  ["workspace logo", "PUT", "/api/v1/admin/workspace/logo"],
  ["workspace banner", "PUT", "/api/v1/admin/workspace/banner"],
  ["workspace icon", "POST", "/api/v1/admin/icons"],
  ["bot avatar", "POST", "/api/v1/admin/bots"],
  ["poll image", "POST", `/api/v1/rooms/${ROOM}/polls`],
])("keeps a scheduled upload private when assigning it as %s", async (_, method, path) => {
  const { server } = harness();
  const signedId = await upload(server, "private.png", onboardingMockupPng(), "image/png");
  const row = await schedule(server, [signedId]);

  const response = await send(server, method, path, {
    signedId,
    avatar: signedId,
    name: "private_upload",
    title: "Private image",
    clientMessageId: "private-poll-image",
    question: "Publish image?",
    options: ["Yes", "No"],
    optionMedia: [{ signedId }, null],
  });

  expect(response.status).toBe(422);
  const pending = await get<ScheduledMessageList>(server, "/api/v1/scheduled_messages");
  expect(pending.scheduledMessages.find((message) => message.id === row.id)?.attachments).toEqual(
    row.attachments,
  );
});

it("serves unassigned and scheduled originals and previews without caching until sent", async () => {
  const { server } = harness();
  const signedId = await upload(server, "private.png", onboardingMockupPng(), "image/png");
  const original = `/rails/active_storage/blobs/redirect/${signedId}/private.png`;
  const fresh = await server.handleBinary({ method: "GET", path: original, bytes: null });
  expect(fresh.headers["Cache-Control"]).toBe("private, no-store");
  const row = await schedule(server, [signedId]);
  const attachment = row.attachments[0]?.attachment;

  for (const url of [original, attachment?.thumbnailUrl ?? ""]) {
    for (const path of [url, url.replace("/redirect/", "/proxy/")]) {
      const response = await server.handleBinary({ method: "GET", path, bytes: null });
      expect(response.status).toBe(200);
      expect(response.headers["Cache-Control"]).toBe("private, no-store");
    }
  }

  await expectStatus(server, "POST", `/api/v1/scheduled_messages/${row.id}/send_now`, {}, 200);
  const published = await server.handleBinary({ method: "GET", path: original, bytes: null });
  expect(published.headers["Cache-Control"]).toBe("max-age=300, private");
});

it("rejects publishing a scheduled upload as a soundboard sound", async () => {
  const { server } = harness();
  const signedId = await upload(server, "private.wav", tinyWav(330), "audio/wav");
  await schedule(server, [signedId]);
  await expectStatus(
    server,
    "POST",
    "/api/v1/admin/soundboard/sounds",
    {
      signedId,
      name: "Private sound",
      emoji: null,
      volume: 1,
    },
    422,
  );
});

it("rejects scheduling a signed id already posted to a room", async () => {
  const { server } = harness();
  const signedId = await upload(server, "published.txt");
  await expectStatus(
    server,
    "POST",
    `/api/v1/rooms/${ROOM}/messages`,
    {
      clientMessageId: "published-upload",
      markdownSource: "",
      attachmentSignedIds: [signedId],
    },
    201,
  );
  await expectStatus(
    server,
    "POST",
    `/api/v1/rooms/${ROOM}/scheduled_messages`,
    {
      markdownSource: "",
      sendAt: new Date(NOW + 3600000).toISOString(),
      attachmentSignedIds: [signedId],
    },
    422,
  );
});

it("refuses scheduling a signed id already published as poll media", async () => {
  const { server } = harness();
  const signedId = await upload(server, "poll.png", onboardingMockupPng(), "image/png");

  const created = await expectStatus<{ poll?: Poll }>(
    server,
    "POST",
    `/api/v1/rooms/${ROOM}/polls`,
    {
      clientMessageId: "published-poll-image",
      question: "Publish image?",
      options: ["Yes", "No"],
      optionMedia: [{ signedId }, null],
    },
    201,
  );

  const media = created.poll?.options[0]?.media;
  const imagePath = media?.kind === "image" ? media.url : "";
  expect(imagePath).not.toBe("");
  const image = () => server.handleBinary({ method: "GET", path: imagePath, bytes: null });

  expect((await image()).status).toBe(200);

  const reuse = await send(server, "POST", `/api/v1/rooms/${ROOM}/scheduled_messages`, {
    markdownSource: "",
    sendAt: new Date(NOW + 3600000).toISOString(),
    attachmentSignedIds: [signedId],
  });

  if (reuse.status === 201) {
    const pending = await get<ScheduledMessageList>(server, "/api/v1/scheduled_messages");

    const reused = pending.scheduledMessages.find((message) =>
      message.attachments.some((file) => file.signedId === signedId),
    );

    if (reused === undefined) throw new Error("reused scheduled message is missing");

    await expectStatus(server, "DELETE", `/api/v1/scheduled_messages/${reused.id}`, {}, 204);
  }

  expect(reuse.status).toBe(422);
  expect((await image()).status).toBe(200);
});

it("allows an unassigned upload as a poll image", async () => {
  const { server } = harness();
  const signedId = await upload(server, "poll.png", onboardingMockupPng(), "image/png");
  await expectStatus(
    server,
    "POST",
    `/api/v1/rooms/${ROOM}/polls`,
    {
      clientMessageId: "fresh-poll-image",
      question: "Publish image?",
      options: ["Yes", "No"],
      optionMedia: [{ signedId }, null],
    },
    201,
  );
});
