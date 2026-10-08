import { describe, expect, it } from "vitest";
import { canCache, requestPolicy } from "./policy.ts";

const origin = "https://smartfire.test";

const precached = new Set([`${origin}/app/assets/index-abcd1234.js`, `${origin}/offline.html`]);

function policy(path: string, method = "GET", mode: RequestMode = "cors") {
  return requestPolicy({ url: new URL(path, origin).href, method, mode }, origin, precached);
}

describe("service worker request policy", () => {
  it("cache-first serves only the exact precache and classic fingerprinted assets", () => {
    expect(policy("/app/assets/index-abcd1234.js")).toBe("precache");
    expect(policy("/offline.html")).toBe("precache");
    expect(policy("/assets/application-1234567890abcdef.css")).toBe("classic-asset");
    expect(policy("/assets/application.css")).toBe("network");
    expect(policy("/app/assets/not-in-precache-12345678.js")).toBe("network");
    expect(policy("/app/assets/index-abcd1234.js?different=1")).toBe("network");
  });

  it("always sends navigations to the network, including precached offline HTML", () => {
    expect(policy("/app/r/12", "GET", "navigate")).toBe("navigation");
    expect(policy("/offline.html", "GET", "navigate")).toBe("navigation");
    expect(policy("/account/settings", "GET", "navigate")).toBe("navigation");
  });

  it("passes foreign origins, writes, shells and private requests through", () => {
    expect(policy("https://elsewhere.test/assets/application-12345678.js")).toBe("network");
    expect(policy("/app/assets/index-abcd1234.js", "POST")).toBe("network");

    for (const path of [
      "/api/v1/boot",
      "/cable",
      "/rails/active_storage/blobs/1/photo",
      "/app/",
      "/app/index.html",
      "/account/settings",
      "/session/transfers/secret",
      "/qr_code",
      "/uploads",
      "/app/offline.html",
    ]) {
      expect(policy(path)).toBe("network");
    }
  });

  it("never stores no-store responses, including mixed-case directives", () => {
    for (const control of ["no-store", "private, no-store", "NO-STORE, max-age=0"]) {
      expect(canCache(new Response("asset", { headers: { "Cache-Control": control } }))).toBe(
        false,
      );
    }

    expect(canCache(new Response("asset", { headers: { "Cache-Control": "no-cache" } }))).toBe(
      true,
    );
    expect(canCache(new Response("missing", { status: 404 }))).toBe(false);
  });

  it("passes responses the Cache API cannot store through without a cache write", () => {
    expect(canCache(new Response("partial asset", { status: 206 }))).toBe(false);
    expect(canCache(new Response("asset", { headers: { Vary: "Origin, *" } }))).toBe(false);
  });
});
