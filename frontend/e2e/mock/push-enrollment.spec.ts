import type { Page } from "@playwright/test";
import { refused } from "../../mock/http.ts";
import { parseJson, stringField } from "../../mock/json.ts";
import { expect, test } from "./support.ts";

declare global {
  interface Window {
    __smartfirePushBrowser?: {
      permissionRequests: number;
      registrations: number;
      subscribes: number;
      unsubscribes: number;
      rootScope: string | null;
      workerUrl: string | null;
      keyBytes: number[];
    };
  }
}

/** Browser permission/push transport are external services; the app and mock HTTP API run real. */
async function browserPush(page: Page, permission: "granted" | "denied") {
  await page.addInitScript(
    ({ permission }) => {
      const state: NonNullable<Window["__smartfirePushBrowser"]> = {
        permissionRequests: 0,
        registrations: 0,
        subscribes: 0,
        unsubscribes: 0,
        rootScope: null,
        workerUrl: null,
        keyBytes: [],
      };

      let heldPermission: NotificationPermission = "default";
      let subscribed = false;

      const subscription = {
        endpoint: "https://fcm.googleapis.com/fcm/send/mock-enrollment",
        toJSON: () => ({ keys: { p256dh: "p256", auth: "auth" } }),
        unsubscribe: async () => {
          state.unsubscribes += 1;
          subscribed = false;

          return true;
        },
      };

      const registration = {
        scope: `${location.origin}/`,
        active: Object.assign(new EventTarget(), {
          scriptURL: `${location.origin}/service-worker.js`,
          state: "activated",
        }),
        installing: null,
        waiting: null,
        pushManager: {
          getSubscription: async () => (subscribed ? subscription : null),
          subscribe: async (options: PushSubscriptionOptionsInit) => {
            if (!options.userVisibleOnly || !(options.applicationServerKey instanceof Uint8Array)) {
              throw new Error("Expected a visible subscription with typed VAPID bytes");
            }

            state.keyBytes = [...options.applicationServerKey];
            state.subscribes += 1;
            subscribed = true;

            return subscription;
          },
        },
      };

      Object.defineProperties(Notification, {
        permission: { configurable: true, get: () => heldPermission },
        requestPermission: {
          configurable: true,
          value: async () => {
            state.permissionRequests += 1;
            heldPermission = permission;

            return permission;
          },
        },
      });
      Object.defineProperties(navigator.serviceWorker, {
        getRegistration: { configurable: true, value: async () => registration },
        register: {
          configurable: true,
          value: async (url: string, options: RegistrationOptions) => {
            state.registrations += 1;
            state.rootScope = options.scope ?? null;
            state.workerUrl = url;

            return registration;
          },
        },
      });
      window.__smartfirePushBrowser = state;
    },
    { permission },
  );
}

async function openDevices(page: Page) {
  await page.goto("/app/settings/devices");
  await expect(page.getByRole("heading", { level: 1, name: "Push devices" })).toBeVisible();
  await page.waitForFunction(() => window.__smartfirePushEnrollment !== undefined);
  await expect(page.locator(".settings-list-row")).toHaveCount(2);
}

test("the explicit enrollment hook adds this browser, deduplicates, and removes its local subscription", async ({
  page,
}) => {
  const keyRequest = Promise.withResolvers<void>();

  await page.route("**/api/v1/settings/push_subscriptions/key", async (route) => {
    await keyRequest.promise;
    await route.continue();
  });
  await browserPush(page, "granted");
  await openDevices(page);

  expect(await page.evaluate(() => window.__smartfirePushBrowser?.permissionRequests)).toBe(0);
  expect(await page.evaluate(() => window.__smartfirePushBrowser?.registrations)).toBe(0);
  const enrolling = page.evaluate(() => window.__smartfirePushEnrollment?.enable());

  await expect.poll(() => page.evaluate(() => window.__smartfirePushEnrollment?.busy)).toBe(true);
  expect(await page.evaluate(() => window.__smartfirePushEnrollment?.enable())).toEqual({
    kind: "failed",
    message: "Enrollment is already in progress.",
  });
  keyRequest.resolve();
  expect(await enrolling).toMatchObject({ kind: "enabled" });
  await expect(page.locator(".settings-list-row")).toHaveCount(3);
  expect(await page.evaluate(() => ({ ...window.__smartfirePushEnrollment }))).toMatchObject({
    permission: "granted",
    subscribed: true,
    busy: false,
  });
  expect(await page.evaluate(() => ({ ...window.__smartfirePushBrowser }))).toMatchObject({
    permissionRequests: 1,
    registrations: 1,
    subscribes: 1,
    rootScope: "/",
    workerUrl: "/service-worker.js",
  });
  expect(await page.evaluate(() => window.__smartfirePushBrowser?.keyBytes.length)).toBe(65);

  expect(await page.evaluate(() => window.__smartfirePushEnrollment?.enable())).toMatchObject({
    kind: "enabled",
  });
  await expect(page.locator(".settings-list-row")).toHaveCount(3);
  expect(await page.evaluate(() => window.__smartfirePushBrowser?.subscribes)).toBe(1);

  const local = page.locator(".settings-list-row").filter({ hasText: "mock-enrollment" });

  await local.getByRole("button", { name: /^Remove / }).click();
  await expect(page.locator(".settings-list-row")).toHaveCount(2);
  await expect.poll(() => page.evaluate(() => window.__smartfirePushBrowser?.unsubscribes)).toBe(1);
  await expect
    .poll(() => page.evaluate(() => window.__smartfirePushEnrollment?.subscribed))
    .toBe(false);
});

