import { expect, openApp, test, USER_IDS } from "./support.ts";

test("profile identity saves, reloads and clears to the account name", async ({ page }) => {
  await openApp(page, "settings/profile");
  const accountName = await page.getByRole("textbox", { name: "Name", exact: true }).inputValue();
  const nickname = page.getByRole("textbox", { name: "Display nickname" });
  const pronouns = page.getByRole("textbox", { name: "Pronouns" });

  await nickname.fill("  R  ");
  await pronouns.fill("  they/them  ");
  await page.getByRole("button", { name: "Save profile" }).click();
  await expect(nickname).toHaveValue("R");
  await expect(pronouns).toHaveValue("they/them");
  await page.reload();
  await expect(nickname).toHaveValue("R");
  await expect(pronouns).toHaveValue("they/them");
  await expect(page.getByRole("textbox", { name: "Name", exact: true })).toHaveValue(accountName);
  await nickname.fill("");
  await pronouns.fill("");
  await page.getByRole("button", { name: "Save profile" }).click();
  await expect(page.getByRole("button", { name: "Save profile" })).toBeDisabled();
  await page.reload();
  await expect(nickname).toHaveValue("");
  await expect(pronouns).toHaveValue("");
});

test("person identity shows muted pronouns beside the nickname and the account name below", async ({
  page,
}) => {
  await page.route(`**/api/v1/people/${USER_IDS.maya}`, async (route) => {
    const response = await route.fetch();
    const profile = await response.json();

    profile.user = {
      ...profile.user,
      name: "Maya",
      accountName: "Maya Okafor",
      pronouns: "she/her",
      updatedAt: new Date(Date.parse(profile.user.updatedAt) + 1000)
        .toISOString()
        .replace("Z", "000Z"),
    };
    await route.fulfill({ response, json: profile });
  });
  await openApp(page, `people/${USER_IDS.maya}`);
  await expect(page.getByRole("heading", { name: "Maya", exact: true })).toBeVisible();
  await expect(page.locator(".page-header .text-muted")).toHaveText("she/her");
  await expect(page.locator(".people-page > .text-muted")).toHaveText("Maya Okafor");
  await expect(page.getByRole("button", { name: "Message Maya" })).toBeVisible();
});
