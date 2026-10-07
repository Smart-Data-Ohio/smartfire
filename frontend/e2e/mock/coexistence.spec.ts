import { expect, matrix, openApp, ROOM_IDS, shot, test } from "./support.ts";

matrix("the user menu offers the way back to classic", async ({ page, theme }) => {
  await openApp(page, "", theme);

  await page.getByRole("button", { name: "Your account" }).click();

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
  });
});

test("a destination the SPA hasn't ported opens on its classic page", async ({ page }) => {
  await page.route("**/searches?classic=1", (route) =>
    route.fulfill({ status: 200, contentType: "text/html", body: "<p>classic search</p>" }),
  );

  await page.goto("/app/search");

  await expect(page.getByText("classic search")).toBeVisible();
  expect(new URL(page.url()).pathname).toBe("/searches");
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
