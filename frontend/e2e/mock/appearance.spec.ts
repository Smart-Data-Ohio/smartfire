import type { Page } from "@playwright/test";
import { ROOM_IDS } from "../../mock/seed.ts";
import { expect, matrix, openApp, saveAccountAppearance, shot, test } from "./support.ts";

/** The computed font size of `selector`, in px. */
function fontSize(page: Page, selector: string): Promise<number> {
  return page
    .locator(selector)
    .first()
    .evaluate((element) => Number.parseFloat(getComputedStyle(element).fontSize));
}

const html = (page: Page) => page.locator("html");

test("the account's theme applies on start, over what this device last showed", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  // A device that last showed the light theme, before the account switched to dark elsewhere.
  await page.addInitScript(() => {
    if (localStorage.getItem("smartfire.appearance") === null) {
      localStorage.setItem("smartfire.appearance", JSON.stringify({ theme: "light" }));
    }
  });
  await openApp(page, "");
  await saveAccountAppearance(page, { theme: "dark" });

  await page.reload();
  await page.getByRole("complementary", { name: "Conversations" }).waitFor();
  await expect(html(page)).toHaveAttribute("data-theme", "dark");
});

test("a theme pinned on this device wins, until it's set back to the account's", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await openApp(page, "settings/appearance");
  await saveAccountAppearance(page, { theme: "dark" });
  await page.reload();
  await expect(html(page)).toHaveAttribute("data-theme", "dark");

  const device = page.getByLabel("Theme on this device");

  await expect(device).toHaveAccessibleDescription(
    "Pin a theme here without changing it on your other devices.",
  );

  await device.selectOption({ label: "Light" });
  await expect(html(page)).toHaveAttribute("data-theme", "light");

  await page.reload();
  await expect(html(page)).toHaveAttribute("data-theme", "light");
  await expect(page.getByRole("radio", { name: "Dark" })).toBeChecked();

  await page.getByLabel("Theme on this device").selectOption({ label: "Use my account's theme" });
  await expect(html(page)).toHaveAttribute("data-theme", "dark");
});

test("the five text sizes change the text, and the choice comes back on start", async ({
  page,
}) => {
  await openApp(page, "settings/appearance");

  const sizes: Record<string, number> = {};

  for (const label of ["Smaller", "Small", "Default", "Large", "Larger"]) {
    await page.getByRole("radio", { name: label, exact: true }).check();
    await expect(page.getByRole("radio", { name: label, exact: true })).toBeChecked();
    sizes[label] = await fontSize(page, ".settings-page h1, h1");
  }

  expect(sizes.Smaller).toBeLessThan(sizes.Small ?? 0);
  expect(sizes.Small).toBeLessThan(sizes.Default ?? 0);
  expect(sizes.Default).toBeLessThan(sizes.Large ?? 0);
  expect(sizes.Large).toBeLessThan(sizes.Larger ?? 0);

  // The account keeps "Larger": a fresh start shows it, messages included.
  await openApp(page, "");
  await expect(html(page)).toHaveAttribute("data-text-size", "larger");
  expect(await page.evaluate(() => getComputedStyle(document.documentElement).fontSize)).toBe(
    "18px",
  );
});

test("the sidebar's theme button saves to the account when nothing is pinned", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await openApp(page, "");

  const button = page.getByRole("button", { name: /^Theme: / });

  await button.click();
  await expect(html(page)).toHaveAttribute("data-theme", /light|dark/);

  const shown = await html(page).getAttribute("data-theme");

  await page.goto("/app/settings/appearance");
  await expect(
    page.getByRole("radio", { name: shown === "dark" ? "Dark" : "Light" }),
  ).toBeChecked();
});

test.describe("on a touch phone", () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

  test("the composer stays at 16px at the smallest text size, so iOS never zooms in", async ({
    page,
  }) => {
    await openApp(page, "");
    await saveAccountAppearance(page, { textSize: "smaller" });
    await openApp(page, `r/${ROOM_IDS.general}`);

    const composer = page.getByRole("textbox", { name: "Message #general" });

    await expect(html(page)).toHaveAttribute("data-text-size", "smaller");
    expect(
      await composer.evaluate((element) => Number.parseFloat(getComputedStyle(element).fontSize)),
    ).toBeGreaterThanOrEqual(16);
  });
});

matrix("appearance from the account", async ({ page, theme }) => {
  await openApp(page, "settings/appearance");
  await saveAccountAppearance(page, { theme, textSize: "large" });
  await page.reload();
  await expect(html(page)).toHaveAttribute("data-theme", theme);
  await page.mouse.move(0, 0);
  await shot(page, "appearance-settings", theme);

  await page.goto("/app/");
  await page.getByRole("complementary", { name: "Conversations" }).waitFor();
  await page.mouse.move(0, 0);
  await shot(page, "appearance-large-text", theme);
});
