/**
 * Vitest network stubs over the in-memory mock backend (mock/server.ts): `fetch` for `/api/v1/*`
 * and `/__mock/*` (JSON) and for `/rails/active_storage/*` and `/icons/*` (bytes: the direct
 * upload `PUT`, blob downloads, icon images), a small `XMLHttpRequest` for uploads that report
 * progress, and a minimal in-process `WebSocket` bridged to the sync hub. Tests opt in per file;
 * nothing installs this globally.
 *
 * ```ts
 * const network = installMockNetwork();
 * afterEach(() => network.restore());
 * ```
 */
import { vi } from "vitest";
import { formJson, type Json, parseJson } from "../../mock/json.ts";
import {
  createMockServer,
  isBinaryPath,
  isMockPath,
  type MockBinaryResponse,
  type MockHeaders,
  type MockServer,
} from "../../mock/server.ts";
import { parseClientFrame, type SyncConnection } from "../../mock/sync.ts";

export interface MockNetwork {
  readonly server: MockServer;
  /** Sockets the code under test opened, oldest first. */
  readonly sockets: readonly MockWebSocket[];
  /** Closes the sockets, puts the real globals back and disposes a server it created. */
  restore(): void;
}

const SYNC_PATH = "/api/v1/sync";

function baseUrl(): string {
  return globalThis.location?.href ?? "http://localhost/";
}

/** A `close` event with its code (`CloseEvent` isn't in every test environment). */
class MockCloseEvent extends Event {
  readonly code: number;
  readonly reason: string;
  readonly wasClean: boolean;

  constructor(code: number, reason: string, wasClean: boolean) {
    super("close");
    this.code = code;
    this.reason = reason;
    this.wasClean = wasClean;
  }
}

type Handler = ((event: Event) => void) | null;

/**
 * The parts of `WebSocket` app code and Effect's `Socket` use: `readyState`, `send`, `close`,
 * open/message/close/error as events and `on*` handlers. Opens, delivers and closes on
 * microtasks, so it is asynchronous like the real thing but needs no timers.
 */
export class MockWebSocket extends EventTarget {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;
  readonly CONNECTING = 0;
  readonly OPEN = 1;
  readonly CLOSING = 2;
  readonly CLOSED = 3;
  readonly url: string;
  readonly protocol = "";
  readonly extensions = "";
  readonly bufferedAmount = 0;
  binaryType: BinaryType = "blob";
  readyState = 0;
  onopen: Handler = null;
  onmessage: Handler = null;
  onclose: Handler = null;
  onerror: Handler = null;
  /** Every frame this socket sent, parsed, for assertions. */
  readonly sent: Json[] = [];
  #connection: SyncConnection | null = null;

  constructor(server: MockServer, url: string | URL) {
    super();
    this.url = new URL(url, baseUrl()).href;

    const path = new URL(this.url).pathname;

    queueMicrotask(() => {
      if (this.readyState !== MockWebSocket.CONNECTING) return;

      if (path !== SYNC_PATH) {
        this.#fire(new Event("error"));
        this.#finish(1006, "", false);

        return;
      }

      this.#connection = server.connect(
        (frame) => {
          const data = JSON.stringify(frame);

          queueMicrotask(() => {
            if (this.readyState === MockWebSocket.OPEN) {
              this.#fire(new MessageEvent("message", { data }));
            }
          });
        },
        () => this.#finish(1006, "", false),
      );
      this.readyState = MockWebSocket.OPEN;
      this.#fire(new Event("open"));
    });
  }

  send(data: string): void {
    if (this.readyState === MockWebSocket.CONNECTING) {
      throw new DOMException("Still in CONNECTING state.", "InvalidStateError");
    }

    if (this.readyState !== MockWebSocket.OPEN) return;

    const json = parseJson(data);
    const frame = parseClientFrame(json);

    if (json !== undefined) this.sent.push(json);

    queueMicrotask(() => {
      if (frame === null) {
        this.close(1003, "unsupported frame");

        return;
      }

      this.#connection?.receive(frame);
    });
  }

  close(code = 1000, reason = ""): void {
    if (this.readyState >= MockWebSocket.CLOSING) return;
    this.readyState = MockWebSocket.CLOSING;
    queueMicrotask(() => this.#finish(code, reason, true));
  }

  /** Hangs up from the "server" side without a close handshake (code 1006). */
  drop(): void {
    this.#finish(1006, "", false);
  }

  #finish(code: number, reason: string, wasClean: boolean) {
    if (this.readyState === MockWebSocket.CLOSED) return;
    this.readyState = MockWebSocket.CLOSED;
    this.#connection?.close();
    this.#connection = null;
    this.#fire(new MockCloseEvent(code, reason, wasClean));
  }

  #fire(event: Event) {
    this.dispatchEvent(event);

    const handler = {
      open: this.onopen,
      message: this.onmessage,
      close: this.onclose,
      error: this.onerror,
    }[event.type];

    handler?.(event);
  }
}

