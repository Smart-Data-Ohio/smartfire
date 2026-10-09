import type { Page } from "@playwright/test";
import { BOARD_ROOM_ID } from "../../mock/s6/seed.ts";
import { DESKTOP, expect, openApp, test } from "./support.ts";

const BOARD = BOARD_ROOM_ID;

function pane(page: Page) {
  return page.locator("aside.right-pane");
}

test.describe("board settings", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize(DESKTOP);
  });

  test("renames a board from its settings", async ({ page }) => {
    await openApp(page, `r/${BOARD}`);
    await page.getByRole("link", { name: /room settings$/ }).click();

    const dialog = page.getByRole("dialog", { name: "Board settings" });

    await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD}/settings$`));
    await expect(dialog.getByRole("switch")).toHaveCount(0);
    await expect(dialog.getByRole("tab", { name: /Members/ })).toBeVisible();
    await expect(dialog.getByRole("button", { name: "Board automations" })).toBeVisible();
    await dialog.getByLabel("Name", { exact: true }).fill("shipped-roadmap");
    await dialog.getByRole("button", { name: "Save changes" }).click();

    await expect(dialog).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD}$`));
    await expect(page.getByRole("heading", { level: 1, name: "shipped-roadmap" })).toBeVisible();
  });

  test("a classic automations link opens the pane", async ({ page }) => {
    await openApp(page, `r/${BOARD}`);
    await page.evaluate((roomId) => {
      const link = document.createElement("a");

      link.href = `/rooms/boards/${roomId}/automations`;
      link.textContent = "open classic automations";
      document.querySelector("main")?.append(link);
    }, BOARD);
    await page.getByText("open classic automations").click();

    await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD}/automations$`));
    await expect(pane(page).getByRole("heading", { name: "Automations" })).toBeVisible();
    await expect(pane(page).getByRole("spinbutton", { name: "Planned nudge minutes" })).toHaveValue(
      "1440",
    );
  });
});
