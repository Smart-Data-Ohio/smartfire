import type { APIRequestContext, Locator, Page } from "@playwright/test";
import { MOCK_TOTP } from "../../mock/s2/account.ts";
import { MOCK_PASSWORD } from "../../mock/s2/settings.ts";
import {
  CODE_RATE_ALERT,
  INVALID_TRANSFER,
  MOCK_PLAIN_EMAIL,
  MOCK_TRANSFER_ID,
  MOCK_TRANSFER_TWO_FACTOR_ID,
  MOCK_TWO_FACTOR_EMAIL,
  SIGN_IN_REJECTION,
  WRONG_CODE,
} from "../../mock/s2/sign-in.ts";
import {
  expect,
  expectNoHorizontalOverflow,
  matrix,
  shot as save,
  THEMES,
  type Theme,
  test,
} from "./support.ts";

/**
 * The SPA's signed-out pages (slice 45) at their reserved `/app/` paths: sign-in, the second
 * step and the sign-in link, against the mock's auth contracts (mock/s2/sign-in.ts). Signing in
 * leaves for the app with a full page load, which the always-signed-in mock then serves.
 */

async function configure(
  request: APIRequestContext,
  change: { readonly google?: boolean; readonly pending?: boolean },
): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/sign-in", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: change,
  });
}

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

async function landedInApp(page: Page): Promise<void> {
  await page.waitForURL(/\/app\/$/);
  await page
    .getByRole("complementary", { name: "Conversations" })
    .or(page.locator(".room-header"))
    .first()
    .waitFor();
}

matrix("signs in through the SPA page after a refusal", async ({ page, theme }) => {
  await configure(page.request, { google: true });
  await open(page, "session/new", theme);

  await expect(page.getByRole("heading", { level: 1, name: "Smart Data" })).toBeVisible();
  await expect(page.getByText("Sign in to your workspace")).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Email address" })).toBeFocused();
  await expect(page.getByRole("button", { name: "Sign in with Google" })).toBeVisible();
  await expect(page.getByText("Google sign-in for @smartdata.example accounts.")).toBeVisible();
  await expect(page.getByRole("link", { name: "Privacy Policy" })).toBeVisible();
  await expect(page.getByRole("link", { name: MOCK_TWO_FACTOR_EMAIL })).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await shot(page, "signed-out-sign-in", theme);

  const submit = page.getByRole("button", { name: "Sign in", exact: true });
  const before = await top(submit);

  await page.getByRole("textbox", { name: "Email address" }).fill(MOCK_PLAIN_EMAIL);
  await page.getByLabel("Password", { exact: true }).fill("wrong-password");
  await page.keyboard.press("Enter");

  const password = page.getByLabel("Password", { exact: true });

  await expect(page.getByText(SIGN_IN_REJECTION)).toBeVisible();
  await expect(password).toHaveAttribute("aria-invalid", "true");
  await expect(password).toBeFocused();
  await expect(password).toHaveValue("");
  expect(await top(submit)).toBe(before);
  await shot(page, "signed-out-sign-in-refused", theme);

  // The translation list beside the label opens and closes from the keyboard.
  await page.getByRole("button", { name: "Translate password" }).click();
  await expect(page.getByText("Introduce tu contraseña")).toBeVisible();
  await shot(page, "signed-out-sign-in-translations", theme);
  await page.keyboard.press("Escape");
  await expect(page.getByText("Introduce tu contraseña")).toBeHidden();

  await password.fill(MOCK_PASSWORD);
  await password.press("Enter");
  await landedInApp(page);
});