function headersOf(headers: Headers): MockHeaders {
  const record: Record<string, string> = {};

  headers.forEach((value, name) => {
    record[name] = value;
  });

  return record;
}

/** The request body as JSON. Read through `Response`, which takes every `BodyInit` kind. */
async function bodyOf(body: BodyInit | null | undefined): Promise<Json | undefined> {
  if (body === null || body === undefined) return undefined;

  if (body instanceof FormData) return formJson(body);

  const text = await new Response(body).text();

  return text.trim() === "" ? undefined : parseJson(text);
}

/** The request body as bytes, or `null` for none. */
async function bytesOf(body: BodyInit | Document | XMLHttpRequestBodyInit | null | undefined) {
  if (body === null || body === undefined) return null;

  if (body instanceof Uint8Array) return body;

  if (body instanceof ArrayBuffer) return new Uint8Array(body);

  // jsdom's Blob isn't one Node's Response understands (it would read "[object Blob]").
  if (body instanceof Blob) return blobBytes(body);

  // SAFETY: every `XMLHttpRequest` body the upload code sends (Blob, File, ArrayBuffer, string)
  // is also a `BodyInit`; a `Document` body isn't used against the mock.
  return new Uint8Array(await new Response(body as BodyInit).arrayBuffer());
}

/** Reads a jsdom Blob or File through FileReader, which every jsdom version implements. */
function blobBytes(blob: Blob): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();

    reader.onload = () => {
      // SAFETY: readAsArrayBuffer always yields an ArrayBuffer result on load.
      resolve(new Uint8Array(reader.result as ArrayBuffer));
    };

    reader.onerror = () => reject(reader.error);
    reader.readAsArrayBuffer(blob);
  });
}

/** A fetch `Response` for a byte route; statuses that can't carry a body get none. */
function binaryResponse(result: MockBinaryResponse): Response {
  const headers = new Headers(result.headers);

  if (result.contentType !== null) headers.set("Content-Type", result.contentType);

  const empty = result.status === 204 || result.status === 304 || result.bytes.length === 0;

  // Copied into a fresh ArrayBuffer so the body type is exact whatever backs the bytes.
  return new Response(empty ? null : result.bytes.slice().buffer, {
    status: result.status,
    headers,
  });
}

type XhrHandler = ((event: ProgressEvent) => void) | null;

function progressEvent(type: string, loaded: number, total: number): ProgressEvent {
  return new ProgressEvent(type, { lengthComputable: true, loaded, total });
}

/** `xhr.upload`: where upload progress is reported. */
class MockXhrUpload extends EventTarget {
  onprogress: XhrHandler = null;
  onload: XhrHandler = null;
  onloadstart: XhrHandler = null;
  onloadend: XhrHandler = null;

  fire(type: "loadstart" | "progress" | "load" | "loadend", loaded: number, total: number) {
    const event = progressEvent(type, loaded, total);

    this.dispatchEvent(event);
    ({
      loadstart: this.onloadstart,
      progress: this.onprogress,
      load: this.onload,
      loadend: this.onloadend,
    })[type]?.(event);
  }
}

