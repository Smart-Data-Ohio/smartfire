import { expect, matrix, openApp, ROOM_IDS, SHOTS, shot, test } from "./support.ts";

// Screenshots only: the Switch to classic test and nav.spec's phone account sheet cover the menu.
if (SHOTS) {
  matrix("the user menu offers the way back to classic", async ({ page, theme, phone }) => {
    await openApp(page, "", theme);

    // A phone has it under the tab bar's You, the sidebar's foot on wider screens.
    await page.getByRole("button", { name: phone ? "You" : "Your account" }).click();

    const menu = page.getByRole("menu", { name: "Your account" });

    await expect(menu.getByRole("menuitem", { name: "Profile and settings" })).toBeVisible();
    await expect(menu.getByRole("menuitem", { name: "Switch to classic" })).toBeVisible();
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
  });
}

test("Switch to classic posts the choice and where the person is", async ({ page }) => {
  let posted: URLSearchParams | null = null;

  await page.route("**/app/ui_preference", async (route) => {
    posted = new URLSearchParams(route.request().postData() ?? "");
    await route.fulfill({ status: 200, contentType: "text/html", body: "<p>classic</p>" });
  });
  await openApp(page, `r/${ROOM_IDS.general}`);

  await page.getByRole("button", { name: "Your account" }).click();
  await page.getByRole("menuitem", { name: "Switch to classic" }).click();

  await expect(page.getByText("classic", { exact: true })).toBeVisible();
  expect(posted && Object.fromEntries(posted)).toEqual({
    ui: "classic",
    return_to: `/app/r/${ROOM_IDS.general}`,
    authenticity_token: expect.any(String),
  });
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