matrix("completes a TOTP challenge after a wrong code", async ({ page, theme }) => {
  await open(page, "session/new", theme);
  await page.getByRole("textbox", { name: "Email address" }).fill(MOCK_TWO_FACTOR_EMAIL);
  await page.getByLabel("Password", { exact: true }).fill(MOCK_PASSWORD);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();

  await page.waitForURL(/\/app\/two_factor\/challenge$/);

  const code = page.getByRole("textbox", { name: "Authenticator code" });

  await expect(page.getByRole("heading", { level: 1, name: "Enter your code" })).toBeVisible();
  await expect(code).toBeFocused();
  await expectNoHorizontalOverflow(page);
  await shot(page, "signed-out-challenge", theme);

  const remember = page.getByRole("checkbox", { name: "Remember this device for 30 days" });
  const before = await top(remember);

  await code.fill("000000");
  await code.press("Enter");
  await expect(page.getByText(WRONG_CODE)).toBeVisible();
  await expect(code).toHaveAttribute("aria-invalid", "true");
  expect(await top(remember)).toBe(before);
  await shot(page, "signed-out-challenge-refused", theme);

  await code.fill("limit");
  await code.press("Enter");
  await expect(page.getByText(CODE_RATE_ALERT)).toBeVisible();

  // The backup-code switch and back, from the keyboard.
  await page.getByRole("button", { name: "Use a backup code instead" }).click();
  await expect(page.getByRole("textbox", { name: "Backup code" })).toBeFocused();
  await shot(page, "signed-out-challenge-backup", theme);
  await page.getByRole("button", { name: "Use your authenticator app instead" }).click();
  await expect(code).toBeFocused();

  await code.fill(MOCK_TOTP);
  await remember.check();
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await landedInApp(page);
});

matrix("lands a sign-in link, and says when one is invalid", async ({ page, theme }) => {
  await open(page, "session/transfers/expired-link", theme);
  await expect(page.getByRole("heading", { level: 1, name: "Couldn't sign you in" })).toBeVisible();
  await expect(page.getByRole("alert")).toHaveText(INVALID_TRANSFER);
  await expectNoHorizontalOverflow(page);
  await shot(page, "signed-out-transfer-invalid", theme);

  await page.getByRole("link", { name: "Go to sign in" }).click();
  await page.waitForURL(/\/app\/session\/new$/);
  await expect(page.getByRole("textbox", { name: "Email address" })).toBeVisible();

  await page.goto(`/app/session/transfers/${MOCK_TRANSFER_TWO_FACTOR_ID}`);
  await page.waitForURL(/\/app\/two_factor\/challenge$/);
  await expect(page.getByRole("textbox", { name: "Authenticator code" })).toBeVisible();

  await page.goto(`/app/session/transfers/${MOCK_TRANSFER_ID}`);
  await landedInApp(page);
});

matrix("sends a challenge with nothing pending back to sign in", async ({ page, theme }) => {
  await configure(page.request, { pending: false });
  await open(page, "two_factor/challenge", theme);
  await page.waitForURL(/\/app\/session\/new$/);
  await expect(page.getByRole("textbox", { name: "Email address" })).toBeVisible();
});

for (const theme of THEMES) {
  test(`scrolls a short phone screen to the last control (${theme})`, async ({ page }) => {
    // A 390 px phone with its keyboard up leaves about 500 px; the app's body lock is the shell's.
    await page.setViewportSize({ width: 390, height: 500 });
    await configure(page.request, { google: true });
    await open(page, "session/new", theme);

    const help = page.getByRole("link", { name: MOCK_TWO_FACTOR_EMAIL });
    const terms = page.getByRole("link", { name: "Terms of Service" });

    await expect(help).not.toBeInViewport();

    // The wheel, as a person scrolls: no programmatic scrollIntoView.
    await page.mouse.move(195, 250);
    await page.mouse.wheel(0, 2000);
    await expect(help).toBeInViewport({ ratio: 1 });
    await expect(terms).toBeInViewport({ ratio: 1 });
    await expect(page.getByRole("button", { name: "Sign in with Google" })).toBeInViewport();
    await expectNoHorizontalOverflow(page);
    await shot(page, "signed-out-sign-in-scrolled", theme);

    const popup = page.waitForEvent("popup");

    await terms.click();
    await (await popup).close();
  });
}
