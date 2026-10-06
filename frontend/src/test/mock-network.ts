/**
 * Vitest network stubs over the in-memory mock backend (mock/server.ts): `fetch` for `/api/v1/*`
 * and `/__mock/*`, and a minimal in-process `WebSocket` bridged to the sync hub. Tests opt in per
 * file; nothing installs this globally.
 *
 * ```ts
 * const network = installMockNetwork();
 * afterEach(() => network.restore());
 * ```
 */
import { vi } from "vitest";
import { type Json, parseJson } from "../../mock/json.ts";
import {
  createMockServer,
  isMockPath,
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

  const text = await new Response(body).text();

  return text.trim() === "" ? undefined : parseJson(text);
}

/**
 * Stubs `fetch` and `WebSocket` with `vi.stubGlobal` so the code under test talks to `server`
 * (a fresh seeded mock without simulation by default). Other URLs reject like a network error.
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

    return new Response(JSON.stringify(response.json), {
      status: response.status,
      headers: { "Content-Type": "application/json; charset=utf-8" },
    });
  };

  class BoundWebSocket extends MockWebSocket {
    constructor(url: string | URL, _protocols?: string | string[]) {
      super(backend, url);
      sockets.push(this);
    }
  }

  vi.stubGlobal("fetch", fetchStub);
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
