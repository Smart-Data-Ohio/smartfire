import type { Page } from "@playwright/test";
import {
  DESKTOP,
  expect,
  openApp,
  PHONE_SMALL,
  postMessage,
  ROOM_IDS,
  SHOTS,
  shot,
  THEMES,
  test,
} from "./support.ts";

/** Lets entrance animations (the live avatar's fade-in, the bitmap's handoff) finish. */
async function settle(page: Page): Promise<void> {
  await page.mouse.move(0, 0);
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
        .map((animation) => animation.finished.catch(() => animation)),
    ),
  );
}

/** Creates Whiskers with a picked cat in a crown, returning its id from the page it opens. */
async function createWhiskers(page: Page, theme: "light" | "dark" = "light"): Promise<number> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto("/app/admin/bots/new");
  await page.locator(".settings-page h1").waitFor();

  await page.getByRole("textbox", { name: "Name" }).fill("Whiskers");
  await expect(page.getByRole("radio", { name: "Bot avatar" })).toBeChecked();
  await page.getByRole("radio", { name: "Cat", exact: true }).check({ force: true });
  await page.getByRole("radio", { name: "Teal" }).check({ force: true });
  await page.getByRole("radio", { name: "Eyes and mouth" }).check({ force: true });
  await page.getByRole("radio", { name: "Crown" }).check({ force: true });
  await expect(page.getByText("Cat in Teal, crown")).toBeVisible();
  // Every body tile has its drawing (each is drawn while the page is idle, cached after).
  await expect(page.locator(".bot-avatar-body img[src^='data:']")).toHaveCount(18, {
    timeout: 15_000,
  });
  await settle(page);
  await shot(page, "bot-avatar-picker", theme);

  // The lightest and darkest swatches keep their outline on the tiles in either theme.
  if (SHOTS) {
    for (const [swatch, name] of [
      ["Snow", "bot-avatar-picker-snow"],
      ["Charcoal", "bot-avatar-picker-charcoal"],
    ] as const) {
      await page.getByRole("radio", { name: swatch }).check({ force: true });
      await expect(page.getByText(`Cat in ${swatch}, crown`)).toBeVisible();
      await expect(page.locator(".bot-avatar-body img[src^='data:']")).toHaveCount(18, {
        timeout: 15_000,
      });
      await settle(page);
      await shot(page, name, theme);
    }

    await page.getByRole("radio", { name: "Teal" }).check({ force: true });
  }

  await page.getByRole("button", { name: "Create bot" }).click();
  await page
    .getByRole("alertdialog", { name: "Whiskers's key" })
    .getByRole("button", { name: "Done" })
    .click();
  await expect(page).toHaveURL(/\/app\/admin\/bots\/\d+$/);

  return Number(new URL(page.url()).pathname.split("/").pop());
}

test("a picked bot avatar is kept, edited and drawn beside the bot's messages", async ({
  page,
  request,
}) => {
  const id = await createWhiskers(page);

  await expect(page.getByRole("heading", { level: 1, name: "Whiskers" })).toBeVisible();
  await expect(page.getByRole("radio", { name: "Cat", exact: true })).toBeChecked();
  await expect(page.getByRole("radio", { name: "Crown" })).toBeChecked();

  await page.getByRole("radio", { name: "Ghost" }).check({ force: true });
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByText("Bot saved")).toBeVisible();

  await page.reload();
  await expect(page.getByRole("radio", { name: "Ghost" })).toBeChecked();
  await expect(page.getByRole("radio", { name: "Crown" })).toBeChecked();

  await postMessage(request, {
    roomId: ROOM_IDS.engineering,
    userId: id,
    markdown: "Purring along",
  });
  await openApp(page, `r/${ROOM_IDS.engineering}`);

  const row = page.locator("[data-message-row]").filter({ hasText: "Purring along" });

  await expect(row.locator('[data-bot-avatar="ghost"]')).toBeVisible();
  // Lists stay still: the row draws a bitmap, never a live canvas.
  await expect(row.locator("canvas")).toHaveCount(0);

  await page.goto(`/app/people/${id}`);
  await expect(page.locator('[data-bot-avatar="ghost"] canvas')).toBeVisible();
});

test("an icon bot keeps its icon until a bot avatar is picked", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/app/admin/bots");
  await page.getByRole("link", { name: "Edit Ember" }).click();

  await expect(page.getByRole("radio", { name: "Keep current" })).toBeChecked();
  await expect(page.getByRole("textbox", { name: "Icon" })).toHaveCount(0);

  await page.getByRole("radio", { name: "Bot avatar" }).check({ force: true });
  await expect(page.getByRole("group", { name: "Body" })).toBeVisible();
});

// Screenshots only: the picker, a message row and the bot's page, at desktop and on a 360 px
// phone, in both themes.
if (SHOTS) {
  for (const theme of THEMES) {
    for (const [label, viewport] of [
      ["desktop", DESKTOP],
      ["phone", PHONE_SMALL],
    ] as const) {
      test(`bot avatar shots (${theme}, ${label})`, async ({ page, request }) => {
        await page.setViewportSize(viewport);

        const id = await createWhiskers(page, theme);

        await postMessage(request, {
          roomId: ROOM_IDS.engineering,
          userId: id,
          markdown: "Purring along",
        });
        await openApp(page, `r/${ROOM_IDS.engineering}`, theme);

        const row = page.locator("[data-message-row]").filter({ hasText: "Purring along" });

        await expect(row.locator('[data-bot-avatar="cat"] img')).toBeVisible();
        await settle(page);
        await shot(page, "bot-avatar-message", theme);

        await page.goto(`/app/people/${id}`);
        await expect(page.locator('[data-bot-avatar="cat"] canvas').first()).toBeVisible();
        await page.waitForTimeout(800);
        await settle(page);
        await shot(page, "bot-avatar-profile", theme);

        // Ember, an icon bot, picks a ghost; its DM draws it.
        await page.goto("/app/admin/bots");
        await page.getByRole("link", { name: "Edit Ember" }).click();
        await page.getByRole("radio", { name: "Bot avatar" }).check({ force: true });
        await page.getByRole("radio", { name: "Ghost" }).check({ force: true });
        await page.getByRole("button", { name: "Save", exact: true }).click();
        await expect(page.getByText("Bot saved")).toBeVisible();

        await openApp(page, `r/${ROOM_IDS.dmEmber}`, theme);

        if (label === "desktop") {
          // Wide screens have no details pane for a DM: the header and rows show the ghost.
          await expect(page.locator('main [data-bot-avatar="ghost"]').first()).toBeVisible();
          await settle(page);
          await shot(page, "bot-avatar-dm", theme);

          return;
        }

        // A phone's DM title opens its details, with the bot drawn live in the hero.
        await page.locator(".room-header .room-title-button").click();

        const pane = page.locator("aside.right-pane");

        await expect(pane.locator('[data-bot-avatar="ghost"] canvas')).toBeVisible();
        await page.waitForTimeout(800);
        await settle(page);
        await shot(page, "bot-avatar-details", theme);
      });
    }
  }
}