/**
 * The parts of `XMLHttpRequest` an upload with a progress bar uses: `open`, `setRequestHeader`,
 * `send`, `abort`, `upload` progress, `status`, `response`/`responseText`, and the load, error,
 * abort and readystatechange events. Requests go to the mock like the `fetch` stub's do; the
 * upload reports progress at the start and when the server answers (held uploads stay at 0).
 */
export class MockXMLHttpRequest extends EventTarget {
  static readonly UNSENT = 0;
  static readonly OPENED = 1;
  static readonly HEADERS_RECEIVED = 2;
  static readonly LOADING = 3;
  static readonly DONE = 4;
  readonly upload = new MockXhrUpload();
  readyState = 0;
  status = 0;
  statusText = "";
  responseType: XMLHttpRequestResponseType = "";
  response: ArrayBuffer | Blob | Json | null = null;
  responseText = "";
  withCredentials = false;
  timeout = 0;
  onload: XhrHandler = null;
  onerror: XhrHandler = null;
  onabort: XhrHandler = null;
  onloadend: XhrHandler = null;
  onreadystatechange: ((event: Event) => void) | null = null;
  readonly #server: MockServer;
  #method = "GET";
  #url = "";
  #headers: Record<string, string> = {};
  #responseHeaders = new Headers();
  #aborted = false;

  constructor(server: MockServer) {
    super();
    this.#server = server;
  }

  open(method: string, url: string | URL): void {
    this.#method = method.toUpperCase();
    this.#url = new URL(url, baseUrl()).href;
    this.#headers = {};
    this.#aborted = false;
    this.#state(MockXMLHttpRequest.OPENED);
  }

  setRequestHeader(name: string, value: string): void {
    this.#headers[name.toLowerCase()] = value;
  }

  getResponseHeader(name: string): string | null {
    return this.#responseHeaders.get(name);
  }

  getAllResponseHeaders(): string {
    let all = "";

    this.#responseHeaders.forEach((value, name) => {
      all += `${name}: ${value}\r\n`;
    });

    return all;
  }

  send(body?: Document | XMLHttpRequestBodyInit | null): void {
    void this.#run(body ?? null);
  }

  abort(): void {
    if (
      this.readyState === MockXMLHttpRequest.DONE ||
      this.readyState === MockXMLHttpRequest.UNSENT
    ) {
      return;
    }

    this.#aborted = true;
    this.#state(MockXMLHttpRequest.DONE);
    this.#fire("abort");
    this.#fire("loadend");
  }

  async #run(body: Document | XMLHttpRequestBodyInit | null) {
    const url = new URL(this.#url);
    const bytes = await bytesOf(body);
    const total = bytes?.length ?? 0;

    this.upload.fire("loadstart", 0, total);
    this.upload.fire("progress", 0, total);

    let response: Response;

    if (isBinaryPath(url.pathname)) {
      response = binaryResponse(
        await this.#server.handleBinary({
          method: this.#method,
          path: `${url.pathname}${url.search}`,
          headers: this.#headers,
          bytes,
        }),
      );
    } else if (isMockPath(url.pathname)) {
      const text = bytes === null ? "" : new TextDecoder().decode(bytes);

      const result = await this.#server.handle({
        method: this.#method,
        path: url.pathname,
        query: url.searchParams,
        body: text.trim() === "" ? undefined : parseJson(text),
        headers: this.#headers,
      });

      response = jsonResponse(result.status, result.json);
    } else {
      if (this.#aborted) return;

      this.#state(MockXMLHttpRequest.DONE);
      this.#fire("error");
      this.#fire("loadend");

      return;
    }

    if (this.#aborted) return;

    this.upload.fire("progress", total, total);
    this.upload.fire("load", total, total);
    this.upload.fire("loadend", total, total);
    this.status = response.status;
    this.statusText = response.statusText;
    this.#responseHeaders = response.headers;
    this.#state(MockXMLHttpRequest.HEADERS_RECEIVED);

    const buffer = await response.arrayBuffer();

    this.responseText = new TextDecoder().decode(buffer);

    switch (this.responseType) {
      case "json":
        this.response = this.responseText === "" ? null : (parseJson(this.responseText) ?? null);
        break;
      case "arraybuffer":
        this.response = buffer;
        break;
      case "blob":
        this.response = new Blob([buffer], { type: response.headers.get("Content-Type") ?? "" });
        break;
      default:
        this.response = this.responseText;
    }

    this.#state(MockXMLHttpRequest.DONE);
    this.#fire("load");
    this.#fire("loadend");
  }

  #state(state: number) {
    this.readyState = state;

    const event = new Event("readystatechange");

    this.dispatchEvent(event);
    this.onreadystatechange?.(event);
  }

  #fire(type: "load" | "error" | "abort" | "loadend") {
    const event = progressEvent(type, 0, 0);

    this.dispatchEvent(event);
    ({ load: this.onload, error: this.onerror, abort: this.onabort, loadend: this.onloadend })[
      type
    ]?.(event);
  }
}

