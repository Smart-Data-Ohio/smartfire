export const CACHE_PREFIX = "smartfire-spa-";

export function cacheName(version: string): string {
  return `${CACHE_PREFIX}${version}`;
}

export interface CacheActivation {
  readonly name: string;
  /** An installed cache has no activation until its worker actually takes control. */
  readonly activation: number | null;
}

/** The previous actual activation, including when an installed update failed or was downgraded. */
export function previousCache(current: string, caches: readonly CacheActivation[]): string | null {
  let previous: CacheActivation | null = null;

  for (const cache of caches) {
    if (
      cache.name !== current &&
      cache.name.startsWith(CACHE_PREFIX) &&
      cache.activation !== null &&
      (previous === null || cache.activation > (previous.activation ?? 0))
    ) {
      previous = cache;
    }
  }

  return previous?.name ?? null;
}

export function obsoleteCaches(
  current: string,
  previous: string | null,
  names: readonly string[],
): string[] {
  return names.filter(
    (name) => name.startsWith(CACHE_PREFIX) && name !== current && name !== previous,
  );
}

export function nextActivation(caches: readonly CacheActivation[]): number {
  return Math.max(0, ...caches.map((cache) => cache.activation ?? 0)) + 1;
}
