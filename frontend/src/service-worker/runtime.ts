import {
  CACHE_PREFIX,
  type CacheActivation,
  cacheName,
  nextActivation,
  obsoleteCaches,
  previousCache,
} from "./cache.ts";
import { canCache, isSpaAsset, requestPolicy } from "./policy.ts";

export interface WorkerBuild {
  readonly version: string;
  readonly precache: readonly string[];
  readonly offline: string;
}

/** Browser APIs stay at this boundary; request selection and cache retention are pure modules. */
export class WorkerCache {
  readonly name: string;
  private readonly origin: string;
  private readonly storage: CacheStorage;
  private readonly fetch: typeof fetch;
  private readonly build: WorkerBuild;
  private readonly marker: string;
  private readonly currentUrls: ReadonlySet<string>;
  private readonly assetPrefix: string;
  private previousUrls: Promise<ReadonlySet<string>> | null = null;

  constructor(build: WorkerBuild, origin: string, storage: CacheStorage, network: typeof fetch) {
    this.build = build;
    this.origin = origin;
    this.storage = storage;
    this.fetch = (...args) => network(...args);
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
    const cache = await this.storage.open(this.name);

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

  private async activations(): Promise<CacheActivation[]> {
    const activations: CacheActivation[] = [];

    for (const name of await this.storage.keys()) {
      if (!name.startsWith(CACHE_PREFIX)) {
        continue;
      }

      const marker = await this.storage.match(this.marker, { cacheName: name });
      const activation = marker === undefined ? null : Number(await marker.text());

      activations.push({
        name,
        activation: activation !== null && Number.isSafeInteger(activation) ? activation : null,
      });
    }

    return activations;
  }

  async activate(): Promise<void> {
    const activations = await this.activations();
    const previous = previousCache(this.name, activations);
    const cache = await this.storage.open(this.name);

    // This internal record is written only on activation, so failed installations never count.
    await cache.put(this.marker, new Response(String(nextActivation(activations))));
    await Promise.all(
      obsoleteCaches(this.name, previous, await this.storage.keys()).map((name) =>
        this.storage.delete(name),
      ),
    );
    this.previousUrls = null;
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

        // Only the previous build's hashed chunks become precached requests. The activation
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
