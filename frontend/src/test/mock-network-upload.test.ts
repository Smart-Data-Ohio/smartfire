import { afterEach, describe, expect, it } from "vitest";
import { md5Base64 } from "../../mock/s2/digest.ts";
import { SEED_IDS } from "../../mock/server.ts";
import type { DirectUpload } from "../gen/DirectUpload.ts";
import type { IconList } from "../gen/IconList.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import { installMockNetwork, type MockNetwork } from "./mock-network.ts";

let network: MockNetwork | null = null;

afterEach(() => {
  network?.restore();
  network = null;
});

const BYTES = new TextEncoder().encode("quarterly notes\n");

/** What the XHR stub reported while a PUT ran. */
interface XhrRun {
  readonly status: number;
  readonly events: readonly string[];
}

function jsonHeaders(active: MockNetwork) {
  return { "Content-Type": "application/json", "X-CSRF-Token": active.server.csrfToken() };
}

async function declare(active: MockNetwork, filename: string): Promise<DirectUpload> {
  const response = await fetch("/api/v1/uploads", {
    method: "POST",
    headers: jsonHeaders(active),
    body: JSON.stringify({
      filename,
      byteSize: BYTES.length,
      checksum: md5Base64(BYTES),
      contentType: "text/plain",
    }),
  });

  expect(response.status).toBe(200);

  return response.json();
}

function putWithXhr(upload: DirectUpload, body: Blob): Promise<XhrRun> {
  const events: string[] = [];
  const request = new XMLHttpRequest();

  request.upload.addEventListener("progress", () => events.push("progress"));
  request.addEventListener("load", () => events.push("load"));

  return new Promise((resolve) => {
    request.addEventListener("loadend", () => resolve({ status: request.status, events }));
    request.open("PUT", upload.uploadUrl);
    request.setRequestHeader("Content-Type", "text/plain");
    request.send(body);
  });
}

describe("uploads over the mock network", () => {
  it("declares with fetch, PUTs with XHR, posts and downloads the bytes", async () => {
    const active = installMockNetwork();

    network = active;

    const upload = await declare(active, "notes.txt");
    const run = await putWithXhr(upload, new Blob([BYTES], { type: "text/plain" }));

    expect(run.status).toBe(204);
    expect(run.events.at(-1)).toBe("load");
    expect(run.events).toContain("progress");

    const posted = await fetch(`/api/v1/rooms/${SEED_IDS.rooms.quiet}/messages`, {
      method: "POST",
      headers: jsonHeaders(active),
      body: JSON.stringify({
        clientMessageId: "net-upload-1",
        markdownSource: "",
        replyToMessageId: null,
        replyNotifyAuthor: null,
        attachmentSignedId: upload.signedId,
      }),
    });

    const message: MessageDTO = await posted.json();

    expect(posted.status).toBe(201);
    expect(message.attachment?.filename).toBe("notes.txt");

    const download = await fetch(message.attachment?.downloadUrl ?? "");

    expect(download.status).toBe(200);
    expect(download.headers.get("Content-Disposition")).toMatch(/^attachment;/);
    expect(Array.from(new Uint8Array(await download.arrayBuffer()))).toEqual(Array.from(BYTES));
  });

  it("answers 422 to a PUT whose bytes don't match", async () => {
    const active = installMockNetwork();

    network = active;

    const upload = await declare(active, "notes.txt");

    const response = await fetch(upload.uploadUrl, {
      method: "PUT",
      headers: { "Content-Type": "text/plain" },
      body: new TextEncoder().encode("something else!\n"),
    });

    expect(response.status).toBe(422);
  });

  it("serves seeded icons and blobs", async () => {
    network = installMockNetwork();

    const icons = await fetch("/api/v1/icons");
    const list: IconList = await icons.json();
    const imageUrl = list.icons.find((icon) => icon.imageUrl !== null)?.imageUrl;

    expect(imageUrl).toBeDefined();

    const image = await fetch(imageUrl ?? "");

    expect(image.status).toBe(200);
    expect(image.headers.get("Content-Type")).toMatch(/^image\//);
  });
});
