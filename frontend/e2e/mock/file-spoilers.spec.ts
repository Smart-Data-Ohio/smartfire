import { onboardingMockupPng } from "../../mock/s2/assets.ts";
import { expect, expectTouchTargets, PHONE_TOUCH, ROOM_IDS, test } from "./support.ts";

for (const phone of [false, true]) {
  test.describe(phone ? "on a phone" : "on desktop", () => {
    if (phone) test.use(PHONE_TOUCH);
    test("marks one file as a spoiler and defers its thumbnail and lightbox until reveal", async ({
      page,
    }) => {
      await page.emulateMedia({ reducedMotion: "reduce" });
      const mediaRequests: string[] = [];
      page.on("request", (request) => {
        if (
          request.url().includes("/rails/active_storage/") &&
          request.url().includes("secret.png")
        )
          mediaRequests.push(request.url());
      });
      await page.goto(`/app/r/${ROOM_IDS.general}`);
      const composer = page.getByRole("textbox", { name: "Message #general" });
      await composer.waitFor();
      await page.locator('.composer input[type="file"]').setInputFiles(
        ["public.png", "secret.png"].map((name) => ({
          name,
          mimeType: "image/png",
          buffer: Buffer.from(onboardingMockupPng()),
        })),
      );
      await expect(page.locator('.tray-chip[data-phase="done"]')).toHaveCount(2);
      const toggle = page.getByRole("button", { name: "Mark secret.png as spoiler" });
      await toggle.focus();
      await toggle.press("Enter");
      await expect(toggle).toHaveAttribute("aria-pressed", "true");

      if (phone) await expectTouchTargets(page, ".tray");
      await composer.fill("Individual file spoilers");
      await composer.press("Enter");

      const row = page.getByRole("log", { name: "Messages" }).locator("[data-message-id]", {
        has: page.getByText("Individual file spoilers", { exact: true }),
      });

      await expect(row).toHaveCount(1);
      await expect(row.locator("img[alt='secret.png']")).toHaveCount(0);
      expect(mediaRequests).toEqual([]);

      const coverBox = await row
        .getByRole("button", { name: "Spoiler, activate to reveal" })
        .boundingBox();

      const tileBox = await row.getByRole("button", { name: "Open public.png" }).boundingBox();

      expect(coverBox?.width).toBe(tileBox?.width);
      expect(coverBox?.height).toBe(tileBox?.height);
      await row.screenshot({
        path: `../.scratch/file-spoilers-${phone ? "phone" : "desktop"}.png`,
      });
      await row.getByRole("button", { name: "Open public.png" }).click();
      const lightbox = page.getByRole("dialog", { name: "public.png" });
      await expect(lightbox).toBeVisible();
      await expect(lightbox.getByRole("button", { name: "Next image" })).toHaveCount(0);
      expect(mediaRequests).toEqual([]);
      await lightbox.getByRole("button", { name: "Close" }).click();
      await expect(lightbox).toBeHidden();
      const cover = row.getByRole("button", { name: "Spoiler, activate to reveal" });
      await cover.focus();
      await expect(cover).toBeFocused();
      await cover.press("Enter");
      await expect(row.getByRole("button", { name: "Open secret.png" })).toBeVisible();
      await expect.poll(() => mediaRequests.length).toBeGreaterThan(0);
      await row.getByRole("button", { name: "Open secret.png" }).click();
      await expect(page.getByRole("dialog", { name: "secret.png" })).toContainText("2 of 2");
      await page.reload();
      await expect(row.getByRole("button", { name: "Spoiler, activate to reveal" })).toBeVisible();
      await page.goto("/app/search?q=Individual");

      const file = page
        .locator("[data-search-hit] .search-hit-file")
        .filter({ hasText: "secret.png" });

      await expect(file.locator("img")).toHaveCount(0);
      await file.getByRole("button", { name: "Spoiler, activate to reveal" }).click();
      await expect(file.locator(".search-hit-thumb")).toBeVisible();
      await expect(page).toHaveURL(/\/app\/search/);
    });
  });
}
