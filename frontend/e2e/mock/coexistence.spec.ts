import { expect, matrix, openApp, ROOM_IDS, SHOTS, shot, test } from "./support.ts";

matrix("the user menu has no way back to classic", async ({ page, theme, phone }) => {
  await openApp(page, "", theme);

  // A phone has it under the tab bar's You, the sidebar's foot on wider screens.
  await page.getByRole("button", { name: phone ? "You" : "Your account" }).click();

  const menu = page.getByRole("menu", { name: "Your account" });

  await expect(menu.getByRole("menuitem", { name: "Profile and settings" })).toBeVisible();
  await expect(menu.getByRole("menuitem", { name: /classic/i })).toHaveCount(0);

  if (SHOTS) {
    // The menu's entrance finishes before the shot.
    await page.evaluate(() =>
      Promise.all(
        document
          .getAnimations()
          .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
          .map((animation) => animation.finished.catch(() => animation)),
      ),
    );
    await shot(page, "user-menu", theme);
  }
});

test("a link to a ported classic page opens in place", async ({ page }) => {
  await openApp(page, "");

  await page.evaluate((roomId) => {
    const link = document.createElement("a");

    link.href = `/rooms/${roomId}`;
    link.textContent = "classic link";
    document.querySelector("main")?.append(link);
    Object.assign(window, { stillHere: true });
  }, ROOM_IDS.general);
  await page.getByText("classic link").click();

  await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}$`));
  expect(await page.evaluate(() => "stillHere" in window)).toBe(true);
});
