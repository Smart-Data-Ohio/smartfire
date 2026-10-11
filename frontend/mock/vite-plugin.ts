/**
 * `smartfireMock()`: serves the in-memory mock backend from the Vite dev server, so the SPA runs
 * with no Rust app (`SMARTFIRE_MOCK=1 pnpm dev` or `vite --mode mock`). Dev server only; nothing
 * here reaches the production bundle.
 *
 * - `/api/v1/*`, `/__mock/*` and classic's tour stamp (`/users/me/tour`) go to
 *   `MockServer.handle` (held sends hold the HTTP response).
 * - `/rails/active_storage/*` (the direct-upload `PUT`, blob and thumbnail downloads) and
 *   `/icons/*` and `/assets/icons/brands/*` go to `MockServer.handleBinary` with the raw body, ahead of the `/rails` proxy.
 * - `/api/v1/sync` upgrades to a WebSocket bridged to `MockServer.connect`; every other upgrade
 *   (Vite's HMR) is left alone.
 * - `/users/:id/avatar` answers with a picture for a couple of people and 404 for the rest, so
 *   the UI's initials fallback shows.
 * - The dev `index.html` gets `<meta name="csrf-param">` and `<meta name="csrf-token">` with the
 *   mock's current token, as the Rust shell writes them.
 * - `vite preview --mode mock` serves the same backend in front of the built dist, and answers
 *   `/app/*` page loads with dist/index.html and the csrf meta in place of `<!--boot-->`, as the
 *   Rust shell does, so the production bundle (its CSS order, its chunking) can be checked too.
 * - `/__auth/*` serves the server-rendered pages the Rust tests write to
 *   e2e/fixtures/auth-pages (sign-in, joining, two-step sign-in...), byte for byte, so the
 *   Playwright pass can look at them without the Rust app.
 */
import { readFile } from "node:fs/promises";
import type { IncomingMessage, ServerResponse } from "node:http";
import { extname, join, normalize } from "node:path";
import type { Duplex } from "node:stream";
import type { Plugin, PreviewServer, ViteDevServer } from "vite";
import { type WebSocket, WebSocketServer } from "ws";
import { formJson, isMultipart, type Json, parseJson } from "./json.ts";
import { BOT_ID, USERS_WITH_PHOTOS } from "./seed.ts";
import {
  createMockServer,
  isBinaryPath,
  isMockPath,
  type MockHeaders,
  type MockServer,
} from "./server.ts";
import { parseClientFrame } from "./sync.ts";

/** Whether the dev server should use the mock: `SMARTFIRE_MOCK=1` or `--mode mock`. */
export function isMockEnabled(mode: string): boolean {
  return process.env.SMARTFIRE_MOCK === "1" || mode === "mock";
}

export interface SmartfireMockOptions {
  /** Seeds the workspace; defaults to `SMARTFIRE_MOCK_SEED` or 1. */
  readonly seed?: number;
  /** Ambient chatter and bot replies; defaults to on (`SMARTFIRE_MOCK_SIMULATE=0` turns it off). */
  readonly simulate?: boolean;
}

const SYNC_PATH = "/api/v1/sync";

const AUTH_PAGES_PREFIX = "/__auth/";

const AUTH_PAGES_DIR = join(import.meta.dirname, "..", "e2e", "fixtures", "auth-pages");

const AUTH_PAGE_TYPES = new Map([
  [".html", "text/html; charset=utf-8"],
  [".css", "text/css; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".woff2", "font/woff2"],
  [".svg", "image/svg+xml"],
  [".png", "image/png"],
]);

/** A file from e2e/fixtures/auth-pages, untransformed; 404 for anything outside it. */
async function serveAuthPage(path: string, response: ServerResponse) {
  const relative = normalize(path.slice(AUTH_PAGES_PREFIX.length));
  const type = AUTH_PAGE_TYPES.get(extname(relative));

  if (relative.startsWith("..") || type === undefined) {
    response.statusCode = 404;
    response.end();

    return;
  }

  try {
    const body = await readFile(join(AUTH_PAGES_DIR, relative));

    response.setHeader("Content-Type", type);
    response.setHeader("Cache-Control", "no-store");
    response.end(body);
  } catch {
    response.statusCode = 404;
    response.end();
  }
}

function readBody(request: IncomingMessage): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];

    request.on("data", (chunk: Buffer) => chunks.push(chunk));
    request.on("end", () => resolve(Buffer.concat(chunks)));
    request.on("error", reject);
  });
}

