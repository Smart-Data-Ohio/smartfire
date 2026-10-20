import { type APIRequestContext, devices, type Page, type WebSocket } from "@playwright/test";
import type { ClientFrame } from "../../src/gen/ClientFrame.ts";
import {
  expect,
  openApp,
  PHONE_TOUCH,
  ROOM_IDS,
  scrollByWheel,
  test,
  USER_IDS,
} from "./support.ts";

const INTERVAL_MS = 26_000;

interface PageSync {
  readonly page: Page;
  readonly request: APIRequestContext;
  readonly socket: WebSocket;
  readonly heartbeats: readonly boolean[];
}

async function nextHeartbeat({
  page,
  request,
  socket,
  heartbeats,
}: PageSync): Promise<boolean | undefined> {
  const seen = heartbeats.length;

  const published = socket.waitForEvent("framereceived", (frame) =>
    String(frame.payload).includes('"type":"presence"'),
  );

  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/presence", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { userId: USER_IDS.maya, presence: "online" },
  });
  await published;
  await page.clock.runFor(INTERVAL_MS);
  await expect.poll(() => heartbeats.length).toBeGreaterThan(seen);

  return heartbeats[seen];
}

async function openGeneral(page: Page, request: APIRequestContext): Promise<PageSync> {
  const heartbeats: boolean[] = [];

  const welcomed = page
    .waitForEvent("websocket", (socket) => socket.url().includes("/api/v1/sync"))
    .then(async (socket) => {
      socket.on("framesent", ({ payload }) => {
        const frame: ClientFrame = JSON.parse(String(payload));

        if (frame.t === "hb") heartbeats.push(frame.active);
      });

      await socket.waitForEvent("framereceived", (frame) =>
        String(frame.payload).includes('"t":"welcome"'),
      );

      return socket;
    });

  await page.clock.install();
  await openApp(page, `r/${ROOM_IDS.general}`);

  const log = page.getByRole("log", { name: "Messages" });

  await expect(log).toHaveAttribute("data-placement-settled", "true");
  await expect(log).toHaveAttribute("data-scroll-settled", "true");
  await expect(log.locator(".unread-divider")).toBeInViewport();

  const sync = { page, request, socket: await welcomed, heartbeats };

  await nextHeartbeat(sync);
  expect(await nextHeartbeat(sync), "the heartbeat after an interval without input").toBe(false);

  return sync;
}

async function expectCounted(
  sync: PageSync,
  input: string,
  act: () => Promise<void>,
): Promise<void> {
  await act();
  expect.soft(await nextHeartbeat(sync), `the heartbeat after ${input}`).toBe(true);
  expect.soft(await nextHeartbeat(sync), `the heartbeat an interval after ${input}`).toBe(false);
}

test.describe("on a touch phone", () => {
  test.use({ ...PHONE_TOUCH, userAgent: devices["Pixel 7"].userAgent });

  test("a tap outside the composer marks the next heartbeat active", async ({ page, request }) => {
    const sync = await openGeneral(page, request);
    const divider = page.getByRole("log", { name: "Messages" }).locator(".unread-divider");

    await expectCounted(sync, "a tap on the messages", () => divider.tap());
  });
});

test("a click, a key press and a wheel outside the composer each mark the next heartbeat active", async ({
  page,
  request,
}) => {
  const sync = await openGeneral(page, request);
  const log = page.getByRole("log", { name: "Messages" });

  await expectCounted(sync, "a click on the messages", () =>
    log.locator(".unread-divider").click(),
  );
  await expectCounted(sync, "a key press in the messages", () => log.press("Shift"));
  await expectCounted(sync, "a wheel over the messages", () => scrollByWheel(page, log, 200));
});

test("automatic scrolling and visibility events leave the next heartbeat idle", async ({
  page,
  request,
}) => {
  const sync = await openGeneral(page, request);

  await page.getByRole("log", { name: "Messages" }).evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  expect(await nextHeartbeat(sync), "the heartbeat after automatic scrolling").toBe(false);

  await page.evaluate(() => document.dispatchEvent(new Event("visibilitychange")));
  expect(await nextHeartbeat(sync), "the heartbeat after a visibility event").toBe(false);
});

test("synthetic document and composer input leave the next heartbeat idle", async ({
  page,
  request,
}) => {
  const sync = await openGeneral(page, request);

  await page.evaluate(() => document.dispatchEvent(new Event("pointerdown", { bubbles: true })));
  expect
    .soft(await nextHeartbeat(sync), "the heartbeat after a synthetic document event")
    .toBe(false);

  await page
    .locator("textarea")
    .first()
    .evaluate((element) => {
      const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set;

      if (!setter) throw new Error("textarea has no native value setter");

      setter.call(element, "Synthetic activity");
      element.dispatchEvent(new Event("input", { bubbles: true }));
    });
  expect(await nextHeartbeat(sync), "the heartbeat after synthetic composer input").toBe(false);
});
