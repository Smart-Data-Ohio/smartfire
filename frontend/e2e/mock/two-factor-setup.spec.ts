import type { Page } from "@playwright/test";
import { expect, expectNoHorizontalOverflow, matrix, openApp, shot, test } from "./support.ts";

/** Sets the mock account's two-step state (`/__mock/two-factor`). */
async function twoFactor(
  page: Page,
  change: { readonly enrolled?: boolean; readonly required?: boolean },
): Promise<void> {
  const state = await (await page.request.get("/__mock/state")).json();

  const response = await page.request.post("/__mock/two-factor", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: change,
  });

  expect(response.ok()).toBe(true);
}

/** Lets the card's entrance finish so a shot is settled. */
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

/** Enters a code and verifies it. */
async function verify(page: Page, code: string): Promise<void> {
  await page.getByRole("textbox", { name: "Authenticator code" }).fill(code);
  await page.getByRole("button", { name: "Verify and continue" }).click();
}

/** The card's place, to check that a refusal moves nothing below the field. */
async function verifyButtonTop(page: Page): Promise<number> {
  const box = await page.getByRole("button", { name: "Verify and continue" }).boundingBox();

  return box?.y ?? Number.NaN;
}

matrix("optional setup from Settings returns to Security", async ({ page, theme, phone }) => {
  await twoFactor(page, { enrolled: false });
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto("/app/settings/security");
  await page.getByRole("link", { name: "Set up two-step sign-in" }).click();

  await expect(page).toHaveURL(/\/app\/two_factor\/setup\?return_to=/);

  const qr = page.getByRole("img", { name: "QR code for your authenticator app" });

  await expect(qr).toBeVisible();
  await expect(page.getByRole("button", { name: "Back to settings" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Open in your authenticator app" })).toBeVisible({
    visible: phone,
  });
  await expectNoHorizontalOverflow(page);
  await settle(page);
  await shot(page, "two-factor-setup", theme);

  const before = await verifyButtonTop(page);

  await verify(page, "000000");
  await expect(page.getByText("That code didn't work.", { exact: false })).toBeVisible();
  expect(await verifyButtonTop(page)).toBe(before);
  await shot(page, "two-factor-setup-wrong", theme);

  await verify(page, "123 456");

  const codes = page.getByRole("list", { name: "Backup codes" });

  await expect(page.getByRole("heading", { name: "Save your backup codes" })).toBeFocused();
  await expect(codes.getByRole("listitem")).toHaveCount(10);
  await expectNoHorizontalOverflow(page);
  await settle(page);
  await shot(page, "two-factor-setup-codes", theme);

  const download = page.waitForEvent("download");

  await page.getByRole("button", { name: "Download" }).click();
  expect((await download).suggestedFilename()).toBe("smartfire-backup-codes.txt");

  await page.getByRole("button", { name: "I've saved these codes" }).click();
  await expect(page).toHaveURL(/\/app\/settings\/security$/);
  await expect(page.getByRole("button", { name: "Turn off" })).toBeVisible();
});

matrix("required setup takes over the app and returns to the room", async ({ page, theme }) => {
  await twoFactor(page, { enrolled: false, required: true });
  await openApp(page, "/r/1", theme);

  await expect(page).toHaveURL(/\/app\/two_factor\/setup\?.*required=true/);
  await expect(page.getByRole("img", { name: "QR code for your authenticator app" })).toBeVisible();
  // No way around it but sign out.
  await expect(page.getByRole("button", { name: "Back to settings" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await settle(page);
  await shot(page, "two-factor-setup-required", theme);

  await verify(page, "123456");
  await page.getByRole("button", { name: "I've saved these codes" }).click();
  await expect(page).toHaveURL(/\/app\/r\/1$/);
});

test("sign out stays available while setup is required", async ({ page }) => {
  await twoFactor(page, { enrolled: false, required: true });
  await openApp(page, "/r/1");
  await expect(page.getByRole("img", { name: "QR code for your authenticator app" })).toBeVisible();

  const signOut = page.waitForRequest(
    (request) => request.method() === "DELETE" && request.url().endsWith("/api/v1/session"),
  );

  await page.getByRole("button", { name: "Sign out" }).click();
  await signOut;
  await expect(page).not.toHaveURL(/two_factor/);
});

matrix("a full load of the setup screen while setup is required", async ({ page, theme }) => {
  await twoFactor(page, { enrolled: false, required: true });
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto("/app/two_factor/setup?return_to=%2Fapp%2Fr%2F1&required=true");

  await expect(page.getByRole("heading", { name: "Set up two-step sign-in" })).toBeVisible();
  await expect(page.getByRole("img", { name: "QR code for your authenticator app" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Back to settings" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await settle(page);
  await shot(page, "two-factor-setup-direct", theme);

  await verify(page, "123456");
  await page.getByRole("button", { name: "I've saved these codes" }).click();
  await expect(page).toHaveURL(/\/app\/r\/1$/);
});
