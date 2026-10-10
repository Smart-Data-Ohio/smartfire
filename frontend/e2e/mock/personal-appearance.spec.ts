import { expect, openApp, test } from "./support.ts";

test("a personal palette saves to the account, syncs to another page and survives reload", async ({
  page,
  context,
}) => {
  await openApp(page, "settings/appearance");
  const other = await context.newPage();
  await other.goto("/app/");
  await expect(other.getByRole("complementary", { name: "Conversations" })).toBeVisible();

  const saved = page.waitForResponse(
    (response) =>
      response.request().method() === "PATCH" && response.url().endsWith("/settings/appearance"),
  );

  await page
    .getByRole("group", { name: "Colour palette" })
    .getByRole("radio", { name: "Ocean" })
    .check();
  const response = await saved;
  expect(response.ok()).toBe(true);
  expect(response.request().postDataJSON()).toMatchObject({
    appearancePreferences: { palette: "ocean" },
  });
  await expect(other.locator("html")).toHaveAttribute("data-palette", "ocean");
  await page.evaluate(() => localStorage.removeItem("smartfire.appearance"));
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-palette", "ocean");

  await page.getByLabel("Personal appearance on this device").selectOption("device");
  await page
    .getByRole("group", { name: "Colour palette" })
    .getByRole("radio", { name: "Ember" })
    .check();
  await expect(page.locator("html")).toHaveAttribute("data-palette", "ember");
  await expect(other.locator("html")).toHaveAttribute("data-palette", "ember");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-palette", "ember");
  await page.getByLabel("Personal appearance on this device").selectOption("account");
  await expect(page.locator("html")).toHaveAttribute("data-palette", "ocean");
  await other.close();
});
