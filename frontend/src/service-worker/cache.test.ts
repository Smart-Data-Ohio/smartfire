import { describe, expect, it } from "vitest";
import { cacheName, nextActivation, obsoleteCaches, previousCache } from "./cache.ts";

describe("service worker cache versions", () => {
  it("names one cache per reproducible build version", () => {
    expect(cacheName("0123abcd")).toBe("smartfire-spa-0123abcd");
    expect(cacheName("0123abcd")).toBe(cacheName("0123abcd"));
    expect(cacheName("fedc4321")).not.toBe(cacheName("0123abcd"));
  });

  it("keeps the current and immediately preceding activation and leaves other namespaces alone", () => {
    const caches = [
      { name: cacheName("a"), activation: 1 },
      { name: cacheName("b"), activation: 2 },
      { name: cacheName("c"), activation: null },
      { name: "smartfire-static-v1", activation: null },
      { name: "unrelated-cache", activation: null },
    ];

    const previous = previousCache(cacheName("c"), caches);

    expect(previous).toBe(cacheName("b"));
    expect(nextActivation(caches)).toBe(3);
    expect(
      obsoleteCaches(
        cacheName("c"),
        previous,
        caches.map((cache) => cache.name),
      ),
    ).toEqual([cacheName("a")]);
  });

  it("ignores failed installs and handles same-version reinstall and downgrade", () => {
    const caches = [
      { name: cacheName("a"), activation: 4 },
      { name: cacheName("b"), activation: 3 },
      { name: cacheName("failed"), activation: null },
    ];

    expect(previousCache(cacheName("a"), caches)).toBe(cacheName("b"));
    expect(previousCache(cacheName("b"), caches)).toBe(cacheName("a"));
    expect(previousCache(cacheName("first"), [])).toBeNull();
    expect(
      obsoleteCaches(
        cacheName("b"),
        cacheName("a"),
        caches.map((cache) => cache.name),
      ),
    ).toEqual([cacheName("failed")]);
  });
});
