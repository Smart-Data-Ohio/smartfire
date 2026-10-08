import { expect, type Page, test } from "@playwright/test";

interface Look {
  readonly name: string;
  readonly theme: "light" | "dark";
  readonly density: "comfortable" | "compact";
  /** "reduce" captures the page at rest; "full" runs the live effects (metal shader, beams). */
  readonly motion: "reduce" | "full";
}

const LIGHT: Look = { name: "light", theme: "light", density: "comfortable", motion: "reduce" };

const LOOKS: readonly Look[] = [
  LIGHT,
  { name: "dark", theme: "dark", density: "comfortable", motion: "reduce" },
  { name: "compact", theme: "dark", density: "compact", motion: "reduce" },
  { name: "dark-full-motion", theme: "dark", density: "comfortable", motion: "full" },
];

async function openKitchenSink(page: Page, look: Look, errors: string[]) {
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") {
      errors.push(message.text());
    }
  });

  await page.addInitScript(
    (appearance) => {
      localStorage.setItem("smartfire.appearance", JSON.stringify(appearance));
    },
    { themeOverride: look.theme, density: look.density, motion: look.motion },
  );

  await page.goto("/app/_kitchen-sink");
  await expect(page.getByRole("heading", { name: "Smartfire design system" })).toBeVisible();
  await page.evaluate(() => document.fonts.ready);
}

for (const look of LOOKS) {
  test(`the kitchen sink renders in ${look.name} without errors`, async ({ page }) => {
    const errors: string[] = [];

    await openKitchenSink(page, look, errors);

    await expect(page.locator("html")).toHaveAttribute("data-theme", look.theme);
    await expect(page.getByRole("region", { name: "App preview" })).toBeVisible();

    // Screenshots are for review, not committed baselines: set SHOTS_DIR to write them.
    const shotsDir = process.env.SHOTS_DIR;

    if (shotsDir !== undefined) {
      await page.setViewportSize({ width: 1440, height: 900 });
      // Bot avatars render to bitmaps when the browser is idle; wait so the shots show them.
      await expect(page.locator(".agent-avatar img").first()).toBeVisible();

      if (look.motion === "full") {
        // Let the lazy effect chunks load and their first frames paint.
        await page.waitForTimeout(2000);
      }

      await page.screenshot({ path: `${shotsDir}/kitchen-sink-${look.name}.png`, fullPage: true });
      await page
        .getByRole("region", { name: "App preview" })
        .screenshot({ path: `${shotsDir}/mock-app-${look.name}.png` });
      await page
        .locator(".ks-section")
        .filter({ has: page.getByRole("heading", { name: "Button", exact: true }) })
        .screenshot({ path: `${shotsDir}/buttons-${look.name}.png` });
    }

    expect(errors).toEqual([]);
  });
}

test("an icon button shows its tooltip on keyboard focus", async ({ page }) => {
  await openKitchenSink(page, LIGHT, []);

  const button = page.getByRole("button", { name: "Search" }).first();

  await button.focus();
  await page.keyboard.press("Shift+Tab");
  await page.keyboard.press("Tab");

  // The bubble is aria-hidden (the button already carries the label), so find it by its role
  // attribute rather than through the accessibility tree.
  await expect(page.locator('[role="tooltip"]', { hasText: "Search" })).toBeVisible();
});
