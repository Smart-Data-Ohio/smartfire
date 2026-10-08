import { type BrowserContext, expect, type Page, test, type Worker } from "@playwright/test";
import { parseJson } from "../mock/json.ts";
import { ROOM_IDS } from "../mock/seed.ts";
import { createMockServer } from "../mock/server.ts";

declare const self: ServiceWorkerGlobalScope & {
  readonly smartfireBuild: {
    readonly version: string;
    readonly page: string;
    readonly precache: readonly string[];
  };
};

declare const SmartfireWorker: { openNotification(path: string): Promise<WindowClient | null> };

const workerPath = "/service-worker.js";

const cachePrefix = "smartfire-spa-";

async function register(page: Page, context: BrowserContext): Promise<Worker> {
  await page.goto("/app/_kitchen-sink");

  const installed = context.waitForEvent("serviceworker");

  await page.evaluate(async (url) => {
    await navigator.serviceWorker.register(url, { scope: "/", updateViaCache: "none" });
    await navigator.serviceWorker.ready;

    if (navigator.serviceWorker.controller === null) {
      await new Promise<void>((resolve) => {
        navigator.serviceWorker.addEventListener("controllerchange", () => resolve(), {
          once: true,
        });
      });
    }
  }, workerPath);

  return installed;
}

async function cacheNames(page: Page): Promise<string[]> {
  return page.evaluate(async () =>
    (await caches.keys()).filter((name) => name.startsWith("smartfire-spa-")).sort(),
  );
}

async function update(page: Page, context: BrowserContext, version: string): Promise<Worker> {
  let existing: Worker | undefined;

  for (const worker of context.serviceWorkers()) {
    if (
      new URL(worker.url()).pathname === workerPath &&
      (await worker.evaluate(() => self.smartfireBuild.version)) === version
    ) {
      existing = worker;
      break;
    }
  }

  // Subscribe before changing the server: the page's own update can win this race.
  const installed =
    existing === undefined
      ? context.waitForEvent(
          "serviceworker",
          async (worker) =>
            new URL(worker.url()).pathname === workerPath &&
            (await worker.evaluate(() => self.smartfireBuild.version)) === version,
        )
      : Promise.resolve(existing);

  await context.request.post(`/__pwa/version?value=${version}`);

  await page.evaluate(async () => {
    const registration = await navigator.serviceWorker.getRegistration("/");

    if (registration === undefined) {
      throw new Error("Missing root service-worker registration");
    }

    await registration.update();

    const installing = registration.installing ?? registration.waiting ?? registration.active;

    if (installing === null) {
      throw new Error("Update has no worker");
    }

    if (installing.state !== "activated") {
      await new Promise<void>((resolve, reject) => {
        const changed = () => {
          if (installing.state === "activated") {
            installing.removeEventListener("statechange", changed);
            resolve();
          } else if (installing.state === "redundant") {
            installing.removeEventListener("statechange", changed);
            reject(new Error("Updated worker did not install"));
          }
        };

        installing.addEventListener("statechange", changed);
        changed();
      });
    }

    if (registration.active !== installing || navigator.serviceWorker.controller !== installing) {
      throw new Error("Updated worker is not the active root controller");
    }
  });

  const worker = await installed;

  expect(await worker.evaluate(() => self.smartfireBuild.version)).toBe(version);

  const modules = await worker.evaluate(() =>
    self.smartfireBuild.precache.filter((path) =>
      new URL(path, self.location.href).pathname.endsWith(".js"),
    ),
  );

  expect(modules.length).toBeGreaterThan(0);

  for (const path of modules) {
    expect(new URL(path, page.url()).searchParams.get("pwa")).toBe(version);
  }

  // The same script URL serves every revision. Check the reporting worker's identity too.
  const confirmation = await page.evaluateHandle(async (expected) => {
    const registration = await navigator.serviceWorker.getRegistration("/");

    const confirmed = new Promise<void>((resolve, reject) => {
      const received = (event: MessageEvent<unknown>) => {
        if (event.data !== `pwa-test:${expected}`) {
          return;
        }

        navigator.serviceWorker.removeEventListener("message", received);

        if (
          registration?.active?.state === "activated" &&
          event.source === registration.active &&
          navigator.serviceWorker.controller === registration.active
        ) {
          resolve();
        } else {
          reject(new Error("Version-matching worker is not the active root controller"));
        }
      };

      navigator.serviceWorker.addEventListener("message", received);
    });

    return { confirmed };
  }, version);

  try {
    await worker.evaluate(async () => {
      for (const client of await self.clients.matchAll({
        type: "window",
        includeUncontrolled: true,
      })) {
        client.postMessage(`pwa-test:${self.smartfireBuild.version}`);
      }
    });
    await confirmation.evaluate(({ confirmed }) => confirmed);
  } finally {
    await confirmation.dispose();
  }

  return worker;
}

