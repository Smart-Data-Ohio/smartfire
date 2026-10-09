import type { APIRequestContext, Page } from "@playwright/test";
import {
  DESKTOP,
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  openApp,
  PHONE_TOUCH,
  ROOM_IDS,
  test,
} from "./support.ts";

const TITLES = [
  "Your rooms live here",
  "Write below the messages",
  "Talk it out in a huddle",
  "Jump anywhere with Ctrl+K",
  "Shortcuts live under Ctrl+/",
] as const;

async function mockState(request: APIRequestContext) {
  return (await request.get("/__mock/state")).json();
}

/** Clears the viewer's tour stamp: the mock seeds it set so the tour stays out of other specs. */
async function newcomer(request: APIRequestContext): Promise<void> {
  const { csrfToken } = await mockState(request);

  await request.post("/__mock/tour", {
    headers: { "X-CSRF-Token": csrfToken },
    data: { completed: false },
  });
}

async function stamps(request: APIRequestContext): Promise<number> {
  return (await mockState(request)).tourStamps;
}

const tourDialog = (page: Page) => page.locator("dialog.tour");

const step = (page: Page, title: string) => page.getByRole("dialog", { name: title });

/** Reloads and waits for `/me`, so a tour that was going to start by itself has had its chance. */
async function reloadSettled(page: Page): Promise<void> {
  const me = page.waitForResponse((response) => response.url().endsWith("/api/v1/me"));

  await page.reload();
  await me;
  await page.waitForTimeout(300);
}

test.describe("the product tour on desktop", () => {
  test.use({ viewport: DESKTOP });

  test("starts for a newcomer, finishes once, stays away on reload, and restarts from help", async ({
    page,
    request,
  }) => {
    await newcomer(request);
    await openApp(page, `r/${ROOM_IDS.general}`);

    const first = step(page, TITLES[0]);

    await expect(first).toBeVisible();
    await expect(first.getByText("Step 1 of 5")).toBeVisible();
    await expect(first.getByRole("button", { name: "Next" })).toBeFocused();
    await expect(first.getByRole("button", { name: "Back" })).toHaveCount(0);
    // Anchored to the sidebar, with the card beside it.
    await expect(tourDialog(page)).toHaveAttribute("data-step", "sidebar");
    await expect(tourDialog(page)).toHaveAttribute("data-anchored", "");

    const sidebar = await page.locator(".sidebar").boundingBox();
    const card = await page.locator(".tour-card").boundingBox();

    expect(card?.x ?? 0).toBeGreaterThanOrEqual((sidebar?.x ?? 0) + (sidebar?.width ?? 0));

    // ←/→ step back and forth; the composer and the call button are on screen in a room.
    await page.keyboard.press("ArrowRight");
    await expect(step(page, TITLES[1])).toBeVisible();
    await expect(tourDialog(page)).toHaveAttribute("data-anchored", "");
    await page.keyboard.press("ArrowLeft");
    await expect(first).toBeVisible();

    // Tab stays in the card.
    for (let press = 0; press < 4; press += 1) await page.keyboard.press("Tab");

    await expect(page.locator(".tour-card button:focus")).toHaveCount(1);

    for (const title of TITLES.slice(0, -1)) {
      await expect(step(page, title)).toBeVisible();
      await step(page, title).getByRole("button", { name: "Next" }).click();
    }

    const last = step(page, TITLES[4]);

    await expect(last.getByText("Step 5 of 5")).toBeVisible();
    await expect(tourDialog(page)).toHaveAttribute("data-anchored", "");
    await last.getByRole("button", { name: "Finish" }).click();
    await expect(tourDialog(page)).toHaveCount(0);
    await expect.poll(() => stamps(request)).toBe(1);

    await reloadSettled(page);
    await expect(tourDialog(page)).toHaveCount(0);

    const account = page.getByRole("button", { name: "Your account" });

    await account.click();

    const menu = page.getByRole("menu", { name: "Your account" });

    await expect(menu.getByRole("menuitem", { name: /Keyboard shortcuts/ })).toBeVisible();
    await menu.getByRole("menuitem", { name: "Restart tour" }).click();
    await expect(first).toBeVisible();
    await expect(first.getByRole("button", { name: "Next" })).toBeFocused();

    // Escape skips, stamps again, and hands focus back.
    await page.keyboard.press("Escape");
    await expect(tourDialog(page)).toHaveCount(0);
    await expect.poll(() => stamps(request)).toBe(2);
    await expect(account).toBeFocused();
  });

  test("glides between anchors only where motion is allowed, and always fades", async ({
    page,
    request,
  }) => {
    const motion = () =>
      page.evaluate(() => {
        const of = (selector: string) => {
          const element = document.querySelector(selector);

          return element === null ? "" : getComputedStyle(element).transitionProperty;
        };

        return {
          spotlight: of(".tour-spotlight"),
          card: of(".tour-card"),
          dialog: of("dialog.tour"),
        };
      });

    /** The app's own motion preference, as lib/appearance.ts writes it on the root. */
    const preferMotion = (value: "reduce" | "full" | null) =>
      page.evaluate((next) => {
        if (next === null) {
          delete document.documentElement.dataset.motion;
        } else {
          document.documentElement.dataset.motion = next;
        }
      }, value);

    await newcomer(request);
    // openApp emulates the OS's reduced motion.
    await openApp(page, `r/${ROOM_IDS.general}`);
    await expect(tourDialog(page)).toBeVisible();

    const still = { spotlight: "none", card: "none" };

    expect(await motion()).toMatchObject(still);
    expect((await motion()).dialog).toContain("opacity");

    // "Full" in the app overrides the OS.
    await preferMotion("full");
    expect((await motion()).spotlight).toContain("top");
    expect((await motion()).card).toContain("left");

    // With no OS preference, the app's own "reduce" stills it too.
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await preferMotion(null);
    expect((await motion()).spotlight).toContain("width");
    await preferMotion("reduce");
    expect(await motion()).toMatchObject(still);
    expect((await motion()).dialog).toContain("opacity");
  });

  test("stays out of the way of someone who completed it", async ({ page, request }) => {
    await openApp(page, `r/${ROOM_IDS.general}`);
    await reloadSettled(page);
    await expect(tourDialog(page)).toHaveCount(0);
    expect(await stamps(request)).toBe(0);
  });
});

