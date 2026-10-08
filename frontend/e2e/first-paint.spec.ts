import { expect, type Page, test } from "@playwright/test";

/**
 * The production build's first paint, before any of the SPA's JavaScript runs: the page is served
 * as the Rust shell serves it (the boot JSON inlined at `<!--boot-->`), and the entry module is
 * held until the test has looked. The stylesheet is a `<link>` in the build, so what's painted
 * here is what a person on a slow connection sees first.
 */
async function openHeld(page: Page, boot: { theme: string; textSize: string }) {
  const entry = Promise.withResolvers<void>();

  await page.route(/\/app\/assets\/index-[\w-]+\.js$/, async (route) => {
    await entry.promise;
    await route.continue();
  });
  await page.route(/\/app\/(\?.*)?$/, async (route) => {
    const response = await route.fetch();

    const inline = JSON.stringify({
      user: { id: 1, name: "Riel St. Amand", avatarUrl: "/avatar.svg" },
      account: { name: "Smart Data" },
      cableUrl: "/cable",
      version: "test",
      revision: null,
      ...boot,
    });

    const html = await response.text();

    expect(html, "the build keeps the shell's placeholder").toContain("<!--boot-->");
    await route.fulfill({
      response,
      body: html.replace(
        "<!--boot-->",
        `<script type="application/json" id="boot">${inline}</script>`,
      ),
    });
  });
  await page.goto("/app/", { waitUntil: "commit" });
  // The stylesheet has applied: the body has a painted background.
  await page.waitForFunction(
    () =>
      document.body !== null &&
      getComputedStyle(document.body).backgroundColor !== "rgba(0, 0, 0, 0)",
  );
  expect(
    await page.evaluate(() => document.getElementById("root")?.childElementCount),
    "the SPA hasn't rendered yet",
  ).toBe(0);

  return entry;
}

/** WCAG relative luminance of the body's painted background. */
function backgroundLuminance(page: Page): Promise<number> {
  return page.evaluate(() => {
    const canvas = document.createElement("canvas").getContext("2d");

    if (canvas === null) throw new Error("no 2d canvas");
    canvas.fillStyle = getComputedStyle(document.body).backgroundColor;
    canvas.fillRect(0, 0, 1, 1);

    const [r = 0, g = 0, b = 0] = canvas.getImageData(0, 0, 1, 1).data;

    const linear = (channel: number) => {
      const c = channel / 255;

      return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    };

    return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
  });
}

test("the account's dark theme and text size are painted before the SPA's script runs", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });

  const entry = await openHeld(page, { theme: "dark", textSize: "larger" });

  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  expect(await backgroundLuminance(page)).toBeLessThan(0.05);
  expect(await page.evaluate(() => getComputedStyle(document.documentElement).fontSize)).toBe(
    "18px",
  );
  entry.resolve();
});

test("a theme pinned on this device is painted first, over the account's", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await page.addInitScript(() =>
    localStorage.setItem("smartfire.appearance", JSON.stringify({ themeOverride: "light" })),
  );

  const entry = await openHeld(page, { theme: "dark", textSize: "default" });

  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  expect(await backgroundLuminance(page)).toBeGreaterThan(0.8);
  entry.resolve();
});
