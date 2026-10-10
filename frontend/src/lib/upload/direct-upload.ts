/**
 * One file's direct upload, as a small state machine the composer's chip renders:
 * hashing -> starting -> uploading (with progress) -> done, or failed / cancelled. A failed upload
 * can be retried (it starts a fresh blob: the old upload URL may have lapsed); a cancelled one is
 * finished for good.
 *
 * The flow is Active Storage's (crates/api_types attachment.rs): MD5 the bytes, `POST /uploads`
 * for a signed id and an upload URL, then `PUT` the raw bytes there with the same
 * `Content-Type` (204; 422 on a length, type or checksum mismatch; 404 once the URL's 5 minutes
 * are up). The signed id then rides on the message.
 */
import type { CreateUpload } from "../../gen/CreateUpload.ts";
import type { DirectUpload } from "../../gen/DirectUpload.ts";
import { md5Base64 } from "./md5.ts";

export type UploadPhase = "hashing" | "starting" | "uploading" | "done" | "failed" | "cancelled";

export interface UploadSnapshot {
  readonly phase: UploadPhase;
  /** Bytes sent so far (uploading), or the whole file once done. */
  readonly loaded: number;
  readonly total: number;
  /** Set once done: `CreateMessage.attachmentSignedId`. */
  readonly signedId: string | null;
  /** Why it failed, fit to show. */
  readonly error: string | null;
}

/** The parts of XMLHttpRequest the PUT uses, so tests can drive a fake. */
export interface XhrLike {
  readonly status: number;
  readonly upload: { onprogress: ((event: ProgressEvent) => void) | null };
  onload: ((event: ProgressEvent) => void) | null;
  onerror: ((event: ProgressEvent) => void) | null;
  onabort: ((event: ProgressEvent) => void) | null;
  open(method: string, url: string): void;
  setRequestHeader(name: string, value: string): void;
  send(body: Blob): void;
  abort(): void;
}

/** What an upload needs from the outside world; the composer passes the real ones. */
export interface UploadDeps {
  readonly limitBytes: number;
  readonly hash: (file: Blob, signal: AbortSignal) => Promise<string>;
  readonly start: (body: CreateUpload) => Promise<DirectUpload>;
  readonly createXhr: () => XhrLike;
}

/** The browser's: WebCrypto-free MD5, `actions.messages.startUpload`, a real XHR. */
export function browserDeps(
  start: (body: CreateUpload) => Promise<DirectUpload>,
  limitBytes: number,
): UploadDeps {
  return {
    limitBytes,
    hash: (file, signal) => md5Base64(file, signal),
    start,
    createXhr: () => new XMLHttpRequest(),
  };
}

export const DEFAULT_UPLOAD_LIMIT_BYTES = 100 * 1024 * 1024;

export function uploadSizeError(file: File, limitBytes: number): string | null {
  if (file.size <= limitBytes) return null;
  const mb = 1024 * 1024;

  const limit =
    limitBytes % mb === 0 ? `${limitBytes / mb} MB` : `${limitBytes.toLocaleString()} bytes`;

  return `"${file.name}" exceeds the ${limit} upload limit.`;
}

/** The MIME type the blob is declared with and the PUT sends. */
export function contentTypeOf(file: File): string {
  return file.type === "" ? "application/octet-stream" : file.type;
}

/** The PUT's failure message for an HTTP status. */
export function putFailureMessage(status: number): string {
  if (status === 404) {
    return "The upload link expired. Try again.";
  }

  if (status === 422) {
    return "The server didn't accept the file.";
  }

  if (status === 413) {
    return "The file is too large.";
  }

  return status === 0 ? "Couldn't reach the server." : `Upload failed (${status}).`;
}

class PutError extends Error {}

