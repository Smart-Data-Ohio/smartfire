import { describe, expect, it } from "vitest";
import type { CreateUpload } from "../../gen/CreateUpload.ts";
import type { DirectUpload } from "../../gen/DirectUpload.ts";
import {
  contentTypeOf,
  putFailureMessage,
  type UploadDeps,
  type UploadPhase,
  type UploadSnapshot,
  UploadTask,
  type XhrLike,
} from "./direct-upload.ts";

/** A scriptable XMLHttpRequest: the test decides when progress, load or failure happen. */
class FakeXhr implements XhrLike {
  status = 0;

  readonly upload: XhrLike["upload"] = { onprogress: null };

  onload: ((event: ProgressEvent) => void) | null = null;

  onerror: ((event: ProgressEvent) => void) | null = null;

  onabort: ((event: ProgressEvent) => void) | null = null;

  method = "";

  url = "";

  readonly headers = new Map<string, string>();

  body: Blob | null = null;

  aborted = false;

  open(method: string, url: string): void {
    this.method = method;
    this.url = url;
  }

  setRequestHeader(name: string, value: string): void {
    this.headers.set(name, value);
  }

  send(body: Blob): void {
    this.body = body;
  }

  abort(): void {
    this.aborted = true;
    this.onabort?.(new ProgressEvent("abort"));
  }

  progress(loaded: number, total: number): void {
    this.upload.onprogress?.(
      new ProgressEvent("progress", { loaded, total, lengthComputable: true }),
    );
  }

  finish(status: number): void {
    this.status = status;
    this.onload?.(new ProgressEvent("load"));
  }

  fail(): void {
    this.onerror?.(new ProgressEvent("error"));
  }
}

interface Harness {
  readonly deps: UploadDeps;
  readonly xhrs: FakeXhr[];
  readonly starts: CreateUpload[];
  readonly phases: UploadPhase[];
  readonly onChange: (snapshot: UploadSnapshot) => void;
}

function harness(startReply: (body: CreateUpload) => Promise<DirectUpload>): Harness {
  const xhrs: FakeXhr[] = [];
  const starts: CreateUpload[] = [];
  const phases: UploadPhase[] = [];

  return {
    xhrs,
    starts,
    phases,
    onChange: (snapshot) => {
      if (phases.at(-1) !== snapshot.phase) {
        phases.push(snapshot.phase);
      }
    },
    deps: {
      hash: () => Promise.resolve("CHECKSUM=="),
      start: (body) => {
        starts.push(body);

        return startReply(body);
      },
      createXhr: () => {
        const xhr = new FakeXhr();

        xhrs.push(xhr);

        return xhr;
      },
    },
  };
}

const reply = (n: number): Promise<DirectUpload> =>
  Promise.resolve({ signedId: `signed-${n}`, uploadUrl: `/rails/active_storage/disk/token-${n}` });

/** Lets queued promise callbacks run. */
const flush = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

function file(name = "photo.png", type = "image/png"): File {
  return new File([new Uint8Array(1000)], name, { type });
}

describe("UploadTask", () => {
  it("hashes, starts the blob, PUTs the bytes with progress and ends done", async () => {
    const h = harness(() => reply(1));
    const task = new UploadTask(file(), h.deps, h.onChange);
    const done = task.start();

    await flush();

    const xhr = h.xhrs[0];

    expect(h.starts).toEqual([
      { filename: "photo.png", byteSize: 1000, checksum: "CHECKSUM==", contentType: "image/png" },
    ]);
    expect(xhr?.method).toBe("PUT");
    expect(xhr?.url).toBe("/rails/active_storage/disk/token-1");
    expect(xhr?.headers.get("Content-Type")).toBe("image/png");
    expect(task.snapshot.phase).toBe("uploading");

    xhr?.progress(400, 1000);
    expect(task.snapshot.loaded).toBe(400);

    xhr?.finish(204);
    await done;

    expect(task.snapshot).toMatchObject({ phase: "done", loaded: 1000, signedId: "signed-1" });
    expect(h.phases).toEqual(["hashing", "starting", "uploading", "done"]);
  });

  it("fails on a rejected PUT and retries from a fresh blob, reusing the checksum", async () => {
    let n = 0;
    let hashes = 0;
    const h = harness(() => reply(++n));

    const deps: UploadDeps = {
      ...h.deps,
      hash: () => {
        hashes += 1;

        return Promise.resolve("CHECKSUM==");
      },
    };

    const task = new UploadTask(file(), deps, h.onChange);
    const first = task.start();

    await flush();
    h.xhrs[0]?.finish(422);
    await first;

    expect(task.snapshot.phase).toBe("failed");
    expect(task.snapshot.error).toBe(putFailureMessage(422));

    const second = task.retry();

    await flush();
    h.xhrs[1]?.finish(204);
    await second;

    expect(task.snapshot).toMatchObject({ phase: "done", signedId: "signed-2", error: null });
    expect(h.starts).toHaveLength(2);
    expect(hashes).toBe(1);
  });

  it("fails when the blob can't be started", async () => {
    const h = harness(() => Promise.reject(new Error("Forbidden")));
    const task = new UploadTask(file(), h.deps, h.onChange);

    await task.start();

    expect(task.snapshot).toMatchObject({ phase: "failed", error: "Forbidden" });
    expect(h.xhrs).toHaveLength(0);
  });

  it("reports a network error", async () => {
    const h = harness(() => reply(1));
    const task = new UploadTask(file(), h.deps, h.onChange);
    const done = task.start();

    await flush();
    h.xhrs[0]?.fail();
    await done;

    expect(task.snapshot).toMatchObject({ phase: "failed", error: putFailureMessage(0) });
  });

  it("cancels mid-upload: the XHR aborts and the task stays cancelled", async () => {
    const h = harness(() => reply(1));
    const task = new UploadTask(file(), h.deps, h.onChange);
    const done = task.start();

    await flush();
    task.cancel();
    await done;

    expect(h.xhrs[0]?.aborted).toBe(true);
    expect(task.snapshot.phase).toBe("cancelled");
    expect(h.phases.at(-1)).toBe("cancelled");
    await task.retry();
    expect(task.snapshot.phase).toBe("cancelled");
  });

  it("cancels while hashing without ever starting a blob", async () => {
    let release: (value: string) => void = () => undefined;
    const h = harness(() => reply(1));

    const deps: UploadDeps = {
      ...h.deps,
      hash: () =>
        new Promise((resolve) => {
          release = resolve;
        }),
    };

    const task = new UploadTask(file(), deps, h.onChange);
    const done = task.start();

    task.cancel();
    release("CHECKSUM==");
    await done;

    expect(task.snapshot.phase).toBe("cancelled");
    expect(h.starts).toHaveLength(0);
  });
});

describe("contentTypeOf", () => {
  it("falls back to octet-stream for files the browser can't type", () => {
    expect(contentTypeOf(file("notes", ""))).toBe("application/octet-stream");
    expect(contentTypeOf(file("a.pdf", "application/pdf"))).toBe("application/pdf");
  });
});
