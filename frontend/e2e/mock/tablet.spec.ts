import { THREAD_IDS } from "../../mock/s2/seed.ts";
import { expect, openApp, ROOM_IDS, shot, TABLET, test } from "./support.ts";

const GENERAL = ROOM_IDS.general;

// Tablets (720–1100 px) keep the sidebar and float the right pane over the room as a sheet.
for (const theme of ["light", "dark"] as const) {
  test.describe(`tablet (${theme})`, () => {
    test.beforeEach(async ({ page }) => {
      await page.setViewportSize(TABLET);
    });

    test("a thread opens as a sheet over the room", async ({ page }) => {
      await openApp(page, `r/${GENERAL}/t/${THREAD_IDS.generalActive}`, theme);

      const pane = page.locator("aside.right-pane");

      await expect(pane.getByRole("log", { name: "Replies" })).toBeVisible();
      await expect(page.getByRole("complementary", { name: "Conversations" })).toBeVisible();
      await page.mouse.move(0, 0);
      await shot(page, "thread-sheet", theme);
    });

    test("the members pane opens as a sheet", async ({ page }) => {
      await openApp(page, `r/${GENERAL}`, theme);
      await page.getByRole("button", { name: /^Members:/ }).click();
      await expect(
        page.locator("aside.right-pane").getByRole("heading", { name: "Members" }),
      ).toBeVisible();
      await expect(
        page.locator("aside.right-pane").getByText("Maya Okafor", { exact: true }),
      ).toBeVisible();
      await page.mouse.move(0, 0);
      await shot(page, "members-sheet", theme);
    });

    test("the room on its own", async ({ page }) => {
      await openApp(page, `r/${GENERAL}`, theme);
      await expect(page.getByRole("log", { name: "Messages" })).toBeVisible();
      await page.mouse.move(0, 0);
      await shot(page, "room", theme);
    });
  });
}