test.describe("the product tour on a 360 px touch phone", () => {
  test.use(PHONE_TOUCH);

  test("anchors what the phone shows, centres the rest, and restarts from the You tab", async ({
    page,
    request,
  }) => {
    await newcomer(request);
    await openApp(page, "");

    const first = step(page, TITLES[0]);

    // The conversation list is on screen: the sidebar step anchors to it, the card a sheet.
    await expect(first).toBeVisible();
    await expect(tourDialog(page)).toHaveAttribute("data-anchored", "");
    await expect(tourDialog(page)).toHaveAttribute("data-dock", /^(top|bottom)$/);
    await expectTouchTargets(page, ".tour-card");
    await expectNoHorizontalOverflow(page);

    // No composer or call button on the list: those steps centre their card.
    await first.getByRole("button", { name: "Next" }).tap();
    await expect(step(page, TITLES[1])).toBeVisible();
    await expect(tourDialog(page)).not.toHaveAttribute("data-anchored", "");

    const card = await page.locator(".tour-card").boundingBox();

    expect(card?.x ?? -1).toBeGreaterThanOrEqual(0);
    expect((card?.x ?? 0) + (card?.width ?? Number.POSITIVE_INFINITY)).toBeLessThanOrEqual(360);
    await expectTouchTargets(page, ".tour-card");

    await step(page, TITLES[1]).getByRole("button", { name: "Next" }).tap();
    await step(page, TITLES[2]).getByRole("button", { name: "Next" }).tap();
    await step(page, TITLES[3]).getByRole("button", { name: "Next" }).tap();

    // The help items are behind the You tab, low on the screen: the sheet docks at the top.
    const last = step(page, TITLES[4]);

    await expect(last).toBeVisible();
    await expect(tourDialog(page)).toHaveAttribute("data-anchored", "");
    await expect(tourDialog(page)).toHaveAttribute("data-dock", "top");
    await last.getByRole("button", { name: "Finish" }).tap();
    await expect(tourDialog(page)).toHaveCount(0);
    await expect.poll(() => stamps(request)).toBe(1);

    await reloadSettled(page);
    await expect(tourDialog(page)).toHaveCount(0);

    await page
      .getByRole("navigation", { name: "Destinations" })
      .getByRole("button", { name: "You" })
      .tap();
    await page
      .getByRole("menu", { name: "Your account" })
      .getByRole("menuitem", { name: "Restart tour" })
      .tap();
    await expect(first).toBeVisible();
    await first.getByRole("button", { name: "Skip tour" }).tap();
    await expect(tourDialog(page)).toHaveCount(0);
    await expect.poll(() => stamps(request)).toBe(2);
  });
});
