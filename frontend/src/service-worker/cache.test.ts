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
      { name: cacheName("a"), activation: 1, page: "/a.js", installing: false },
      { name: cacheName("b"), activation: 2, page: "/b.js", installing: false },
      { name: cacheName("c"), activation: null, page: "/c.js", installing: false },
      { name: "smartfire-static-v1", activation: null, page: null, installing: false },
      { name: "unrelated-cache", activation: null, page: null, installing: false },
    ];

    const previous = previousCache(cacheName("c"), caches);

    expect(previous).toBe(cacheName("b"));
    expect(nextActivation(caches)).toBe(3);
    expect(obsoleteCaches(cacheName("c"), previous, caches, [])).toEqual([cacheName("a")]);
  });

  it("ignores pending installs as predecessors and handles same-version reinstall and downgrade", () => {
    const caches = [
      { name: cacheName("a"), activation: 4, page: "/a.js", installing: false },
      { name: cacheName("b"), activation: 3, page: "/b.js", installing: false },
      { name: cacheName("failed"), activation: null, page: "/failed.js", installing: false },
    ];

    expect(previousCache(cacheName("a"), caches)).toBe(cacheName("b"));
    expect(previousCache(cacheName("b"), caches)).toBe(cacheName("a"));
    expect(previousCache(cacheName("first"), [])).toBeNull();
    expect(obsoleteCaches(cacheName("b"), cacheName("a"), caches, [])).toEqual([]);
  });

  it("never lets a running worker prune a cache another worker may still be installing", () => {
    const caches = [
      { name: cacheName("retired"), activation: 1, page: "/retired.js", installing: false },
      { name: cacheName("previous"), activation: 2, page: "/previous.js", installing: false },
      { name: cacheName("a"), activation: 3, page: "/a.js", installing: false },
      { name: cacheName("installing-b"), activation: null, page: null, installing: false },
    ];

    expect(obsoleteCaches(cacheName("a"), cacheName("previous"), caches, ["/a.js"])).toEqual([
      cacheName("retired"),
    ]);
    expect(obsoleteCaches(cacheName("a"), cacheName("previous"), caches, [])).toEqual([
      cacheName("retired"),
    ]);
  });

  it("keeps an installation pin even when a rollback cache has an older activation", () => {
    const caches = [
      { name: cacheName("a"), activation: 1, page: "/a.js", installing: true },
      { name: cacheName("c"), activation: 2, page: "/c.js", installing: false },
      { name: cacheName("d"), activation: 3, page: "/d.js", installing: false },
    ];

    expect(obsoleteCaches(cacheName("d"), cacheName("c"), caches, [])).toEqual([]);
    expect(
      obsoleteCaches(
        cacheName("d"),
        cacheName("c"),
        caches.map((cache) => ({ ...cache, installing: false })),
        [],
      ),
    ).toEqual([cacheName("a")]);
  });

  it("retains every live page's build through later activations and prunes after reload or close", () => {
    const caches = [
      { name: cacheName("a"), activation: 1, page: "/a.js", installing: false },
      { name: cacheName("b"), activation: 2, page: "/b.js", installing: false },
      { name: cacheName("c"), activation: 3, page: "/c.js", installing: false },
      { name: cacheName("d"), activation: 4, page: "/d.js", installing: false },
    ];

    expect(obsoleteCaches(cacheName("d"), cacheName("c"), caches, ["/a.js", "/b.js"])).toEqual([]);
    expect(obsoleteCaches(cacheName("d"), cacheName("c"), caches, ["/b.js", "/d.js"])).toEqual([
      cacheName("a"),
    ]);
    expect(obsoleteCaches(cacheName("d"), cacheName("c"), caches, [])).toEqual([
      cacheName("a"),
      cacheName("b"),
    ]);
  });

  it("retains caches conservatively for unknown live clients, unrecognized builds and old markers", () => {
    const caches = [
      { name: cacheName("old"), activation: 1, page: null, installing: false },
      { name: cacheName("b"), activation: 2, page: "/b.js", installing: false },
      { name: cacheName("c"), activation: 3, page: "/c.js", installing: false },
    ];

    expect(obsoleteCaches(cacheName("c"), cacheName("b"), caches, [null])).toEqual([]);
    expect(obsoleteCaches(cacheName("c"), cacheName("b"), caches, ["/unknown.js"])).toEqual([]);
    expect(obsoleteCaches(cacheName("c"), cacheName("b"), caches, [])).toEqual([cacheName("old")]);
  });
});
