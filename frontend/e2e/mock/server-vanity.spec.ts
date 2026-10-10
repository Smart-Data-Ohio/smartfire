import { expect, test } from "./support.ts";

test("sets the workspace description and vanity invite URL", async ({ page }) => {
  await page.goto("/app/admin/workspace");
  await page.getByRole("textbox", { name: "Description" }).fill("A place for Smart Data.");
  await page.getByRole("button", { name: "Save description" }).click();
  await expect(page.getByRole("region", { name: "About Smart Data" })).toContainText(
    "A place for Smart Data.",
  );
  await page.getByRole("textbox", { name: "Vanity slug" }).fill("smart-data");
  await page.getByRole("button", { name: "Save vanity slug" }).click();
  await expect(page.getByText("Vanity invite saved", { exact: true })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Vanity invite URL" })).toHaveValue(
    /\/join\/smart-data$/,
  );
  await expect(page.getByRole("button", { name: "Copy vanity invite" })).toBeEnabled();
  await page.reload();
  await expect(page.getByRole("textbox", { name: "Description" })).toHaveValue(
    "A place for Smart Data.",
  );
  await expect(page.getByRole("textbox", { name: "Vanity slug" })).toHaveValue("smart-data");
});
