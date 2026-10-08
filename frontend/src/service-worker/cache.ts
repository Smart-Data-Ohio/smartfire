export const CACHE_PREFIX = "smartfire-spa-";

export function cacheName(version: string): string {
  return `${CACHE_PREFIX}${version}`;
}

export interface CacheActivation {
  readonly name: string;
  /** An installed cache has no activation until its worker actually takes control. */
  readonly activation: number | null;
  /** The entry URL serialized from the bundle this cache belongs to. */
  readonly page: string | null;
  /** An installation can pin a previously activated cache during rollback. */
  readonly installing: boolean;
}

/** The latest actual activation, including when an update failed or was downgraded. */
export function latestCache(caches: readonly CacheActivation[]): string | null {
  let latest: CacheActivation | null = null;

  for (const cache of caches) {
    if (
      cache.name.startsWith(CACHE_PREFIX) &&
      cache.activation !== null &&
      (latest === null || cache.activation > (latest.activation ?? 0))
    ) {
      latest = cache;
    }
  }

  return latest?.name ?? null;
}

export function previousCache(current: string, caches: readonly CacheActivation[]): string | null {
  return latestCache(caches.filter((cache) => cache.name !== current));
}

export function obsoleteCaches(
  current: string,
  previous: string | null,
  caches: readonly CacheActivation[],
  livePages: readonly (string | null)[],
): string[] {
  // A page that has not answered (including an older app or a classic page) might still need
  // any retained build. An unknown fingerprint is also insufficient evidence to delete one.
  if (livePages.some((page) => page === null || !caches.some((cache) => cache.page === page))) {
    return [];
  }

  // Worker-only or offline-only updates can share an entry. Keep every matching cache rather
  // than attribute a page to a newer activation it never loaded.
  // An unactivated cache may still be populated by an installing worker. Keep it until its
  // activation marker proves it is eligible for pruning; failed-install orphans stay retained.
  return caches.flatMap((cache) =>
    cache.name.startsWith(CACHE_PREFIX) &&
    cache.activation !== null &&
    !cache.installing &&
    cache.name !== current &&
    cache.name !== previous &&
    !livePages.some((page) => cache.page === page)
      ? [cache.name]
      : [],
  );
}

export function nextActivation(caches: readonly CacheActivation[]): number {
  return Math.max(0, ...caches.map((cache) => cache.activation ?? 0)) + 1;
}
