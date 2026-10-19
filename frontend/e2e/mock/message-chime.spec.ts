import type { APIRequestContext, Page } from "@playwright/test";
import { THREAD_IDS } from "../../mock/s2/seed.ts";
import { B5_HZ } from "../../src/features/huddle/chime.ts";
import { expect, openApp, postMessage, ROOM_IDS, syncWelcomed, test } from "./support.ts";

const MAYA = 2;

const JONAH = 3;

declare global {
  interface Window {
    chimes: number;
  }
}

async function spyOnChimes(page: Page, { focused = true, gestured = true } = {}): Promise<void> {
  await page.addInitScript(
    ({ windowFocused, pageGestured, note }) => {
      const start = OscillatorNode.prototype.start;

      window.chimes = 0;
      OscillatorNode.prototype.start = function (this: OscillatorNode, when?: number) {
        if (Math.abs(this.frequency.value - note) < 0.01) {
          window.chimes += 1;
        }

        start.call(this, when);
      };

      if (!windowFocused) {
        Document.prototype.hasFocus = () => false;
      }

      if (!pageGestured) {
        Object.defineProperty(navigator, "userActivation", {
          value: { hasBeenActive: false, isActive: false },
        });
      }
    },
    { windowFocused: focused, pageGestured: gestured, note: B5_HZ },
  );
}

const activated = (page: Page) => page.evaluate(() => navigator.userActivation.hasBeenActive);

async function open(page: Page, path: string, { gestured = true } = {}): Promise<void> {
  const welcomed = syncWelcomed(page);

  await openApp(page, path);
  await welcomed;
  // Browsers allow sound only after a gesture on the page.
  await page.keyboard.press("Shift");
  await expect.poll(() => activated(page)).toBe(gestured);
}

async function replyInThread(request: APIRequestContext, markdown: string): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/thread-post", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { threadId: THREAD_IDS.generalActive, userId: MAYA, markdown },
  });
}

const chimes = (page: Page) => page.evaluate(() => window.chimes);

test("a direct message in a background room chimes once", async ({ page, request }) => {
  await spyOnChimes(page);
  await open(page, `r/${ROOM_IDS.general}`);

  await postMessage(request, { roomId: ROOM_IDS.dmMaya, userId: MAYA, markdown: "Got a minute?" });

  await expect.poll(() => chimes(page)).toBe(1);
  await page.waitForTimeout(1000);
  expect(await chimes(page)).toBe(1);
});

test("the conversation you're reading in a focused tab doesn't chime, another one does", async ({
  page,
  request,
}) => {
  await spyOnChimes(page);
  await open(page, `r/${ROOM_IDS.dmMaya}`);

  await postMessage(request, { roomId: ROOM_IDS.dmMaya, userId: MAYA, markdown: "Read live" });
  await expect(page.getByText("Read live")).toBeVisible();
  await page.waitForTimeout(1000);
  expect(await chimes(page)).toBe(0);

  await postMessage(request, { roomId: ROOM_IDS.groupDm, userId: JONAH, markdown: "Over here" });
  await expect.poll(() => chimes(page)).toBe(1);
});

test("the conversation on screen chimes while the window is out of focus", async ({
  page,
  request,
}) => {
  await spyOnChimes(page, { focused: false });
  await open(page, `r/${ROOM_IDS.dmMaya}`);

  await postMessage(request, { roomId: ROOM_IDS.dmMaya, userId: MAYA, markdown: "Still there?" });
  await expect(page.getByText("Still there?")).toBeVisible();
  await expect.poll(() => chimes(page)).toBe(1);
});

test("two open tabs chime once between them", async ({ context, page, request }) => {
  const second = await context.newPage();

  await spyOnChimes(page);
  await spyOnChimes(second);
  await open(page, `r/${ROOM_IDS.general}`);
  await open(second, `r/${ROOM_IDS.general}`);

  await postMessage(request, { roomId: ROOM_IDS.dmMaya, userId: MAYA, markdown: "Both tabs" });

  await expect.poll(async () => (await chimes(page)) + (await chimes(second))).toBe(1);
  await page.waitForTimeout(1000);
  expect((await chimes(page)) + (await chimes(second))).toBe(1);
});

test("a tab that can't play yet leaves the chime to one that can", async ({
  context,
  page,
  request,
}) => {
  const silent = await context.newPage();

  await spyOnChimes(page);
  await spyOnChimes(silent, { gestured: false });
  await open(silent, `r/${ROOM_IDS.general}`, { gestured: false });
  await open(page, `r/${ROOM_IDS.general}`);

  for (const [index, markdown] of ["First", "Second", "Third"].entries()) {
    await postMessage(request, { roomId: ROOM_IDS.dmMaya, userId: MAYA, markdown });
    await expect.poll(() => chimes(page)).toBe(index + 1);
    await page.waitForTimeout(2100);
  }

  expect(await chimes(silent)).toBe(0);
});

test("a reply in a followed thread chimes while you read its room", async ({ page, request }) => {
  await spyOnChimes(page);
  await open(page, `r/${ROOM_IDS.general}`);

  await replyInThread(request, "Over in the thread");

  await expect.poll(() => chimes(page)).toBe(1);
});

test("a reply in the thread pane you're reading doesn't chime", async ({ page, request }) => {
  await spyOnChimes(page);
  await open(page, `r/${ROOM_IDS.general}/t/${THREAD_IDS.generalActive}`);

  await replyInThread(request, "Right here in the pane");
  await expect(page.getByText("Right here in the pane")).toBeVisible();
  await page.waitForTimeout(1000);
  expect(await chimes(page)).toBe(0);
});