/** A fetch `Response` for a JSON route; a 204 has no body. */
function jsonResponse(status: number, json: Json): Response {
  if (status === 204) return new Response(null, { status });

  return new Response(JSON.stringify(json), {
    status,
    headers: { "Content-Type": "application/json; charset=utf-8" },
  });
}

/**
 * Stubs `fetch`, `XMLHttpRequest` and `WebSocket` with `vi.stubGlobal` so the code under test
 * talks to `server` (a fresh seeded mock without simulation by default). Other URLs reject like
 * a network error.
 */
export function installMockNetwork(server?: MockServer): MockNetwork {
  const owned = server === undefined;
  const backend = server ?? createMockServer({ simulate: false });
  const sockets: MockWebSocket[] = [];

  const fetchStub = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    // Not `new Request(input, init)`: Node's Request rejects jsdom's AbortSignal.
    const original = input instanceof Request ? input : null;
    const url = new URL(original === null ? input.toString() : original.url, baseUrl());
    const method = init?.method ?? original?.method ?? "GET";

    if (isBinaryPath(url.pathname)) {
      const bytes =
        init?.body !== undefined
          ? await bytesOf(init.body)
          : original === null || original.body === null
            ? null
            : new Uint8Array(await original.arrayBuffer());

      return binaryResponse(
        await backend.handleBinary({
          method: method.toUpperCase(),
          path: `${url.pathname}${url.search}`,
          headers: headersOf(new Headers(init?.headers ?? original?.headers)),
          bytes,
        }),
      );
    }

    if (!isMockPath(url.pathname)) {
      throw new TypeError(`mock network: no route to ${method} ${url.href}`);
    }

    const body =
      init?.body !== undefined
        ? await bodyOf(init.body)
        : parseJson((await original?.text()) ?? "");

    const response = await backend.handle({
      method,
      path: url.pathname,
      query: url.searchParams,
      body,
      headers: headersOf(new Headers(init?.headers ?? original?.headers)),
    });

    return jsonResponse(response.status, response.json);
  };

  class BoundWebSocket extends MockWebSocket {
    constructor(url: string | URL, _protocols?: string | string[]) {
      super(backend, url);
      sockets.push(this);
    }
  }

  class BoundXMLHttpRequest extends MockXMLHttpRequest {
    constructor() {
      super(backend);
    }
  }

  vi.stubGlobal("fetch", fetchStub);
  vi.stubGlobal("XMLHttpRequest", BoundXMLHttpRequest);
  vi.stubGlobal("WebSocket", BoundWebSocket);

  return {
    server: backend,
    sockets,
    restore() {
      for (const socket of sockets) socket.close();
      vi.unstubAllGlobals();

      if (owned) backend.dispose();
    },
  };
}
