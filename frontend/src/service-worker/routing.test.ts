// @vitest-environment node

import { URLPattern } from "node:url";
import { describe, expect, it } from "vitest";
import { installNetworkRoute, type NetworkRoute, networkRoute } from "./routing.ts";
import type { WorkerBuild } from "./runtime.ts";

const origin = "https://smartfire.test";

const build: WorkerBuild = {
  version: "a",
  page: "/app/assets/index-aaaaaaaa.js",
  offline: "/offline.html",
  precache: ["/app/assets/index-aaaaaaaa.js", "/offline.html"],
};

function usesNetwork(route: NetworkRoute, request: Pick<Request, "url" | "method" | "mode">) {
  return !route.condition.not.or.some((condition) => {
    if (request.method !== condition.requestMethod) return false;

    return "requestMode" in condition
      ? request.mode === condition.requestMode
      : new URLPattern(condition.urlPattern).test(request.url);
  });
}

describe("service worker static network route", () => {
  const route = networkRoute(origin, build);

  it("uses sole-member not/or conditions, with GET on every worker-handled leaf", () => {
    expect(route.source).toBe("network");
    expect(Object.keys(route.condition)).toEqual(["not"]);
    expect(Object.keys(route.condition.not)).toEqual(["or"]);
    expect(route.condition.not.or).toHaveLength(4);

    for (const condition of route.condition.not.or) {
      expect(condition.requestMethod).toBe("GET");
    }
  });

  it.each([
    "/api/v1/boot",
    "/activity/unread_count.json",
    "/memberships/1.json",
    "/rails/active_storage/blobs/redirect/secret/avatar.png",
    "/cable",
    "/account/settings",
    "/app/",
  ])("routes private GET %s to the network", (path) => {
    expect(usesNetwork(route, { url: `${origin}${path}`, method: "GET", mode: "cors" })).toBe(true);
  });

  it.each([
    "/assets/application-01234567.js",
    "/assets/application-01234567.css?version=a",
    "/app/assets/index-aaaaaaaa.js",
    "/app/assets/chunk-bbbbbbbb.js?version=b",
    "/offline.html",
  ])("keeps the asset GET %s with the existing worker policy", (path) => {
    expect(usesNetwork(route, { url: `${origin}${path}`, method: "GET", mode: "cors" })).toBe(
      false,
    );
  });

  it("keeps all GET navigations with the worker and passes writes and foreign assets through", () => {
    for (const path of ["/app/r/1", "/assets/application-01234567.js", "/offline.html"]) {
      const url = `${origin}${path}`;

      expect(usesNetwork(route, { url, method: "GET", mode: "navigate" })).toBe(false);
      expect(usesNetwork(route, { url, method: "POST", mode: "navigate" })).toBe(true);
      expect(usesNetwork(route, { url, method: "POST", mode: "cors" })).toBe(true);
    }

    for (const path of ["/assets/application-01234567.js", "/app/assets/index-aaaaaaaa.js"]) {
      expect(
        usesNetwork(route, {
          url: `https://elsewhere.test${path}`,
          method: "GET",
          mode: "cors",
        }),
      ).toBe(true);
    }
  });

  it("derives the SPA paths and protects additional precache entries without duplicating chunks", () => {
    const custom = networkRoute(origin, {
      ...build,
      offline: "/offline.html",
      page: "/next/assets/index-aaaaaaaa.js",
      precache: [
        ...Array.from({ length: 300 }, (_, index) => `/next/assets/chunk-${index}.js`),
        "/offline.html",
        "/shared/font-aaaaaaaa.woff2",
      ],
    });

    expect(custom.condition.not.or).toHaveLength(5);

    for (const path of [
      "/next/assets/chunk-1.js",
      "/offline.html",
      "/shared/font-aaaaaaaa.woff2",
    ]) {
      expect(usesNetwork(custom, { url: `${origin}${path}`, method: "GET", mode: "cors" })).toBe(
        false,
      );
    }
  });
});

describe("installing the optional network route", () => {
  it("calls addRoutes synchronously with its receiver and waits for registration", async () => {
    const registered = Promise.withResolvers<void>();

    const event = new (class extends Event {
      readonly routes: NetworkRoute[] = [];

      addRoutes(route: NetworkRoute): Promise<void> {
        this.routes.push(route);

        return registered.promise;
      }
    })("install");

    let finished = false;

    const installing = installNetworkRoute(event, origin, build).then(() => {
      finished = true;
    });

    expect(event.routes).toEqual([networkRoute(origin, build)]);
    await Promise.resolve();
    expect(finished).toBe(false);
    registered.resolve();
    await installing;
    expect(finished).toBe(true);
  });

  it("preserves installation when addRoutes is absent", async () => {
    await expect(installNetworkRoute(new Event("install"), origin, build)).resolves.toBeUndefined();
  });

  it.each(["throw", "reject"])(
    "preserves installation when an older router reports %s",
    async (failure) => {
      const event = new (class extends Event {
        addRoutes(): Promise<void> {
          const error = new TypeError("The not/or condition is unsupported");

          if (failure === "throw") throw error;

          return Promise.reject(error);
        }
      })("install");

      await expect(installNetworkRoute(event, origin, build)).resolves.toBeUndefined();
    },
  );
});
