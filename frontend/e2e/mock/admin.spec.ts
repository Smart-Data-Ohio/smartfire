import type { Page } from "@playwright/test";
import { expect, matrix, openApp, shot, test } from "./support.ts";

/** Opens an admin section (`""` for the workspace) with motion reduced. */
async function openAdmin(page: Page, section: string, theme: "light" | "dark" = "light") {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/admin${section === "" ? "" : `/${section}`}`);
  await page.locator(".settings-page h1").waitFor();
}

function nav(page: Page) {
  return page.getByRole("navigation", { name: "Workspace sections" });
}

/** Lets entrance animations finish so a shot is settled. */
async function settle(page: Page): Promise<void> {
  await page.mouse.move(0, 0);
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
        .map((animation) => animation.finished.catch(() => animation)),
    ),
  );
}

matrix("the admin sections", async ({ page, theme }) => {
  await openAdmin(page, "", theme);

  await expect(page.getByRole("heading", { level: 1, name: "Workspace" })).toBeVisible();
  await settle(page);
  await shot(page, "admin-workspace", theme);

  for (const [link, heading, name] of [
    ["People", "People", "admin-people"],
    ["Workspace icons", "Workspace icons", "admin-icons"],
    ["Custom styles", "Custom CSS", "admin-styles"],
    ["Audit log", "Audit log", "admin-audit-log"],
    ["Integration health", "Integration health", "admin-integrations"],
  ] as const) {
    await nav(page).getByRole("link", { name: link }).click();
    await expect(page.getByRole("heading", { level: 1, name: heading })).toBeVisible();
    await settle(page);
    await shot(page, name, theme);
  }
});

test("the user menu opens the workspace in place", async ({ page }) => {
  await openApp(page, "");

  await page.getByRole("button", { name: "Your account" }).click();
  await page.getByRole("menuitem", { name: "Workspace and people" }).click();

  await expect(page).toHaveURL(/\/app\/admin$/);
  await expect(page.getByRole("heading", { level: 1, name: "Workspace" })).toBeVisible();
  await expect(page.getByRole("complementary", { name: "Conversations" })).toBeVisible();
  await expect(page).toHaveTitle(/Workspace · Smartfire/);
});

test("renaming the workspace saves and lands in the audit log", async ({ page }) => {
  await openAdmin(page, "");

  const name = page.getByRole("textbox", { name: "Name" });

  await name.fill("Smart Data Labs");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByText("Workspace saved")).toBeVisible();

  await nav(page).getByRole("link", { name: "Audit log" }).click();
  await expect(
    page.getByRole("cell", { name: "name: Smart Data → Smart Data Labs" }),
  ).toBeVisible();
});

test("a role change and a removal update the people list", async ({ page }) => {
  await openAdmin(page, "people");

  const maya = page.locator(".admin-person", { hasText: "Maya" });

  await maya.getByRole("switch").click();
  await expect(
    page.getByRole("region", { name: "Administrators" }).getByText("maya@smartdata.example"),
  ).toBeVisible();
  // The row moved lists and remounted; its switch keeps the focus.
  await expect(maya.getByRole("switch")).toBeFocused();

  await maya.getByRole("button", { name: /Remove Maya/ }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Remove" }).click();
  await expect(page.getByText("was removed")).toBeVisible();
  await expect(page.locator(".admin-person", { hasText: "Maya" })).toHaveCount(0);
  // The removed row was the dialog's return target; a neighbouring row takes the focus.
  await expect(page.locator(".admin-person:focus, .admin-person :focus")).toHaveCount(1);
});

test("deleting an icon keeps the focus in the list", async ({ page }) => {
  await openAdmin(page, "icons");

  await page.getByRole("button", { name: "Delete :smartdata:" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Delete" }).click();

  await expect(page.getByText("No workspace icons yet.")).toBeVisible();
  await expect(page.locator(".admin-focus-root")).toBeFocused();
});

test("custom CSS waits out the password confirmation", async ({ page, request }) => {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/lapse-sudo", { headers: { "X-CSRF-Token": state.csrfToken } });
  await page.route("**/sudo/new", (route) =>
    route.fulfill({
      status: 200,
      contentType: "text/html",
      body: "<h1>Confirm your password</h1>",
    }),
  );
  await openAdmin(page, "styles");

  await page.getByRole("textbox", { name: "Custom CSS" }).fill("body { color: red; }");
  await page.getByRole("button", { name: "Save changes" }).click();
  await expect(page).toHaveURL(/\/sudo\/new$/);

  await openAdmin(page, "styles");

  await expect(page.getByRole("textbox", { name: "Custom CSS" })).toHaveValue(
    "body { color: red; }",
  );
  await expect(page.getByText("Your unsaved CSS is back")).toBeVisible();
});

test("a write that needs the password goes to the confirmation page", async ({ page, request }) => {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/lapse-sudo", { headers: { "X-CSRF-Token": state.csrfToken } });
  await page.route("**/sudo/new", (route) =>
    route.fulfill({
      status: 200,
      contentType: "text/html",
      body: "<h1>Confirm your password</h1>",
    }),
  );
  await openAdmin(page, "people");

  await page.locator(".admin-person", { hasText: "Jonah" }).getByRole("switch").click();

  await expect(page).toHaveURL(/\/sudo\/new$/);
});

test("a new icon names what's wrong, as the classic form does", async ({ page }) => {
  await openAdmin(page, "icons");

  await page.getByRole("textbox", { name: "Name" }).fill("Not Valid");
  await page.getByRole("button", { name: "Upload icon" }).click();

  await expect(page.getByText("Title can't be blank")).toBeVisible();
  await expect(page.getByText("Image must be attached")).toBeVisible();
});

test("the audit log filters by action", async ({ page }) => {
  await openAdmin(page, "audit-log");

  await page.getByLabel("Action").selectOption("user.role.change");
  await page.getByRole("button", { name: "Filter" }).click();

  await expect(page.getByText("No audit entries match these filters.")).toBeVisible();
  await page.getByRole("button", { name: "Clear" }).click();
  await expect(page.getByRole("cell", { name: "account.settings.change" })).toBeVisible();
});
