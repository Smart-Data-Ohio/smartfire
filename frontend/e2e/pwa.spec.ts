import { type BrowserContext, expect, type Page, test, type Worker } from "@playwright/test";

declare const self: ServiceWorkerGlobalScope & {
  readonly smartfireBuild: { readonly version: string };
};

declare const SmartfireWorker: { openNotification(path: string): Promise<WindowClient | null> };

const workerPath = "/app/service-worker.js";

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
  await context.request.post(`/__pwa/version?value=${version}`);

  const installed = context.waitForEvent("serviceworker");

  await page.evaluate(async () => {
    const registration = await navigator.serviceWorker.getRegistration("/");

    if (registration === undefined) {
      throw new Error("Missing root service-worker registration");
    }

    const found = new Promise<ServiceWorker>((resolve, reject) => {
      registration.addEventListener(
        "updatefound",
        () => {
          const installing = registration.installing;

          if (installing === null) {
            reject(new Error("Update has no installing worker"));
          } else {
            resolve(installing);
          }
        },
        { once: true },
      );
    });

    await registration.update();

    const installing = await found;

    if (installing.state !== "activated") {
      await new Promise<void>((resolve, reject) => {
        installing.addEventListener("statechange", () => {
          if (installing.state === "activated") {
            resolve();
          } else if (installing.state === "redundant") {
            reject(new Error("Updated worker did not install"));
          }
        });
      });
    }
  });

  const worker = await installed;

  await expect.poll(() => worker.evaluate(() => self.smartfireBuild.version)).toBe(version);

  return worker;
}

test.beforeEach(async ({ request }) => {
  await request.post("/__pwa/version?value=");
});

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

test("the production worker opens a classic push permalink on its SPA screen", async ({
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

  await expect(page).toHaveURL(/\/app\/r\/12\/m\/34\?highlight=1$/);
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

test("updates activate B then C, preserve one previous build, and keep serving pages", async ({
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
    .toEqual([`${cachePrefix}pwa-e2e-b`, `${cachePrefix}pwa-e2e-c`]);
  await page.goto("/app/_kitchen-sink");
  await expect(page.getByRole("heading", { name: "Smartfire design system" })).toBeVisible();
});

test("classic and SPA scripts replace one root registration in both directions", async ({
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
