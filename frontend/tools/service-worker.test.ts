import { describe, expect, it } from "vitest";
import { assetHash, buildVersion, contentHash, precacheFiles } from "./service-worker.ts";

describe("service worker build injection", () => {
  it("uses every emitted content-hashed asset and stable offline.html, excluding the shell", () => {
    expect(
      precacheFiles(
        [
          "assets/index-abcd1234.js",
          "assets/styles-12345678.css",
          "assets/font-abCD1234.woff2",
          "assets/service-worker-12345678.js",
          "assets/offline-page-87654321.html",
          "assets/unhashed.js",
          "index.html",
          "offline.html",
          "service-worker.js",
          ".vite/manifest.json",
        ],
        "/app/",
      ),
    ).toEqual([
      "/app/assets/font-abCD1234.woff2",
      "/app/assets/index-abcd1234.js",
      "/app/assets/offline-page-87654321.html",
      "/app/assets/service-worker-12345678.js",
      "/app/assets/styles-12345678.css",
      "/offline.html",
    ]);
  });

  it("derives reproducible versions from sorted names, including worker and offline-page changes", () => {
    expect(buildVersion(["b", "a"])).toBe(buildVersion(["a", "b"]));
    expect(buildVersion(["a", "b"])).not.toBe(buildVersion(["a", "c"]));
    expect(contentHash("worker A")).not.toBe(contentHash("worker B"));
    expect(contentHash("offline A")).not.toBe(contentHash("offline B"));
    expect(assetHash("runtime")).toMatch(/^[A-Za-z0-9_-]{8}$/);
  });
});