test.beforeEach(async ({ request }) => {
  await request.post("/__pwa/version?value=");
  await request.post("/__pwa/retire");
});

/** Exercise real production chunks with the existing wire-contract mock, without a dev bundle. */
async function mockApi(page: Page): Promise<() => void> {
  const mock = createMockServer();

  await page.route("**/api/v1/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const text = request.postData();

    const result = await mock.handle({
      method: request.method(),
      path: url.pathname,
      query: url.searchParams,
      body: text === null ? undefined : parseJson(text),
      headers: await request.allHeaders(),
    });

    await route.fulfill({
      status: result.status,
      contentType: "application/json",
      headers: { "Cache-Control": "no-store" },
      body: result.status === 204 ? "" : JSON.stringify(result.json),
    });
  });

  return () => mock.dispose();
}

async function openPoll(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Attach and more" }).click();
  await page.getByRole("menuitem", { name: "Create a poll" }).click();
}

async function pollChunk(page: Page, cache: string): Promise<string> {
  return page.evaluate(async (name) => {
    const stored = await caches.open(name);

    const request = (await stored.keys()).find((entry) =>
      /\/create-poll-dialog-[\w-]+\.js$/.test(new URL(entry.url).pathname),
    );

    if (request === undefined) {
      throw new Error("The actual create-poll lazy chunk was not precached");
    }

    return new URL(request.url).pathname;
  }, cache);
}

test("offline navigation shows the designed offline page and recovers online", async ({
  page,
  context,
}) => {
  await register(page, context);
  await context.setOffline(true);
  await page.goto("/app/r/12");
  await expect(page.getByRole("heading", { name: "You're offline" })).toBeVisible();

  await context.setOffline(false);
  // The offline page reloads itself on the online event. A second reload races that navigation.
  await expect(page.getByRole("navigation", { name: "Destinations" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "You're offline" })).toHaveCount(0);
});

test("the production worker leaves classic push permalinks to the server aliases", async ({
  page,
  context,
}) => {
  const worker = await register(page, context);

  // Evaluate the actual exported click handler in the worker, avoiding synthetic notification
  // events (whose waitUntil lifecycle is not a browser notification's) or a shipped message hook.
  await worker.evaluate(async () => {
    try {
      await SmartfireWorker.openNotification("/rooms/12/@34?highlight=1");
    } catch (error) {
      if (error instanceof DOMException && error.name === "InvalidAccessError") {
        // evaluate has no real notification's user-activation privilege. Navigation succeeded;
        // Chrome refuses only the subsequent focus, which a real click is permitted to perform.
        return;
      }

      throw error;
    }
  });

  await expect(page).toHaveURL(/\/rooms\/12\/@34\?highlight=1$/);
});

test("no-store classic assets are fetched each time and never enter either cache", async ({
  page,
  context,
}) => {
  await register(page, context);

  const results = await page.evaluate(async () => {
    const path = "/assets/pwa-no-store-0123456789abcdef.txt";
    const first = await fetch(path);
    const second = await fetch(path);

    return {
      first: await first.text(),
      second: await second.text(),
      cached: (await caches.match(path)) !== undefined,
    };
  });

  expect(Number(results.second)).toBe(Number(results.first) + 1);
  expect(results.cached).toBe(false);
});

test("updates retain live A through B and C, then prune its closed page", async ({
  page,
  context,
}) => {
  const a = await register(page, context);
  const versionA = await a.evaluate(() => self.smartfireBuild.version);
  const cacheA = `${cachePrefix}${versionA}`;
  const oldChunk = "/app/assets/retired-chunk-01234567.js";
  const oldClassic = "/assets/retired-classic-01234567.js";

  await expect.poll(() => cacheNames(page)).toEqual([cacheA]);
  await page.evaluate(
    async ({ name, path, classicPath }) => {
      const cache = await caches.open(name);

      await cache.put(path, new Response("previous build's lazy chunk"));
      await cache.put(classicPath, new Response("previous build's classic asset"));
    },
    { name: cacheA, path: oldChunk, classicPath: oldClassic },
  );
  await update(page, context, "pwa-e2e-b");
  // An unchanged worker emits no updatefound event; accept the already active revision.
  await update(page, context, "pwa-e2e-b");
  await expect.poll(() => cacheNames(page)).toEqual([cacheA, `${cachePrefix}pwa-e2e-b`].sort());

  await context.setOffline(true);

  const previousChunk = await page.evaluate(async (path) => (await fetch(path)).text(), oldChunk);

  expect(previousChunk).toBe("previous build's lazy chunk");

  const previousClassic = await page.evaluate(
    async (path) => (await fetch(path)).text(),
    oldClassic,
  );

  expect(previousClassic).toBe("previous build's classic asset");

  await context.setOffline(false);
  await update(page, context, "pwa-e2e-c");
  await expect
    .poll(() => cacheNames(page))
    .toEqual([cacheA, `${cachePrefix}pwa-e2e-b`, `${cachePrefix}pwa-e2e-c`].sort());

  const current = await context.newPage();

  await current.goto("/app/_kitchen-sink");
  await expect(current.getByRole("heading", { name: "Smartfire design system" })).toBeVisible();
  await page.close();
  // A later real fetch/messages performs pruning; there is no retention timer.
  await current.evaluate(() => fetch("/offline.html"));
  await expect
    .poll(() => cacheNames(current))
    .toEqual([`${cachePrefix}pwa-e2e-b`, `${cachePrefix}pwa-e2e-c`]);
});

