import type { Page } from "@playwright/test";
import { B5_HZ } from "../../src/features/huddle/chime.ts";
import { expect, openApp, postMessage, ROOM_IDS, syncWelcomed, test } from "./support.ts";

const MAYA = 2;

const JONAH = 3;

declare global {
  interface Window {
    chimes: number;
  }
}

async function spyOnChimes(page: Page, { focused = true } = {}): Promise<void> {
  await page.addInitScript(
    ({ windowFocused, note }) => {
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
    },
    { windowFocused: focused, note: B5_HZ },
  );
}

async function open(page: Page, path: string): Promise<void> {
  const welcomed = syncWelcomed(page);

  await openApp(page, path);
  await welcomed;
  // Browsers allow sound only after a gesture on the page.
  await page.keyboard.press("Shift");
  await expect.poll(() => page.evaluate(() => navigator.userActivation.hasBeenActive)).toBe(true);
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
