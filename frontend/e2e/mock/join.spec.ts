import { readFileSync } from "node:fs";
import type { Locator, Page } from "@playwright/test";
import {
  INVITE_REASONS,
  MOCK_DEAD_INVITES,
  MOCK_INVITE,
  MOCK_JOIN_CODE,
  type MockJoined,
} from "../../mock/s2/join.ts";
import { MOCK_PLAIN_EMAIL, MOCK_TWO_FACTOR_EMAIL } from "../../mock/s2/sign-in.ts";
import { expect, expectNoHorizontalOverflow, matrix, shot as save, type Theme } from "./support.ts";

/**
 * Joining the workspace through the SPA (slice 46): the join code's page and an invite's page at
 * their reserved `/app/` paths, against the mock's join contracts (mock/s2/join.ts). Joining
 * leaves for the app with a full page load, which the always-signed-in mock then serves.
 */

const AVATAR = {
  name: "me.png",
  mimeType: "image/png",
  buffer: readFileSync(
    new URL("../../../fixtures/files/workspace_icons/square_64.png", import.meta.url),
  ),
};

async function open(page: Page, path: string, theme: Theme): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("main").waitFor();
  await page.evaluate(() => document.fonts.ready);
}

/** A shot once entrance animations have finished, with the pointer out of the way. */
async function shot(page: Page, name: string, theme: Theme): Promise<void> {
  await page.mouse.move(0, 0);
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
        .map((animation) => animation.finished.catch(() => animation)),
    ),
  );
  await save(page, name, theme);
}

/** The top of `locator`, to show that a message appearing above it moves nothing. */
async function top(locator: Locator): Promise<number> {
  return (await locator.boundingBox())?.y ?? Number.NaN;
}

async function joined(page: Page): Promise<readonly MockJoined[]> {
  const state = await (await page.request.get("/__mock/state")).json();

  return state.joined;
}

async function landedInApp(page: Page): Promise<void> {
  await page.waitForURL(/\/app\/$/);
  await page
    .getByRole("complementary", { name: "Conversations" })
    .or(page.locator(".room-header"))
    .first()
    .waitFor();
}

matrix("joins with the join code after fixing the form", async ({ page, theme }) => {
  await open(page, `join/${MOCK_JOIN_CODE}`, theme);

  const name = page.getByRole("textbox", { name: "Name" });
  const email = page.getByRole("textbox", { name: "Email address" });
  const password = page.getByLabel("Password", { exact: true });
  const submit = page.getByRole("button", { name: "Create account" });

  await expect(page.getByRole("heading", { level: 1, name: "Join Smart Data" })).toBeVisible();
  await expect(name).toBeFocused();
  await expect(page.getByRole("link", { name: "Sign in" })).toBeVisible();
  await expect(page.getByRole("link", { name: MOCK_TWO_FACTOR_EMAIL })).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await shot(page, "join-code", theme);

  // Errors land in each field's reserved line: nothing below them moves.
  const before = await top(submit);

  await page.keyboard.press("Enter");
  await expect(page.getByText("Enter your name.")).toBeVisible();
  await expect(page.getByText("Enter your email address.")).toBeVisible();
  await expect(page.getByText("Enter a password.")).toBeVisible();
  await expect(name).toHaveAttribute("aria-invalid", "true");
  await expect(name).toBeFocused();
  expect(await top(submit)).toBe(before);
  await shot(page, "join-code-refused", theme);

  // The avatar picker comes before the fields in keyboard order (the name's translations first).
  await page.keyboard.press("Shift+Tab");
  await expect(page.getByRole("button", { name: "Translate name" })).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(page.getByLabel("Upload avatar")).toBeFocused();
  await page.getByLabel("Upload avatar").setInputFiles(AVATAR);
  await expect(page.getByLabel("Change avatar, me.png chosen")).toBeAttached();
  await expect(page.locator(".auth-view-avatar-preview")).toBeVisible();

  await name.fill("Ada Lovelace");
  await email.fill("ada@smartdata.example");
  await password.fill("secret123456");
  await expectNoHorizontalOverflow(page);
  await shot(page, "join-code-filled", theme);

  await password.press("Enter");
  await landedInApp(page);

  expect(await joined(page)).toEqual([
    { name: "Ada Lovelace", emailAddress: "ada@smartdata.example", avatar: "me.png" },
  ]);
});

matrix("joins with an invite", async ({ page, theme }) => {
  await open(page, `invite/${MOCK_INVITE}`, theme);
  await expect(page.getByRole("heading", { level: 1, name: "Join Smart Data" })).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await shot(page, "join-invite", theme);

  await page.getByRole("textbox", { name: "Name" }).fill("Grace Hopper");
  await page.getByRole("textbox", { name: "Email address" }).fill("grace@smartdata.example");
  await page.getByLabel("Password", { exact: true }).fill("secret123456");
  await page.getByRole("button", { name: "Create account" }).click();
  await landedInApp(page);

  expect(await joined(page)).toEqual([
    { name: "Grace Hopper", emailAddress: "grace@smartdata.example", avatar: null },
  ]);
});

matrix("says why an invite can't be used, and links to sign in", async ({ page, theme }) => {
  await open(page, `invite/${MOCK_DEAD_INVITES.expired}`, theme);
  await expect(page.getByRole("alert")).toHaveText("This invite is no longer valid.");
  await expect(
    page.getByText(`${INVITE_REASONS.expired} Ask a workspace administrator for a new invite.`),
  ).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Name" })).toBeHidden();
  await expectNoHorizontalOverflow(page);
  await shot(page, "join-invite-expired", theme);

  await open(page, "join/wrong-code", theme);
  await expect(page.getByText("This join link isn't valid.")).toBeVisible();
  await shot(page, "join-code-wrong", theme);

  await page.getByRole("link", { name: "Sign in" }).click();
  await page.waitForURL(/\/app\/session\/new$/);
  await expect(page.getByRole("textbox", { name: "Email address" })).toBeVisible();
});

matrix("sends an address that has an account to sign in", async ({ page, theme }) => {
  await open(page, `join/${MOCK_JOIN_CODE}`, theme);
  await page.getByRole("textbox", { name: "Name" }).fill("Theo");
  await page.getByRole("textbox", { name: "Email address" }).fill(MOCK_PLAIN_EMAIL);
  await page.getByLabel("Password", { exact: true }).fill("secret123456");
  await page.getByRole("button", { name: "Create account" }).click();

  await page.waitForURL(/\/app\/session\/new\?email_address=/);
  await expect(page.getByRole("textbox", { name: "Email address" })).toHaveValue(MOCK_PLAIN_EMAIL);
  expect(await joined(page)).toEqual([]);
});