test("an A page opens its unloaded real poll chunk after B and C activate", async ({
  page,
  context,
}) => {
  const dispose = await mockApi(page);

  try {
    await page.emulateMedia({ reducedMotion: "reduce" });
    const a = await register(page, context);
    const cacheA = `${cachePrefix}${await a.evaluate(() => self.smartfireBuild.version)}`;

    await page.goto(`/app/r/${ROOM_IDS.general}`);
    await expect(page.getByRole("textbox", { name: /^Message #/ })).toBeVisible();
    const chunk = await pollChunk(page, cacheA);

    await update(page, context, "pwa-live-b");
    await update(page, context, "pwa-live-c");
    await context.request.post(`/__pwa/retire?path=${encodeURIComponent(chunk)}`);
    await page.evaluate(async (path) => {
      for (const name of await caches.keys()) {
        if (name === "smartfire-spa-pwa-live-b" || name === "smartfire-spa-pwa-live-c") {
          const stored = await caches.open(name);

          for (const request of await stored.keys()) {
            if (new URL(request.url).pathname === path) await stored.delete(request);
          }
        }
      }
    }, chunk);
    const session = await context.newCDPSession(page);

    await session.send("Network.enable");
    await session.send("Network.clearBrowserCache");
    await session.send("Network.setCacheDisabled", { cacheDisabled: true });
    await openPoll(page);
    await expect(page.getByRole("dialog", { name: "Create a poll" })).toBeVisible();
    await expect(page.locator(".app-shell")).toHaveAttribute("data-update-required", "false");
    await expect.poll(() => cacheNames(page)).toContain(cacheA);
  } finally {
    dispose();
  }
});

test("an A page keeps its unloaded chunk when a classic tab registers the shared worker", async ({
  page,
  context,
}) => {
  const dispose = await mockApi(page);

  try {
    await page.emulateMedia({ reducedMotion: "reduce" });
    const a = await register(page, context);
    const cacheA = `${cachePrefix}${await a.evaluate(() => self.smartfireBuild.version)}`;

    await page.goto(`/app/r/${ROOM_IDS.general}`);
    await expect(page.getByRole("textbox", { name: /^Message #/ })).toBeVisible();
    const chunk = await pollChunk(page, cacheA);

    // A classic tab registers the same root script and retains the SPA page's cache.
    const other = await context.newPage();

    await other.goto("/app/_kitchen-sink");
    await other.evaluate(async () => {
      await navigator.serviceWorker.register("/service-worker.js", { scope: "/" });
    });
    await expect
      .poll(() =>
        other.evaluate(async () => {
          const registration = await navigator.serviceWorker.getRegistration("/");

          return new URL(registration?.active?.scriptURL ?? location.href).pathname;
        }),
      )
      .toBe("/service-worker.js");
    await other.close();

    await context.request.post(`/__pwa/retire?path=${encodeURIComponent(chunk)}`);
    const session = await context.newCDPSession(page);

    await session.send("Network.enable");
    await session.send("Network.clearBrowserCache");
    await session.send("Network.setCacheDisabled", { cacheDisabled: true });
    await openPoll(page);
    await expect(page.getByRole("dialog", { name: "Create a poll" })).toBeVisible();
    await expect(page.locator(".app-shell")).toHaveAttribute("data-update-required", "false");
    expect(await cacheNames(page)).toContain(cacheA);
  } finally {
    dispose();
  }
});

test("a forced real lazy-chunk failure preserves the page and signals explicit update recovery", async ({
  page,
  context,
}) => {
  const dispose = await mockApi(page);
  const errors: string[] = [];

  page.on("pageerror", (error) => errors.push(error.message));

  try {
    await page.emulateMedia({ reducedMotion: "reduce" });
    const a = await register(page, context);
    const cacheA = `${cachePrefix}${await a.evaluate(() => self.smartfireBuild.version)}`;

    await page.goto(`/app/r/${ROOM_IDS.general}`);
    const composer = page.getByRole("textbox", { name: /^Message #/ });

    await expect(composer).toBeVisible();
    await composer.fill("Keep this draft while the feature is unavailable");
    const chunk = await pollChunk(page, cacheA);

    await page.evaluate(async (path) => {
      for (const name of await caches.keys()) {
        await (await caches.open(name)).delete(path);
      }
    }, chunk);
    await context.request.post(`/__pwa/retire?path=${encodeURIComponent(chunk)}`);
    const session = await context.newCDPSession(page);

    await session.send("Network.enable");
    await session.send("Network.clearBrowserCache");
    await session.send("Network.setCacheDisabled", { cacheDisabled: true });
    let navigations = 0;

    page.on("framenavigated", (frame) => {
      if (frame === page.mainFrame()) navigations += 1;
    });
    await openPoll(page);
    await expect(page.locator(".app-shell")).toHaveAttribute("data-update-required", "true");
    await expect(composer).toHaveText("Keep this draft while the feature is unavailable");
    await expect(page.getByRole("navigation", { name: "Destinations" })).toBeVisible();
    await expect(page.getByRole("dialog", { name: "Create a poll" })).toHaveCount(0);
    expect(errors).toEqual([]);
    expect(navigations).toBe(0);

    const banner = page.locator(".update-banner");

    // The server still hands out this tab's build, so the strip doesn't claim a deploy.
    await expect(banner).toHaveAttribute("data-open", "true");
    await expect(banner).toHaveAttribute("data-kind", "failed");
    await expect(banner).toContainText("Couldn't load part of Smartfire. Reload to try again.");
    await banner.getByRole("button", { name: "Reload" }).click();
    await expect.poll(() => navigations).toBe(1);
  } finally {
    dispose();
  }
});

test("a lazy-chunk failure after a deploy says Smartfire has been updated", async ({
  page,
  context,
}) => {
  const dispose = await mockApi(page);

  try {
    await page.emulateMedia({ reducedMotion: "reduce" });
    const a = await register(page, context);
    const cacheA = `${cachePrefix}${await a.evaluate(() => self.smartfireBuild.version)}`;

    await page.goto(`/app/r/${ROOM_IDS.general}`);
    await expect(page.getByRole("textbox", { name: /^Message #/ })).toBeVisible();
    const chunk = await pollChunk(page, cacheA);

    await page.evaluate(async (path) => {
      for (const name of await caches.keys()) {
        await (await caches.open(name)).delete(path);
      }
    }, chunk);
    await context.request.post(`/__pwa/retire?path=${encodeURIComponent(chunk)}`);
    // The server now names a different entry module, as it does after a real deploy.
    await context.request.post("/__pwa/version?value=deployed-b");
    const session = await context.newCDPSession(page);

    await session.send("Network.enable");
    await session.send("Network.clearBrowserCache");
    await session.send("Network.setCacheDisabled", { cacheDisabled: true });
    await openPoll(page);

    const banner = page.locator(".update-banner");

    await expect(banner).toHaveAttribute("data-kind", "updated");
    await expect(banner).toContainText(
      "Smartfire has been updated. Reload to get the latest version.",
    );
  } finally {
    dispose();
  }
});

test("legacy SPA worker aliases update the same root registration to the canonical script", async ({
  page,
  context,
}) => {
  await page.goto("/app/_kitchen-sink");
  await page.evaluate(async () => {
    await navigator.serviceWorker.register("/service-worker.js", { scope: "/" });
    await navigator.serviceWorker.ready;
  });

  const registrations = () =>
    page.evaluate(async () => {
      const registrations = await navigator.serviceWorker.getRegistrations();

      return registrations.map((registration) => ({
        scope: new URL(registration.scope).pathname,
        script: new URL(registration.active?.scriptURL ?? location.href).pathname,
      }));
    });

  await expect.poll(registrations).toEqual([{ scope: "/", script: "/service-worker.js" }]);
  await page.evaluate(async () => {
    await navigator.serviceWorker.register("/app/service-worker.js", {
      scope: "/",
      updateViaCache: "none",
    });
  });
  await expect.poll(registrations).toEqual([{ scope: "/", script: "/app/service-worker.js" }]);
  await page.evaluate(async () => {
    await navigator.serviceWorker.register("/service-worker.js", { scope: "/" });
  });
  await expect.poll(registrations).toEqual([{ scope: "/", script: "/service-worker.js" }]);
  expect(context.serviceWorkers().length).toBeGreaterThan(0);
});
