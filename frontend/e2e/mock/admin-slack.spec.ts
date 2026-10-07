import type { Page } from "@playwright/test";
import { expect, matrix, shot, test } from "./support.ts";

/** A run page reads its status every five seconds; two reads settle a mock run. */
const SETTLED = { timeout: 20_000 };

/** Opens a page under `/app` with motion reduced. */
async function open(page: Page, path: string, theme: "light" | "dark" = "light") {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.locator(".settings-page h1").waitFor();
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

/** Answers the confirmation dialog titled `name`. */
async function confirm(page: Page, name: string) {
  await page.getByRole("alertdialog", { name }).getByRole("button", { name }).click();
}

const facts = (page: Page) => page.locator(".slack-facts");

matrix("the Slack import pages", async ({ page, theme }) => {
  await open(page, "admin/slack", theme);
  await expect(page.getByRole("heading", { level: 1, name: "Slack import" })).toBeVisible();
  await settle(page);
  await shot(page, "admin-slack", theme);

  await open(page, "admin/slack/runs/1/plan", theme);
  // In view, not scrolled off the table's end.
  await expect(page.getByRole("combobox", { name: "Target for general" })).toBeInViewport();
  await settle(page);
  await shot(page, "admin-slack-plan", theme);

  await open(page, "settings/slack", theme);
  await expect(page.getByRole("heading", { level: 1, name: "Import from Slack" })).toBeVisible();
  await settle(page);
  await shot(page, "settings-slack", theme);
});

test("the workspace nav and the integrations card open the Slack pages", async ({ page }) => {
  await page.goto("/app/admin");
  await page
    .getByRole("navigation", { name: "Workspace sections" })
    .getByRole("link", { name: "Slack import" })
    .click();
  await expect(page).toHaveURL(/\/app\/admin\/slack$/);
  await expect(page.getByText("Credentials saved.", { exact: false })).toBeVisible();

  await open(page, "settings/integrations");
  await page.getByRole("link", { name: "Import from Slack" }).click();
  await expect(page).toHaveURL(/\/app\/settings\/slack$/);
});

test("a dry run shows its progress live, then offers the plan", async ({ page }) => {
  await open(page, "admin/slack");

  await page.getByRole("button", { name: "Start dry run" }).click();

  await expect(page).toHaveURL(/\/app\/admin\/slack\/runs\/2$/);
  await expect(page.getByText("Dry run started.")).toBeVisible();
  await expect(facts(page)).toContainText("running", SETTLED);
  await expect(facts(page)).toContainText("completed", SETTLED);

  await page.getByRole("link", { name: "Review the plan" }).click();
  await expect(page).toHaveURL(/\/app\/admin\/slack\/runs\/2\/plan$/);
});

test("a full import from the plan, then undoing it", async ({ page }) => {
  await open(page, "admin/slack/runs/1/plan");

  await page.getByRole("checkbox", { name: "Import design" }).uncheck();
  await page.getByRole("button", { name: "Full import" }).click();
  await confirm(page, "Full import");

  await expect(page).toHaveURL(/\/app\/admin\/slack\/runs\/2$/);
  await expect(page.getByText("Full import started.")).toBeVisible();
  await expect(facts(page)).toContainText("completed", SETTLED);
  await expect(facts(page)).toContainText("428");

  await page.getByRole("button", { name: "Undo import" }).click();
  await confirm(page, "Undo import");

  await expect(page.getByText("Undo started.")).toBeVisible();
  // The button went away, so focus goes to the run's status.
  await expect(page.getByRole("region", { name: "Run status" })).toBeFocused();
  await expect(facts(page)).toContainText("undone", SETTLED);
  await expect(page.getByRole("button", { name: "Undo import" })).toHaveCount(0);
});

test("an import needs a checked conversation", async ({ page }) => {
  await open(page, "admin/slack/runs/1/plan");

  await page.getByRole("button", { name: "Select none" }).click();

  await expect(page.getByRole("button", { name: "Full import" })).toBeDisabled();
  await expect(page.getByRole("button", { name: "Test import" })).toBeDisabled();

  await page.getByRole("checkbox", { name: "Import design" }).check();
  await expect(page.getByRole("button", { name: "Full import" })).toBeEnabled();
});

test("an undo waits for a later import of the same conversations", async ({ page }) => {
  await open(page, "admin/slack/runs/1/plan");
  await page.getByRole("button", { name: "Full import" }).click();
  await confirm(page, "Full import");
  await expect(page).toHaveURL(/\/app\/admin\/slack\/runs\/2$/);
  await expect(facts(page)).toContainText("completed", SETTLED);

  await page.getByRole("button", { name: "Run catch-up import" }).click();
  await confirm(page, "Run catch-up import");
  await expect(page).toHaveURL(/\/app\/admin\/slack\/runs\/3$/);
  await expect(facts(page)).toContainText("completed", SETTLED);

  await open(page, "admin/slack/runs/2");

  const undo = page.getByRole("button", { name: "Undo import" });

  await expect(undo).toBeDisabled();
  await expect(undo).toHaveAccessibleDescription(
    "A later import (#3) also imported some of these conversations; undo that one first.",
  );
});

test("a person previews their conversations and imports the checked ones", async ({ page }) => {
  await open(page, "settings/slack");

  await page.getByRole("button", { name: "Start preview" }).click();

  await expect(page).toHaveURL(/\/app\/settings\/slack\/2$/);
  await expect(page.getByRole("heading", { name: "Your plan" })).toBeVisible(SETTLED);

  await page.getByRole("checkbox", { name: "Import grace, alan, you" }).uncheck();
  await page.getByRole("button", { name: "Import checked" }).click();
  await confirm(page, "Import checked");

  await expect(page).toHaveURL(/\/app\/settings\/slack\/3$/);
  await expect(page.getByText("Import started.")).toBeVisible();
  await expect(facts(page)).toContainText("completed", SETTLED);

  await page.getByRole("link", { name: "Your imports" }).click();
  await expect(page.getByRole("link", { name: "#3" })).toBeVisible();
  await expect(page.getByRole("link", { name: "#2" })).toBeVisible();
});