function headersOf(request: IncomingMessage): MockHeaders {
  const headers: Record<string, string | undefined> = {};

  for (const [name, value] of Object.entries(request.headers)) {
    headers[name] = Array.isArray(value) ? value.join(", ") : value;
  }

  return headers;
}

function sendJson(response: ServerResponse, status: number, json: Json) {
  response.statusCode = status;

  if (status === 204) {
    response.end();

    return;
  }

  response.setHeader("Content-Type", "application/json; charset=utf-8");
  response.setHeader("Cache-Control", "no-store");
  response.end(JSON.stringify(json));
}

/** A small flat picture for the people who have one (the rest get initials). */
function avatarSvg(userId: number): string {
  const [from, to] = userId === BOT_ID ? ["#ff7a18", "#d4145a"] : ["#7f5af0", "#2cb67d"];

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${from}"/><stop offset="1" stop-color="${to}"/></linearGradient></defs><rect width="64" height="64" fill="url(#g)"/><circle cx="32" cy="26" r="11" fill="#fff" fill-opacity=".85"/><path d="M12 60c2-12 10-18 20-18s18 6 20 18z" fill="#fff" fill-opacity=".85"/></svg>`;
}

async function serveHttp(mock: MockServer, request: IncomingMessage, response: ServerResponse) {
  const url = new URL(request.url ?? "/", "http://localhost");
  const raw = await readBody(request);
  const contentType = request.headers["content-type"];
  const multipart = isMultipart(contentType);
  const text = multipart ? "" : raw.toString("utf8");

  const body = multipart
    ? formJson(
        await new Response(new Uint8Array(raw), {
          headers: { "content-type": contentType ?? "" },
        }).formData(),
      )
    : text.trim() === ""
      ? undefined
      : parseJson(text);

  if (text.trim() !== "" && body === undefined) {
    sendJson(response, 400, { error: { message: "The request body isn't JSON" } });

    return;
  }

  const result = await mock.handle({
    method: request.method ?? "GET",
    path: url.pathname,
    query: url.searchParams,
    body,
    headers: headersOf(request),
  });

  sendJson(response, result.status, result.json);
}

/** The byte routes: the raw `PUT` body in, the blob or icon bytes out. */
async function serveBinary(mock: MockServer, request: IncomingMessage, response: ServerResponse) {
  const method = request.method ?? "GET";
  const raw = method === "PUT" || method === "POST" ? await readBody(request) : null;

  const result = await mock.handleBinary({
    method,
    path: request.url ?? "/",
    headers: headersOf(request),
    bytes: raw === null ? null : new Uint8Array(raw.buffer, raw.byteOffset, raw.byteLength),
  });

  response.statusCode = result.status;

  for (const [name, value] of Object.entries(result.headers)) response.setHeader(name, value);

  if (result.contentType !== null) response.setHeader("Content-Type", result.contentType);

  response.end(method === "HEAD" ? undefined : Buffer.from(result.bytes));
}

function bridgeSocket(mock: MockServer, socket: WebSocket) {
  const connection = mock.connect(
    (frame) => socket.send(JSON.stringify(frame)),
    () => socket.terminate(),
  );

  socket.on("message", (data, isBinary) => {
    const frame = isBinary ? null : parseClientFrame(parseJson(data.toString()));

    if (frame === null) {
      socket.close(1003, "unsupported frame");

      return;
    }

    connection.receive(frame);
  });
  socket.on("close", () => connection.close());
  socket.on("error", () => connection.close());
}

/** The csrf meta the Rust shell writes into the page head. */
function csrfMeta(token: string): string {
  return `<meta name="csrf-param" content="authenticity_token" />\n<meta name="csrf-token" content="${token}" />`;
}

/**
 * In preview, `/app/*` page loads (no file extension) get dist/index.html with the csrf meta in
 * place of `<!--boot-->`; everything else (the hashed assets) falls through to the static server.
 */
async function servePreviewShell(
  server: PreviewServer,
  token: string,
  response: ServerResponse,
): Promise<void> {
  const index = join(server.config.root, server.config.build.outDir, "index.html");
  const html = (await readFile(index, "utf8")).replace("<!--boot-->", csrfMeta(token));

  response.setHeader("Content-Type", "text/html; charset=utf-8");
  response.setHeader("Cache-Control", "no-store");
  response.end(html);
}

/** The Vite plugin. Only active for `vite serve` and `vite preview` with the mock enabled. */
export function smartfireMock(options: SmartfireMockOptions = {}): Plugin {
  let mock: MockServer | null = null;

  const current = (): MockServer => {
    if (mock === null) {
      mock = createMockServer({
        seed: options.seed ?? Number(process.env.SMARTFIRE_MOCK_SEED ?? 1),
        simulate: options.simulate ?? process.env.SMARTFIRE_MOCK_SIMULATE !== "0",
      });
    }

    return mock;
  };

  return {
    name: "smartfire-mock",
    apply: (_config, env) => env.command === "serve" && isMockEnabled(env.mode),
    configureServer(server) {
      attach(server);
    },
    configurePreviewServer(server) {
      server.middlewares.use((request, response, next) => {
        const path = new URL(request.url ?? "/", "http://localhost").pathname;

        if ((path === "/app" || path.startsWith("/app/")) && extname(path) === "") {
          servePreviewShell(server, current().csrfToken(), response).catch(next);

          return;
        }

        next();
      });
      attach(server);
    },
    transformIndexHtml: () => [
      // As the Rust shell renders it, ahead of index.html's blocking appearance script.
      {
        tag: "script",
        attrs: { type: "application/json", id: "boot" },
        children: current().inlineBoot(),
        injectTo: "head-prepend",
      },
      {
        tag: "meta",
        attrs: { name: "csrf-param", content: "authenticity_token" },
        injectTo: "head",
      },
      {
        tag: "meta",
        attrs: { name: "csrf-token", content: current().csrfToken() },
        injectTo: "head",
      },
    ],
  };

  function attach(server: ViteDevServer | PreviewServer) {
    // Playwright's request agent pools keep-alive sockets and ignores this server's
    // Keep-Alive timeout: the agent is `keepAlive: true` with no `timeout`, so Node
    // drops the advertised hint and reuses a socket this process has already closed.
    // Node's default idle close is a few seconds. The next `route.fetch` then fails
    // with `read ECONNRESET`. The scrollbar pagination spec hits that gap: its
    // forwarded `messages?before` fetch runs only after the reader has been dragging.
    // Leave idle mock sockets up for the life of this server.
    if (server.httpServer !== null && "keepAliveTimeout" in server.httpServer) {
      server.httpServer.keepAliveTimeout = 0;
    }

    const backend = current();
    const sockets = new WebSocketServer({ noServer: true });

    server.middlewares.use((request, response, next) => {
      const path = new URL(request.url ?? "/", "http://localhost").pathname;
      const avatar = /^\/users\/(\d+)\/avatar$/.exec(path);

      if (path.startsWith(AUTH_PAGES_PREFIX)) {
        serveAuthPage(path, response).catch(next);

        return;
      }

      if (avatar !== null) {
        const userId = Number(avatar[1]);

        if (!USERS_WITH_PHOTOS.has(userId)) {
          response.statusCode = 404;
          response.end();

          return;
        }

        response.setHeader("Content-Type", "image/svg+xml");
        response.end(avatarSvg(userId));

        return;
      }

      if (isBinaryPath(path)) {
        serveBinary(backend, request, response).catch(next);

        return;
      }

      if (!isMockPath(path) || path === SYNC_PATH) {
        next();

        return;
      }

      serveHttp(backend, request, response).catch(next);
    });

    server.httpServer?.on("upgrade", (request: IncomingMessage, socket: Duplex, head: Buffer) => {
      const path = new URL(request.url ?? "/", "http://localhost").pathname;

      if (path !== SYNC_PATH) return;

      sockets.handleUpgrade(request, socket, head, (ws) => bridgeSocket(backend, ws));
    });

    server.httpServer?.on("close", () => {
      backend.dispose();
      sockets.close();
    });

    server.config.logger.info(
      "  smartfire mock: /api/v1 and /api/v1/sync are served in memory; controls at /__mock/state",
    );
  }
}
