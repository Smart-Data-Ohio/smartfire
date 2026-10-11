import type { APIRequestContext, Page } from "@playwright/test";
import type { SudoMethod } from "../../src/gen/SudoMethod.ts";
import type { SudoRetry } from "../../src/gen/SudoRetry.ts";
import {
  DESKTOP,
  expect,
  expectNoHorizontalOverflow,
  PHONE,
  shot,
  THEMES,
  type Theme,
  test,
} from "./support.ts";

/** The mock's password and authenticator code (mock/s2/settings.ts, mock/s2/account.ts). */
const PASSWORD = "secret123456";

const CODE = "123456";

const SIZES = [
  ["desktop", DESKTOP],
  ["phone", PHONE],
] as const;

/** A form the server's classic gate held: connecting Google from Settings › Integrations. */
const connectRetry: SudoRetry = {
  method: "POST",
  path: "/google/connect",
  returnTo: "/app/settings/integrations",
};

/** A page it held, asked for in full. */
const pageRetry: SudoRetry = {
  method: "GET",
  path: "/app/settings/profile",
  returnTo: "/app/settings",
};

/**
 * Sets the mock's confirmation: lapsed (asking with `methods`) or fresh, and the request its
 * classic gate held meanwhile.
 */
async function sudo(
  request: APIRequestContext,
  lapsed: boolean,
  retry: SudoRetry | null,
  methods: readonly SudoMethod[] = ["password"],
): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/lapse-sudo", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { on: lapsed, methods, retry },
  });
}

/** Loads `/app/sudo/new` in full, as the server's gate sends a page there. */
async function openPage(page: Page, theme: Theme): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto("/app/sudo/new");
}

const heading = (page: Page) => page.getByRole("heading", { level: 1, name: "Confirm it's you" });

for (const theme of THEMES) {
  for (const [size, viewport] of SIZES) {
    test.describe(`(${theme}, ${size})`, () => {
      test.beforeEach(async ({ page }) => {
        await page.setViewportSize(viewport);
      });

      test("asks by keyboard, then goes back to make the change again, never sending it", async ({
        page,
        request,
      }) => {
        const posts: string[] = [];

        page.on("request", (sent) => {
          if (sent.method() === "POST" && !sent.url().includes("/api/")) posts.push(sent.url());
        });
        await sudo(request, true, connectRetry, ["password", "totp", "google"]);
        await openPage(page, theme);

        await expect(heading(page)).toBeVisible();
        await expect(page.getByLabel("Password")).toBeFocused();
        await expect(page.getByLabel("Authenticator code")).toBeVisible();
        await expect(page.getByRole("button", { name: "Confirm with Google" })).toBeVisible();
        await expectNoHorizontalOverflow(page);
        await shot(page, "sudo-new-asked", theme);

        await page.keyboard.type("not-the-password");
        await page.keyboard.press("Enter");
        await expect(page.getByText("Confirmation failed. Try again.")).toBeVisible();
        await expect(page.getByLabel("Password")).toBeFocused();
        await expect(page.getByLabel("Password")).toHaveAttribute("aria-invalid", "true");
        await expectNoHorizontalOverflow(page);
        await shot(page, "sudo-new-refused", theme);

        await page.keyboard.type(PASSWORD);
        await page.keyboard.press("Enter");
        await expect(page).toHaveURL(/\/app\/settings\/integrations$/);
        await expect(page.getByText("Confirmed. Try that again.")).toBeVisible();
        expect(posts).toEqual([]);
      });

      test("after a code, the page the server held loads again", async ({ page, request }) => {
        await sudo(request, true, pageRetry, ["totp"]);
        await openPage(page, theme);

        const field = page.getByLabel("Authenticator code");

        await expect(field).toBeFocused();
        await expect(page.getByLabel("Password")).toHaveCount(0);
        await field.fill(CODE);
        await page.getByRole("button", { name: "Confirm code" }).click();
        await expect(page).toHaveURL(/\/app\/settings\/profile$/);
      });

      test("already confirmed, it goes straight on", async ({ page, request }) => {
        await sudo(request, false, pageRetry);
        await openPage(page, theme);

        await expect(page).toHaveURL(/\/app\/settings\/profile$/);
        await expect(heading(page)).toHaveCount(0);
      });

      test("with Google, it comes back through the continue page", async ({ page, request }) => {
        await sudo(request, true, connectRetry, ["google"]);
        await openPage(page, theme);

        await expect(heading(page)).toBeVisible();
        await shot(page, "sudo-new-google", theme);
        await page.getByRole("button", { name: "Confirm with Google" }).click();
        await expect(page).toHaveURL(/\/app\/settings\/integrations$/);
        await expect(page.getByText("Try that again.")).toBeVisible();
        expect(
          await page.evaluate(() => sessionStorage.getItem("smartfire:confirmation-pending")),
        ).toBeNull();
      });

      test("Cancel goes back to where the request came from", async ({ page, request }) => {
        await sudo(request, true, connectRetry);
        await openPage(page, theme);

        await expect(page.getByLabel("Password")).toBeFocused();
        await page.getByRole("button", { name: "Cancel" }).click();
        await expect(page).toHaveURL(/\/app\/settings\/integrations$/);
        await expect(page.getByText("Confirmed")).toHaveCount(0);
      });

      // A session that ended answers 401; no session at all, the same hand-off at 200.
      for (const status of [200, 401]) {
        test(`signed out (${status}), it goes to sign in`, async ({ page, request }) => {
          await sudo(request, true, null);
          await page.route(/\/api\/v1\/sudo(\?|$)/, (route) =>
            route.fulfill({
              status,
              contentType: "application/json",
              body: JSON.stringify({ kind: "navigate", location: "/app/session/new" }),
            }),
          );
          await openPage(page, theme);

          await expect(page).toHaveURL(/\/app\/session\/new/);
        });
      }

      test("a full-page action asks in place first, then goes", async ({ page, request }) => {
        const connects: string[] = [];

        await page.route("**/google/connect", async (route) => {
          connects.push(route.request().postData() ?? "");
          await route.fulfill({ status: 200, contentType: "text/html", body: "<p>Google</p>" });
        });
        await sudo(request, true, null, ["password", "google"]);
        await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
        await page.goto("/app/settings/integrations");

        await page.getByRole("button", { name: "Enable Drive previews" }).click();

        const dialog = page.getByRole("dialog", { name: "Confirm it's you" });

        await expect(dialog).toBeVisible();
        await expect(
          dialog.getByText(/When you're back, click what you were doing again to finish\./),
        ).toBeVisible();
        await expectNoHorizontalOverflow(page);
        await shot(page, "sudo-preflight-asked", theme);
        expect(connects).toEqual([]);

        await dialog.getByLabel("Password").fill(PASSWORD);
        await dialog.getByLabel("Password").press("Enter");
        await expect(page.getByText("Google", { exact: true })).toBeVisible();
        expect(connects).toHaveLength(1);
        expect(connects[0]).toContain("features%5B%5D=drive");
      });

      test("a full-page action confirmed with Google asks for the click again", async ({
        page,
        request,
      }) => {
        await sudo(request, true, null, ["password", "google"]);
        await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
        await page.goto("/app/settings/integrations");

        await page.getByRole("button", { name: "Enable Drive previews" }).click();
        await page
          .getByRole("dialog", { name: "Confirm it's you" })
          .getByRole("button", { name: "Confirm with Google" })
          .click();

        await expect(page).toHaveURL(/\/app\/settings\/integrations$/);
        await expect(page.getByText("Try that again.")).toBeVisible();
        await shot(page, "sudo-preflight-google-back", theme);
      });
    });
  }
}