test("rotated-key rows keep the local transport until the final endpoint row is removed", async ({
  page,
}) => {
  await browserPush(page, "granted");
  await openDevices(page);
  expect(await page.evaluate(() => window.__smartfirePushEnrollment?.enable())).toMatchObject({
    kind: "enabled",
  });

  const state = parseJson(await (await page.request.get("/__mock/state")).text());
  const csrfToken = stringField(state, "csrfToken");

  if (csrfToken === null) throw new Error("The mock did not provide its CSRF token.");

  const rotated = await page.request.post("/api/v1/settings/push_subscriptions", {
    headers: { "X-CSRF-Token": csrfToken },
    data: {
      endpoint: "https://fcm.googleapis.com/fcm/send/mock-enrollment",
      p256dhKey: "p256-rotated",
      authKey: "auth-rotated",
    },
  });

  expect(rotated.status()).toBe(200);
  expect(await page.evaluate(() => window.__smartfirePushEnrollment?.enable())).toMatchObject({
    kind: "enabled",
  });
  await expect(page.locator(".settings-list-row")).toHaveCount(4);

  const matching = page.locator(".settings-list-row").filter({ hasText: "mock-enrollment" });

  await matching
    .first()
    .getByRole("button", { name: /^Remove / })
    .click();
  await expect(page.locator(".settings-list-row")).toHaveCount(3);
  expect(await page.evaluate(() => window.__smartfirePushBrowser?.unsubscribes)).toBe(0);
  expect(await page.evaluate(() => window.__smartfirePushEnrollment?.subscribed)).toBe(true);

  await matching.getByRole("button", { name: /^Remove / }).click();
  await expect(page.locator(".settings-list-row")).toHaveCount(2);
  await expect.poll(() => page.evaluate(() => window.__smartfirePushBrowser?.unsubscribes)).toBe(1);
  await expect
    .poll(() => page.evaluate(() => window.__smartfirePushEnrollment?.subscribed))
    .toBe(false);
});

test("denied permission does not register or add a device", async ({ page }) => {
  await browserPush(page, "denied");
  await openDevices(page);

  expect(await page.evaluate(() => window.__smartfirePushEnrollment?.enable())).toEqual({
    kind: "denied",
    permission: "denied",
  });
  await expect(page.locator(".settings-list-row")).toHaveCount(2);
  await expect
    .poll(() => page.evaluate(() => window.__smartfirePushEnrollment?.permission))
    .toBe("denied");
  expect(await page.evaluate(() => window.__smartfirePushBrowser?.registrations)).toBe(0);
});

test("server refusal returns a failed outcome and keeps the device list", async ({ page }) => {
  await browserPush(page, "granted");
  await page.route("**/api/v1/settings/push_subscriptions", async (route) => {
    if (route.request().method() !== "POST") {
      await route.continue();

      return;
    }

    await route.fulfill({
      status: 422,
      contentType: "application/json",
      body: JSON.stringify({ error: refused("Push endpoint rejected").error }),
    });
  });
  await openDevices(page);

  expect(await page.evaluate(() => window.__smartfirePushEnrollment?.enable())).toEqual({
    kind: "failed",
    message: "Push endpoint rejected",
  });
  await expect(page.locator(".settings-list-row")).toHaveCount(2);
  await expect.poll(() => page.evaluate(() => window.__smartfirePushEnrollment?.busy)).toBe(false);
});

test("unsupported browsers load devices without a permission prompt", async ({ page }) => {
  await page.addInitScript(() => {
    Reflect.deleteProperty(window, "PushManager");
  });
  await openDevices(page);

  expect(await page.evaluate(() => window.__smartfirePushEnrollment?.enable())).toEqual({
    kind: "unsupported",
  });
  expect(await page.evaluate(() => ({ ...window.__smartfirePushEnrollment }))).toMatchObject({
    permission: "unsupported",
    subscribed: false,
    busy: false,
  });
  await expect(page.locator(".settings-list-row")).toHaveCount(2);
});
