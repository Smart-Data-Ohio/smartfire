import { describe, expect, it } from "vitest";
import { cacheName } from "./cache.ts";
import {
  type CacheCoordinator,
  type WorkerBuild,
  WorkerCache,
  type WorkerCacheStorage,
} from "./runtime.ts";

const origin = "https://smartfire.test";

/** CacheStorage.delete unlinks the cache; a previously opened handle can still receive writes. */
class MemoryStorage implements WorkerCacheStorage {
  private readonly contents = new Map<string, Map<string, Response>>();

  async keys(): Promise<string[]> {
    return [...this.contents.keys()];
  }

  async delete(name: string): Promise<boolean> {
    return this.contents.delete(name);
  }

  async open(name: string): Promise<Pick<Cache, "keys" | "put">> {
    const content = this.contents.get(name) ?? new Map<string, Response>();

    this.contents.set(name, content);

    return {
      put: async (request, response) => {
        content.set(new Request(request).url, response.clone());
      },
      keys: async () => [...content.keys()].map((url) => new Request(url)),
    };
  }

  async match(
    request: Parameters<CacheStorage["match"]>[0],
    options?: Parameters<CacheStorage["match"]>[1],
  ): Promise<Response | undefined> {
    const url = new Request(request).url;

    if (options?.cacheName !== undefined) {
      return this.contents.get(options.cacheName)?.get(url)?.clone();
    }

    for (const cache of this.contents.values()) {
      const response = cache.get(url);

      if (response !== undefined) {
        return response.clone();
      }
    }

    return undefined;
  }
}

/** Shared by distinct worker contexts, as the origin-wide Web Lock is in the browser. */
class SharedCoordinator implements CacheCoordinator {
  private pending: Promise<void> = Promise.resolve();

  run<Result>(work: () => Promise<Result>): Promise<Result> {
    const result = this.pending.then(work);

    this.pending = result.then(
      () => undefined,
      () => undefined,
    );

    return result;
  }
}

function build(version: string): WorkerBuild & { readonly precache: readonly [string, string] } {
  return {
    version,
    page: `/app/assets/index-${version.repeat(8)}.js`,
    offline: "/app/offline.html",
    precache: [`/app/assets/chunk-${version.repeat(8)}.js`, "/app/offline.html"],
  };
}

const network: typeof fetch = async () => new Response("complete asset bytes");

describe("service worker cache lifecycle coordination", () => {
  it("pins an activated rollback cache while its installer fetches, without blocking serving or pruning", async () => {
    const storage = new MemoryStorage();
    const coordinator = new SharedCoordinator();
    const a = new WorkerCache(build("a"), origin, storage, network, coordinator);
    const c = new WorkerCache(build("c"), origin, storage, network, coordinator);
    const d = new WorkerCache(build("d"), origin, storage, network, coordinator);
    const liveA = [new URL(build("a").page, origin).href];

    await a.install();
    await a.activate(liveA);
    await c.install();
    await c.activate(liveA);
    await d.install();
    await d.activate(liveA);

    const fetching = Promise.withResolvers<void>();
    const resume = Promise.withResolvers<void>();

    const rollbackNetwork: typeof fetch = async () => {
      fetching.resolve();
      await resume.promise;

      return new Response("rollback asset bytes");
    };

    const rollback = new WorkerCache(build("a"), origin, storage, rollbackNetwork, coordinator);
    const installing = rollback.install();

    await fetching.promise;

    try {
      // The last A page closed. D's fetch/message pruning must not delete its reused cache.
      await d.prune([]);
      expect(await storage.keys()).toContain(cacheName("a"));
      const asset = await d.response(new Request(new URL(build("a").precache[0], origin)));

      expect(await asset?.text()).toBe("complete asset bytes");
    } finally {
      resume.resolve();
    }

    await installing;
    await rollback.activate([]);
    expect((await storage.keys()).sort()).toEqual([cacheName("a"), cacheName("d")]);
    expect(
      await (
        await storage.match(new URL(build("a").precache[0], origin).href, {
          cacheName: cacheName("a"),
        })
      )?.text(),
    ).toBe("rollback asset bytes");
  });

  it("uses the global activation order when a stale older worker finishes pruning", async () => {
    const storage = new MemoryStorage();
    const coordinator = new SharedCoordinator();

    const a = new WorkerCache(build("a"), origin, storage, network, coordinator);

    const workers = [
      a,
      ...["b", "c", "d"].map(
        (version) => new WorkerCache(build(version), origin, storage, network, coordinator),
      ),
    ];

    const liveA = [new URL(build("a").page, origin).href];

    for (const worker of workers) {
      await worker.install();
      await worker.activate(liveA);
    }

    await a.prune([]);
    expect((await storage.keys()).sort()).toEqual([cacheName("c"), cacheName("d")]);
  });

  it("conservatively retains a persistent pin after a rollback installation fails", async () => {
    const storage = new MemoryStorage();
    const coordinator = new SharedCoordinator();
    const liveA = [new URL(build("a").page, origin).href];

    for (const version of ["a", "c", "d"]) {
      const worker = new WorkerCache(build(version), origin, storage, network, coordinator);

      await worker.install();
      await worker.activate(liveA);
    }

    const failure = new Error("The installer went offline");

    const failingNetwork: typeof fetch = async () => {
      throw failure;
    };

    const rollback = new WorkerCache(build("a"), origin, storage, failingNetwork, coordinator);

    await expect(rollback.install()).rejects.toBe(failure);
    await new WorkerCache(build("d"), origin, storage, network, coordinator).prune([]);
    expect((await storage.keys()).sort()).toEqual([cacheName("a"), cacheName("c"), cacheName("d")]);
  });

  it("keeps installation and serving functional but never prunes without cross-worker locking", async () => {
    const storage = new MemoryStorage();

    for (const version of ["a", "b", "c"]) {
      const worker = new WorkerCache(build(version), origin, storage, network);

      await worker.install();
      await worker.activate([]);
      await worker.prune([]);

      const response = await worker.response(
        new Request(new URL(build(version).precache[0], origin)),
      );

      expect(await response?.text()).toBe("complete asset bytes");
    }

    expect((await storage.keys()).sort()).toEqual([cacheName("a"), cacheName("b"), cacheName("c")]);
  });
});
