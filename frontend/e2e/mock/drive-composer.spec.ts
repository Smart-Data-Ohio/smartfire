import { expect, expectTouchTargets, PHONE_TOUCH, ROOM_IDS, test } from "./support.ts";

test.use(PHONE_TOUCH);

test("search attaches a Drive file, and an edit can remove it", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM_IDS.general}`);
  await page.getByRole("log", { name: "Messages" }).waitFor();

  await page.getByRole("button", { name: "Attach and more" }).click();
  await page.getByRole("menuitem", { name: "From Google Drive" }).click();

  const search = page.getByRole("combobox", { name: "Search Drive files" });

  await search.fill("roadmap");
  await expect(page.getByRole("option", { name: /Q4 roadmap/ })).toBeVisible();
  await expectTouchTargets(page, ".drive-picker");
  await search.press("ArrowDown");
  await search.press("Enter");

  await expect(page.getByRole("dialog", { name: "Share a Drive file" })).toBeVisible();
  await page.getByRole("button", { name: "Attach only" }).click();
  await expect(page.getByRole("button", { name: "Remove Q4 roadmap" })).toBeVisible();

  const input = page.getByRole("textbox", { name: "Message #general" });

  await input.click();
  await input.pressSequentially("see the roadmap");
  await expect(input).toHaveValue("see the roadmap");
  await page.getByRole("button", { name: "Send message" }).tap();

  const row = page.locator("[data-message-row]", { hasText: "see the roadmap" });

  await expect(row).toHaveAttribute("data-message-id", /^\d+$/);
  await expect(row.getByRole("link", { name: /Google Drive file/ })).toBeVisible();

  await row.locator(".message-body").click({ button: "right" });
  await page.getByRole("menuitem", { name: "Edit message" }).click();
  await page.getByRole("button", { name: "Remove Google Drive file" }).click();
  await page.getByRole("textbox", { name: "Edit message" }).press("Enter");
  await expect(row.getByRole("link", { name: /Google Drive file/ })).toHaveCount(0);
});
