// @vitest-environment node

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { runInNewContext } from "node:vm";
import { afterEach, describe, expect, it, vi } from "vitest";
import { type NetworkRoute, networkRoute } from "./routing.ts";
import type { WorkerBuild } from "./runtime.ts";

const origin = "https://smartfire.test";

const build: WorkerBuild = {
  version: "a",
  page: "/app/assets/index-aaaaaaaa.js",
  offline: "/offline.html",
  precache: ["/app/assets/index-aaaaaaaa.js", "/offline.html"],
};

class InstallEvent extends Event {
  readonly completions: Promise<unknown>[] = [];

  waitUntil(completion: Promise<unknown>): void {
    this.completions.push(completion);
  }
}

async function worker(kind: "SPA" | "classic") {
  const handlers = new Map<string, (event: InstallEvent) => void>();
  const precache = Promise.withResolvers<void>();
  const add = vi.fn(() => precache.promise);
  const put = vi.fn(async () => undefined);

  const storage = {
    keys: async () => [],
    open: vi.fn(async () => ({ add, put })),
  };

  const network = vi.fn(async (_request: Request) => {
    await precache.promise;

    return new Response("public asset");
  });

  const skipWaiting = vi.fn(async () => undefined);

  const scope = {
    location: new URL(origin),
    navigator: {},
    smartfireBuild: build,
    skipWaiting,
    addEventListener: (type: string, handler: (event: InstallEvent) => void) => {
      handlers.set(type, handler);
    },
  };

  if (kind === "SPA") {
    vi.resetModules();
    vi.stubGlobal("self", scope);
    vi.stubGlobal("caches", storage);
    vi.stubGlobal("fetch", network);
    await import("./worker.ts");
  } else {
    runInNewContext(readFileSync(resolve("../crates/spa/pwa/service_worker.js"), "utf8"), {
      self: scope,
      caches: storage,
      fetch: network,
      URL,
    });
  }

  function install(addRoutes?: (route: NetworkRoute) => Promise<void>) {
    const event = new InstallEvent("install");

    if (addRoutes !== undefined) Object.assign(event, { addRoutes });

    const handler = handlers.get("install");

    expect(handler).toBeDefined();
    handler?.(event);
    expect(event.completions).toHaveLength(1);

    const completion = Promise.all(event.completions);

    // Observe early failures while precache is checked; the caller still asserts this promise.
    void completion.catch(() => undefined);

    return { event, completion };
  }

  function expectPrecached() {
    if (kind === "SPA") {
      expect(network.mock.calls.map(([request]) => request.url)).toEqual(
        build.precache.map((path) => new URL(path, origin).href),
      );
      expect(storage.open).toHaveBeenCalledWith("smartfire-spa-a");
      // The actual WorkerCache writes its installation marker before the two public assets.
      expect(put).toHaveBeenCalledTimes(build.precache.length + 1);
    } else {
      expect(storage.open).toHaveBeenCalledExactlyOnceWith("smartfire-static-v1");
      expect(add).toHaveBeenCalledExactlyOnceWith("/offline.html");
    }
  }

  async function waitForPrecache() {
    await vi.waitFor(() => {
      expect(kind === "SPA" ? network : add).toHaveBeenCalledOnce();
    });
  }

  return { install, precache, skipWaiting, expectPrecached, waitForPrecache };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe.each(["SPA", "classic"] as const)("%s worker install handler", (kind) => {
  it("calls addRoutes synchronously with its receiver and waits before skipWaiting", async () => {
    const f = await worker(kind);
    const registered = Promise.withResolvers<void>();
    let receiver: unknown;

    const addRoutes = vi.fn(function (this: InstallEvent, _route: NetworkRoute) {
      receiver = this;

      return registered.promise;
    });

    const installation = f.install(addRoutes);

    expect(addRoutes).toHaveBeenCalledOnce();
    expect(receiver).toBe(installation.event);
    expect(addRoutes).toHaveBeenCalledWith(
      kind === "SPA"
        ? networkRoute(origin, build)
        : {
            source: "network",
            condition: {
              not: {
                or: [
                  { requestMethod: "GET", requestMode: "navigate" },
                  { requestMethod: "GET", urlPattern: `${origin}/assets/*` },
                  { requestMethod: "GET", urlPattern: `${origin}/app/assets/*` },
                  { requestMethod: "GET", urlPattern: `${origin}/offline.html` },
                ],
              },
            },
          },
    );
    f.precache.resolve();
    await vi.waitFor(() => f.expectPrecached());
    expect(f.skipWaiting).not.toHaveBeenCalled();
    registered.resolve();
    await expect(installation.completion).resolves.toHaveLength(1);
    expect(f.skipWaiting).toHaveBeenCalledOnce();
  });

  it.each(["absent", "reject", "throw"])(
    "precaches and skips waiting when addRoutes is %s",
    async (support) => {
      const f = await worker(kind);

      const addRoutes = vi.fn(() => {
        const error = new TypeError("The not/or condition is unsupported");

        if (support === "throw") throw error;

        return Promise.reject(error);
      });

      const installation = f.install(support === "absent" ? undefined : addRoutes);

      expect(addRoutes).toHaveBeenCalledTimes(support === "absent" ? 0 : 1);
      await f.waitForPrecache();
      expect(f.skipWaiting).not.toHaveBeenCalled();
      f.precache.resolve();
      await expect(installation.completion).resolves.toHaveLength(1);
      f.expectPrecached();
      expect(f.skipWaiting).toHaveBeenCalledOnce();
    },
  );
});
