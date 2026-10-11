import type { Page } from "@playwright/test";
import { MOCK_DEAD_INVITES } from "../../mock/s2/join.ts";
import { MOCK_TWO_FACTOR_EMAIL } from "../../mock/s2/sign-in.ts";
import {
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  matrix,
  PHONE_TOUCH,
  shot,
  type Theme,
  test,
} from "./support.ts";

/**
 * Private invites (slice 50): the administrator's invites section at `/app/admin/invites` against
 * the mock's invites module (mock/s2/invites.ts), and the invite page's states for an invite that
 * can't be used. An invite made here works on the invite page until it is revoked.
 */

async function openInvites(page: Page, theme: Theme): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto("/app/admin/invites");
  await page.locator(".settings-page h1").waitFor();
  await table(page).waitFor();
}

async function openInvite(page: Page, token: string, theme: Theme): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/invite/${token}`);
  await page.getByRole("main").waitFor();
  await page.evaluate(() => document.fonts.ready);
}

function table(page: Page) {
  return page.getByRole("table", { name: "Invites, newest first" });
}

/** The invite rows, newest first (the header row left out). */
function rows(page: Page) {
  return table(page).locator("tbody").getByRole("row");
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

matrix("creates an invite, copies its link once, and revokes it", async ({ page, theme }) => {
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  await openInvites(page, theme);

  await expect(page.getByRole("heading", { level: 1, name: "Invites" })).toBeAttached();
  await expect(rows(page)).toHaveCount(4);
  await expect(rows(page).first()).toContainText("3 of 10");
  await expect(
    rows(page)
      .first()
      .getByRole("button", { name: /^Revoke/ }),
  ).toBeVisible();
  // Dead invites say why, and can't be revoked again.
  await expect(rows(page).nth(1)).toContainText("Used up");
  await expect(rows(page).nth(2)).toContainText("Expired");
  await expect(rows(page).nth(3)).toContainText("Revoked");
  await expect(rows(page).nth(1).getByRole("button")).toHaveCount(0);
  await expectNoHorizontalOverflow(page);
  await settle(page);
  await shot(page, "admin-invites", theme);

  await page.getByRole("combobox", { name: "Expire after" }).selectOption("1d");
  await page.getByRole("combobox", { name: "Max number of uses" }).selectOption("1");
  await expect(page.getByText("Expires in 1 day. Works for 1 person.")).toBeVisible();
  await page.getByRole("button", { name: "Create invite link" }).click();

  const dialog = page.getByRole("alertdialog", { name: "Your invite link" });
  const link = dialog.getByText(/^http:\/\/127\.0\.0\.1\/invite\/[0-9a-f]{64}$/);

  await expect(link).toBeVisible();

  const url = (await link.textContent()) ?? "";
  const token = url.split("/").at(-1) ?? "";

  await dialog.getByRole("button", { name: "Copy link" }).click();
  await expect(dialog.getByRole("button", { name: "Copied" })).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(url);
  await expectNoHorizontalOverflow(page);
  await settle(page);
  await shot(page, "admin-invite-link", theme);

  await dialog.getByRole("button", { name: "Done" }).click();
  await expect(dialog).toBeHidden();

  // Listed first, live, and its link is gone for good.
  await expect(rows(page)).toHaveCount(5);
  await expect(rows(page).first()).toContainText("0 of 1");
  await expect(rows(page).first()).toContainText("Active");
  await expect(page.getByText(url)).toHaveCount(0);

  // The link works.
  await openInvite(page, token, theme);
  await expect(page.getByRole("heading", { level: 1, name: "Join Smart Data" })).toBeVisible();

  await openInvites(page, theme);
  await rows(page)
    .first()
    .getByRole("button", { name: /^Revoke/ })
    .click();

  const ask = page.getByRole("alertdialog", { name: "Revoke this invite?" });

  await expect(ask).toContainText("No one has joined with it yet.");
  await ask.getByRole("button", { name: "Revoke invite" }).click();
  await expect(ask).toBeHidden();
  await expect(rows(page).first()).toContainText("Revoked");
  await expect(
    rows(page)
      .first()
      .getByRole("button", { name: /^Revoke/ }),
  ).toHaveCount(0);

  // Revoked invites, on their own.
  await page.getByRole("tab", { name: "Revoked" }).click();
  await expect(rows(page)).toHaveCount(2);

  // The link no longer works, and says so.
  await openInvite(page, token, theme);
  await expect(
    page.getByRole("heading", { level: 1, name: "This invite was turned off" }),
  ).toBeVisible();
});

matrix("points to the shared join link on the workspace page", async ({ page, theme }) => {
  await openInvites(page, theme);
  await page
    .getByRole("link", { name: "The join link and vanity invite are on the Workspace page" })
    .click();
  await expect(page).toHaveURL(/\/app\/admin\/workspace$/);
  await expect(page.getByRole("button", { name: "Copy join link" })).toBeVisible();

  await page.getByRole("link", { name: "create a private invite" }).click();
  await expect(page).toHaveURL(/\/app\/admin\/invites$/);
});

for (const [refusal, token, title, next] of [
  [
    "expired",
    MOCK_DEAD_INVITES.expired,
    "This invite has expired",
    "Ask the person who sent it for a fresh link.",
  ],
  [
    "exhausted",
    MOCK_DEAD_INVITES.exhausted,
    "This invite is full",
    "Ask the person who sent it for a new link.",
  ],
  [
    "revoked",
    MOCK_DEAD_INVITES.revoked,
    "This invite was turned off",
    "If you expected to join, ask them for a new invite.",
  ],
  [
    "unknown",
    "never-issued",
    "We couldn't find this invite",
    "Check that you copied the whole link, or ask for a new one.",
  ],
] as const) {
  matrix(`a ${refusal} invite says why and what to do next`, async ({ page, theme }) => {
    await openInvite(page, token, theme);

    await expect(page.getByRole("heading", { level: 1, name: title })).toBeVisible();
    await expect(page.getByText(next)).toBeVisible();
    await expect(page.getByRole("textbox", { name: "Name" })).toHaveCount(0);
    await expect(page.getByRole("link", { name: "Email Riel" })).toHaveAttribute(
      "href",
      new RegExp(`^mailto:${MOCK_TWO_FACTOR_EMAIL.replace(".", "\\.")}\\?subject=`),
    );
    await expect(page.getByRole("link", { name: "Sign in" })).toBeVisible();
    await expectNoHorizontalOverflow(page);
    await settle(page);
    await shot(page, `join-invite-${refusal}`, theme);
  });
}

test.describe("on a phone", () => {
  test.use(PHONE_TOUCH);

  test("invites are cards a thumb can work", async ({ page }) => {
    await openInvites(page, "light");

    await expect(rows(page)).toHaveCount(4);
    await expectNoHorizontalOverflow(page);
    await expectTouchTargets(page, ".admin-invites tbody");
    await expectTouchTargets(page, ".admin-invite-form");

    await page.getByRole("button", { name: "Create invite link" }).click();

    const dialog = page.getByRole("alertdialog", { name: "Your invite link" });

    await expect(dialog.getByRole("button", { name: "Copy link" })).toBeVisible();
    await expectNoHorizontalOverflow(page);
    await expectTouchTargets(page, ".admin-invite-link");
  });
});
