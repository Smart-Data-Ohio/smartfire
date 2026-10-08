import {
  CACHE_PREFIX,
  type CacheActivation,
  cacheName,
  latestCache,
  nextActivation,
  obsoleteCaches,
  previousCache,
} from "./cache.ts";
import { canCache, isSpaAsset, requestPolicy } from "./policy.ts";

export interface WorkerBuild {
  readonly version: string;
  readonly precache: readonly string[];
  readonly offline: string;
  readonly page: string;
}

/** The small Cache API surface used by the worker, also exercised by lifecycle tests. */
export interface WorkerCacheStorage {
  readonly keys: CacheStorage["keys"];
  readonly delete: CacheStorage["delete"];
  readonly match: CacheStorage["match"];
  readonly open: (name: string) => Promise<Pick<Cache, "keys" | "put">>;
}

export interface CacheCoordinator {
  run<Result>(work: () => Promise<Result>): Promise<Result>;
}

/** Browser APIs stay at this boundary; request selection and cache retention are pure modules. */
export class WorkerCache {
  readonly name: string;
  private readonly origin: string;
  private readonly storage: WorkerCacheStorage;
  private readonly fetch: typeof fetch;
  private readonly build: WorkerBuild;
  private readonly marker: string;
  private readonly currentUrls: ReadonlySet<string>;
  private readonly assetPrefix: string;
  private previousUrls: Promise<ReadonlySet<string>> | null = null;
  private readonly coordinator: CacheCoordinator | null;

  constructor(
    build: WorkerBuild,
    origin: string,
    storage: WorkerCacheStorage,
    network: typeof fetch,
    coordinator: CacheCoordinator | null = null,
  ) {
    this.build = build;
    this.origin = origin;
    this.storage = storage;
    this.fetch = (...args) => network(...args);
    this.coordinator = coordinator;
    this.name = cacheName(build.version);
    this.marker = new URL(`${build.offline}.activation`, origin).href;
    this.currentUrls = new Set(build.precache.map((path) => new URL(path, origin).href));
    this.assetPrefix = new URL("assets/", new URL(build.offline, origin)).pathname;
  }

  handles(request: Request): boolean {
    const url = new URL(request.url);

    return (
      request.method === "GET" &&
      url.origin === this.origin &&
      (requestPolicy(request, this.origin, this.currentUrls) !== "network" ||
        isSpaAsset(url.pathname, this.assetPrefix))
    );
  }

  async install(): Promise<void> {
    await this.coordinate(async () => {
      const existing = (await this.activations()).find((cache) => cache.name === this.name);
      const cache = await this.storage.open(this.name);

      // A rollback reuses an activated cache. Pin it under the same origin-wide lock used by
      // pruning, preserving its activation/page so existing tabs can still read its assets.
      await cache.put(
        this.marker,
        new Response(
          JSON.stringify({
            activation: existing?.activation ?? null,
            page: existing?.page ?? new URL(this.build.page, this.origin).href,
            installing: true,
          }),
        ),
      );
    });
    const cache = await this.storage.open(this.name);

    // The persistent pin protects the cache while these network requests run without a lock.
    // addAll would cache no-store replies. Reject an incomplete precache instead of activating it.
    for (const url of this.currentUrls) {
      const request = new Request(url, { cache: "reload" });
      const response = await this.fetch(request);

      if (!canCache(response)) {
        throw new Error(`Precache response cannot be stored: ${url}`);
      }

      await cache.put(request, response);
    }
  }

  private async coordinate<Result>(work: () => Promise<Result>): Promise<Result> {
    return this.coordinator === null ? work() : this.coordinator.run(work);
  }

  private async activations(): Promise<CacheActivation[]> {
    const activations: CacheActivation[] = [];

    for (const name of await this.storage.keys()) {
      if (!name.startsWith(CACHE_PREFIX)) {
        continue;
      }

      const marker = await this.storage.match(this.marker, { cacheName: name });
      const content = marker === undefined ? "" : await marker.text();
      let activation: number | null = null;
      let page: string | null = null;
      let installing = false;

      try {
        const value: unknown = JSON.parse(content);

        if (Number.isSafeInteger(value)) {
          // A worker from before live-build reporting left a numeric activation marker.
          activation = Number(value);
        } else if (value instanceof Object) {
          if ("activation" in value && Number.isSafeInteger(value.activation)) {
            activation = Number(value.activation);
          }

          if ("page" in value && String(value.page) === value.page) {
            page = value.page;
          }

          installing = "installing" in value && value.installing === true;
        }
      } catch {
        // Incomplete or unrecognized internal metadata never proves a cache is activated.
      }

      activations.push({
        name,
        activation: activation !== null && Number.isSafeInteger(activation) ? activation : null,
        page,
        installing,
      });
    }

    return activations;
  }

