import type { Page } from "@playwright/test";
import {
  expect,
  matrix,
  openApp,
  SHOTS,
  saveAccountAppearance,
  shot,
  type Theme,
  test,
} from "./support.ts";

/**
 * The server-rendered pages outside the SPA (sign-in, joining, two-step sign-in, password
 * confirmation, the sign-in link, first run, the public pages), as the Rust templates render them:
 * a Rust test writes each one to e2e/fixtures/auth-pages with its stylesheet and script, and the
 * mock dev server serves that directory at /__auth/ untouched.
 */

interface AuthPage {
  readonly name: string;
  /** The page's main heading. */
  readonly heading: RegExp;
}

const PAGES: readonly AuthPage[] = [
  { name: "sign-in", heading: /^Signal$/ },
  { name: "sign-in-google-alert", heading: /^Signal$/ },
  { name: "join", heading: /^Join Signal$/ },
  { name: "invite", heading: /^Join Signal$/ },
  { name: "invite-expired", heading: /^Join Signal$/ },
  { name: "invite-exhausted", heading: /^Join Signal$/ },
  { name: "invite-revoked", heading: /^Join Signal$/ },
  { name: "invite-unknown", heading: /^Join Signal$/ },
  { name: "first-run", heading: /^Set up Smartfire$/ },
  { name: "two-factor-setup", heading: /^Set up two-step sign-in$/ },
  { name: "two-factor-challenge-alert", heading: /^Enter your code$/ },
  { name: "backup-codes", heading: /^Save your backup codes$/ },
  { name: "sudo-all", heading: /^Confirm it's you$/ },
  { name: "sudo-totp", heading: /^Confirm it's you$/ },
  { name: "sudo-continue", heading: /^Confirmed — continuing$/ },
  { name: "transfer", heading: /^Signing you in$/ },
  { name: "about", heading: /^About Smartfire$/ },
  { name: "privacy", heading: /^Privacy Policy$/ },
  { name: "terms", heading: /^Terms of Service$/ },
];

/** Where the two self-submitting pages post. */
const SELF_SUBMITTING = /\/(session\/transfers|account\/users)\//;

for (const [state, reason] of [
  ["expired", "It has expired."],
  ["exhausted", "All its uses have been taken."],
  ["revoked", "It has been revoked."],
  ["unknown", "The invite could not be found."],
] as const) {
  test(`a ${state} invite explains why enrollment is unavailable`, async ({ page }) => {
    await page.goto(`/__auth/invite-${state}.html`);
    await expect(page.getByRole("alert")).toHaveText("This invite is no longer valid.");
    await expect(page.getByText(reason, { exact: false })).toBeVisible();
    await expect(page.getByRole("button", { name: "Create account" })).toHaveCount(0);
    await expect(page.locator("form")).toHaveCount(0);
    await expect(page.getByRole("link", { name: "Sign in", exact: true })).toHaveAttribute(
      "href",
      "/session/new",
    );
  });
}

test("an active invite keeps enrollment on its own token route", async ({ page }) => {
  await page.goto("/__auth/invite.html");
  await expect(page.getByRole("heading", { name: "Join Signal" })).toBeVisible();
  await expect(page.locator("form")).toHaveAttribute("action", "/invite/fixture-invite-token");
  await expect(page.locator("form")).toHaveAttribute("method", "post");
  await expect(page.getByRole("button", { name: "Create account" })).toBeVisible();
});

/**
 * Opens a fixture page in `theme` with motion reduced and waits for the fonts and every finite
 * animation. The two pages that submit themselves stay put: their submission would cancel the rest
 * of the page's loading, fonts included.
 */
async function openPage(page: Page, name: string, theme: Theme): Promise<void> {
  await page.addInitScript(() => {
    HTMLFormElement.prototype.requestSubmit = function requestSubmit() {
      this.dataset.autoSubmitted = "true";
    };
  });
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/__auth/${name}.html`);
  await page.evaluate(async () => {
    await document.fonts.ready;
    await Promise.all(
      document
        .getAnimations()
        .flatMap((animation) =>
          animation.effect?.getComputedTiming().iterations === Infinity
            ? []
            : [animation.finished.catch(() => animation)],
        ),
    );
  });
}

/**
 * Opens a page that submits itself and returns the form it posts. The post is answered with
 * 204 No Content, so the browser stays on the page.
 */
async function selfSubmitted(page: Page, name: string): Promise<URLSearchParams> {
  await page.route(SELF_SUBMITTING, (route) => route.fulfill({ status: 204 }));

  const submitted = page.waitForRequest(
    (request) => SELF_SUBMITTING.test(request.url()) && request.method() === "POST",
  );

  // Up to commit only: the page's own submission never commits, and `goto` would wait on it.
  await page.goto(`/__auth/${name}.html`, { waitUntil: "commit" });

  return new URLSearchParams((await submitted).postData() ?? "");
}

// Screenshots only: the Rust fixture test pins each page's HTML, and the tests below drive them.
if (SHOTS) {
  for (const { name, heading } of PAGES) {
    matrix(`the ${name} page in the SPA's look`, async ({ page, theme }) => {
      await openPage(page, name, theme);
      await expect(page.getByRole("heading", { level: 1, name: heading })).toBeVisible();
      await page.mouse.move(0, 0);
      await shot(page, `auth-${name}`, theme);
    });
  }
}