/** `PUT`s `file` to `url` with progress; resolves on a 2xx. Aborting rejects with an AbortError. */
export function putFile(
  xhr: XhrLike,
  url: string,
  file: Blob,
  contentType: string,
  onProgress: (loaded: number, total: number) => void,
  signal: AbortSignal,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const onAbort = () => xhr.abort();

    xhr.upload.onprogress = (event) => {
      onProgress(event.loaded, event.lengthComputable ? event.total : file.size);
    };

    xhr.onload = () => {
      signal.removeEventListener("abort", onAbort);

      if (xhr.status >= 200 && xhr.status < 300) {
        resolve();
      } else {
        reject(new PutError(putFailureMessage(xhr.status)));
      }
    };

    xhr.onerror = () => {
      signal.removeEventListener("abort", onAbort);
      reject(new PutError(putFailureMessage(0)));
    };

    xhr.onabort = () => {
      signal.removeEventListener("abort", onAbort);
      reject(new DOMException("Upload cancelled", "AbortError"));
    };

    xhr.open("PUT", url);
    xhr.setRequestHeader("Content-Type", contentType);
    signal.addEventListener("abort", onAbort, { once: true });
    xhr.send(file);
  });
}

function isAbort(error: Error): boolean {
  return error.name === "AbortError";
}

/**
 * One file's upload. Construct, `start()`, and listen through `onChange` (called with each new
 * snapshot). `cancel()` stops it wherever it is; `retry()` runs a failed one again.
 */
export class UploadTask {
  readonly file: File;

  private current: UploadSnapshot;

  private controller: AbortController | null = null;

  private checksum: string | null = null;

  private readonly deps: UploadDeps;

  private readonly onChange: (snapshot: UploadSnapshot) => void;

  constructor(file: File, deps: UploadDeps, onChange: (snapshot: UploadSnapshot) => void) {
    this.file = file;
    this.deps = deps;
    this.onChange = onChange;
    this.current = {
      phase: "hashing",
      loaded: 0,
      total: file.size,
      signedId: null,
      error: null,
    };
  }

  get snapshot(): UploadSnapshot {
    return this.current;
  }

  /** Runs the upload; resolves when it has settled (done, failed or cancelled). */
  start(): Promise<void> {
    if (this.controller !== null || this.current.phase === "done") {
      return Promise.resolve();
    }

    const controller = new AbortController();

    this.controller = controller;

    return this.run(controller.signal).finally(() => {
      if (this.controller === controller) {
        this.controller = null;
      }
    });
  }

  /** Runs a failed upload again from a fresh blob. */
  retry(): Promise<void> {
    if (this.current.phase !== "failed") {
      return Promise.resolve();
    }

    this.set({ phase: "hashing", loaded: 0, error: null });

    return this.start();
  }

  /** Stops it for good (a finished upload stays done: its blob is simply never attached). */
  cancel(): void {
    if (this.current.phase === "done" || this.current.phase === "cancelled") {
      return;
    }

    this.controller?.abort();
    this.controller = null;
    this.set({ phase: "cancelled", error: null });
  }

  private set(patch: Partial<UploadSnapshot>): void {
    this.current = { ...this.current, ...patch };
    this.onChange(this.current);
  }

  private async run(signal: AbortSignal): Promise<void> {
    const { file, deps } = this;
    const contentType = contentTypeOf(file);

    try {
      const sizeError = uploadSizeError(file, deps.limitBytes);

      if (sizeError !== null) throw new Error(sizeError);
      this.set({ phase: "hashing", loaded: 0, error: null });
      this.checksum ??= await deps.hash(file, signal);
      signal.throwIfAborted();
      this.set({ phase: "starting" });

      const upload = await deps.start({
        filename: file.name,
        byteSize: file.size,
        checksum: this.checksum,
        contentType,
      });

      signal.throwIfAborted();
      this.set({ phase: "uploading", loaded: 0 });

      await putFile(
        deps.createXhr(),
        upload.uploadUrl,
        file,
        contentType,
        (loaded, total) => this.set({ loaded, total }),
        signal,
      );

      this.set({ phase: "done", loaded: file.size, signedId: upload.signedId });
    } catch (caught) {
      if (signal.aborted || this.current.phase === "cancelled") {
        return;
      }

      const error = caught instanceof Error ? caught : new Error("Upload failed.");

      if (isAbort(error)) {
        return;
      }

      this.set({ phase: "failed", error: error.message || "Upload failed." });
    }
  }
}