  async activate(livePages: readonly (string | null)[]): Promise<void> {
    await this.coordinate(async () => {
      const activations = await this.activations();
      const cache = await this.storage.open(this.name);

      // This internal record is written only on activation, so failed installations never count.
      await cache.put(
        this.marker,
        new Response(
          JSON.stringify({
            activation: nextActivation(activations),
            page: new URL(this.build.page, this.origin).href,
            installing: false,
          }),
        ),
      );
      await this.pruneLocked(livePages);
      this.previousUrls = null;
    });
  }

  async prune(livePages: readonly (string | null)[]): Promise<void> {
    if (this.coordinator === null) {
      // Without cross-worker exclusion, reading a pin and deleting the cache is not atomic.
      // Keep caching/installing usable but retain all builds rather than risk an unsafe prune.
      return;
    }

    await this.coordinator.run(() => this.pruneLocked(livePages));
  }

  private async pruneLocked(livePages: readonly (string | null)[]): Promise<void> {
    if (this.coordinator === null) {
      return;
    }

    const activations = await this.activations();
    // An older worker can still finish an event after its replacement activates. The durable
    // activation order, read under the shared lock, identifies the actual current/previous.
    const current = latestCache(activations) ?? this.name;
    const previous = previousCache(current, activations);
    const obsolete = obsoleteCaches(current, previous, activations, livePages);

    await Promise.all(obsolete.map((name) => this.storage.delete(name)));

    if (obsolete.length > 0) {
      this.previousUrls = null;
    }
  }

  private async retainedUrls(): Promise<ReadonlySet<string>> {
    const urls = new Set(this.currentUrls);

    for (const { name, activation } of await this.activations()) {
      if (name === this.name || activation === null) {
        continue;
      }

      const cache = await this.storage.open(name);

      for (const request of await cache.keys()) {
        const url = new URL(request.url);

        // Only retained builds' hashed chunks become precached requests. The activation
        // record, offline HTML, and arbitrary cache entries cannot extend the fetch policy.
        if (url.origin === this.origin && isSpaAsset(url.pathname, this.assetPrefix)) {
          urls.add(url.href);
        }
      }
    }

    return urls;
  }

  async response(request: Request): Promise<Response | null> {
    this.previousUrls ??= this.retainedUrls();

    const policy = requestPolicy(request, this.origin, await this.previousUrls);

    if (policy === "network") {
      return null;
    }

    if (policy === "navigation") {
      try {
        return await this.fetch(request);
      } catch (error) {
        const offline = await this.storage.match(new URL(this.build.offline, this.origin).href, {
          cacheName: this.name,
          ignoreVary: true,
        });

        if (offline === undefined) {
          throw error;
        }

        return offline;
      }
    }

    // These are public, fingerprinted assets (or the fixed offline page), so Origin and encoding
    // Vary headers cannot change their content. Module fetches and install fetches send different
    // Origin headers; compare the exact URL rather than missing the precache on that difference.
    const current = await this.storage.match(request, { cacheName: this.name, ignoreVary: true });

    if (current !== undefined) {
      return current;
    }

    if (policy === "classic-asset") {
      const classic = await this.storage.match(request, {
        cacheName: "smartfire-static-v1",
        ignoreVary: true,
      });

      if (classic !== undefined) {
        return classic;
      }
    }

    if (policy === "precache" || policy === "classic-asset") {
      for (const name of await this.storage.keys()) {
        if (name === this.name || !name.startsWith(CACHE_PREFIX)) {
          continue;
        }

        const previous = await this.storage.match(request, { cacheName: name, ignoreVary: true });

        if (previous !== undefined) {
          return previous;
        }
      }
    }

    const response = await this.fetch(request);

    if (canCache(response)) {
      const cache = await this.storage.open(this.name);

      await cache.put(request, response.clone());
    }

    return response;
  }
}