test("a pinned system theme follows the OS over the account's theme", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.addInitScript(() =>
    localStorage.setItem(
      "smartfire.appearance",
      JSON.stringify({ themeOverride: "system", density: "comfortable", motion: "system" }),
    ),
  );
  // As a signed-in page whose account theme is dark.
  await page.route("**/__auth/sudo-totp.html", async (route) => {
    const response = await route.fetch();
    const body = (await response.text()).replace(/data-theme="[a-z]+"/, 'data-theme="dark"');

    await route.fulfill({ response, body });
  });
  await page.goto("/__auth/sudo-totp.html");

  await expect(page.locator("html")).not.toHaveAttribute("data-theme", /./);

  const background = () => page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  const light = await background();

  await page.emulateMedia({ colorScheme: "dark" });

  expect(await background()).not.toBe(light);
});

/** Serves the password-confirmation page as the server renders it for an account in `theme`. */
async function sudoPageFor(page: Page, theme: Theme): Promise<void> {
  await page.route("**/__auth/sudo-totp.html", async (route) => {
    const response = await route.fetch();
    const body = (await response.text()).replace(/data-theme="[a-z]+"/, `data-theme="${theme}"`);

    await route.fulfill({ response, body });
  });
}

test("one person's account theme never follows the next person onto these pages", async ({
  page,
}) => {
  // A uses dark on their account, with nothing pinned on this device.
  await openApp(page, "");
  await saveAccountAppearance(page, { theme: "dark" });
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");

  // A signs out: the sign-in page follows the OS (light).
  await page.goto("/__auth/sign-in.html");
  await expect(page.locator("html")).not.toHaveAttribute("data-theme", "dark");

  // B, light on their account, signs in and confirms their password.
  await sudoPageFor(page, "light");
  await page.goto("/__auth/sudo-totp.html");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
});

test("a theme pinned on this device stays through sign-out", async ({ page }) => {
  await openApp(page, "settings/appearance");
  await page.getByLabel("Theme on this device").selectOption({ label: "Dark" });
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");

  await page.goto("/__auth/sign-in.html");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");

  await sudoPageFor(page, "light");
  await page.goto("/__auth/sudo-totp.html");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
});

test("sign-in shows a rejected attempt above the card and shakes it", async ({ page }) => {
  await openPage(page, "sign-in-google-alert", "light");

  await expect(page.getByRole("alert")).toHaveText("Too many requests or unauthorized.");
  await expect(page.locator(".auth-card")).toHaveAttribute("data-shake", "");

  for (const field of [page.getByLabel("Email address"), page.getByLabel("Password")]) {
    await expect(field).toHaveAttribute("aria-invalid", "true");
    await expect(field).toHaveAccessibleDescription("Too many requests or unauthorized.");
  }

  await expect(page.getByRole("button", { name: "Sign in with Google" })).toBeVisible();
});

test("a field's translations open from the globe and close on Escape or a click elsewhere", async ({
  page,
}) => {
  await openPage(page, "sign-in", "light");

  const list = page.locator("details.auth-translate").first();

  await list.locator("summary").click();
  await expect(list).toHaveAttribute("open", "");
  await expect(list.getByText("Introduce tu correo electrónico")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(list).not.toHaveAttribute("open", "");
  await list.locator("summary").click();
  await page.getByRole("heading", { level: 1 }).click();
  await expect(list).not.toHaveAttribute("open", "");
});

test("joining previews the avatar you pick before it's uploaded", async ({ page }) => {
  await openPage(page, "join", "light");

  const preview = page.locator("[data-avatar-preview]");

  await expect(preview).toBeHidden();
  await page.getByLabel("Upload avatar").setInputFiles({
    name: "me.png",
    mimeType: "image/png",
    buffer: Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
      "base64",
    ),
  });
  await expect(preview).toBeVisible();
  await expect(preview).toHaveAttribute("src", /^blob:/);
});

test("Copy all puts every backup code on the clipboard", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await openPage(page, "backup-codes", "light");

  const codes = await page.locator("#two_factor_backup_codes code").allTextContents();

  await page.getByRole("button", { name: "Copy all" }).click();
  await expect(page.getByRole("button", { name: "Copied" })).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(codes.join("\n"));
});

test("a confirmed action carries on by itself with what it was carrying", async ({ page }) => {
  const form = await selfSubmitted(page, "sudo-continue");

  // As classic's auto-submit: requestSubmit() with no submitter, so no `commit`.
  expect([...form.keys()].sort()).toEqual(["_method", "authenticity_token", "user[name]"]);
  expect(form.get("_method")).toBe("patch");
  expect(form.get("user[name]")).toBe("David <&>");
});

test("a second press while a form is leaving is dropped", async ({ page }) => {
  let posts = 0;

  await page.route(/\/sudo$/, (route) => {
    posts += 1;

    return route.fulfill({ status: 204 });
  });
  await openPage(page, "sudo-totp", "light");
  await page.getByLabel("Authenticator code").fill("123456");

  const confirm = page.getByRole("button", { name: "Confirm", exact: true });

  await confirm.click();
  await expect.poll(() => posts).toBe(1);
  await confirm.click({ force: true });
  await expect(confirm).toHaveAttribute("aria-busy", "true");
  // A frame for a second submission to leave, were it not dropped.
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(resolve)));
  expect(posts).toBe(1);
});
