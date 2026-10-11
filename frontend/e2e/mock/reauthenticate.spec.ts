import type { APIRequestContext, Page } from "@playwright/test";
import type { SudoMethod } from "../../src/gen/SudoMethod.ts";
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

/** Lapses the confirmation: guarded writes answer `SudoRequired`, offering `methods`. */
async function lapse(request: APIRequestContext, methods: readonly SudoMethod[]): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/lapse-sudo", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { on: true, methods },
  });
}

/** Opens a settings or admin page with motion reduced, and waits for its heading. */
async function open(page: Page, path: string, theme: Theme): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(path);
  await page.locator(".settings-page h1").waitFor();
}

/** Lets the sheet's entrance finish so a shot is settled. */
async function settle(page: Page): Promise<void> {
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
        .map((animation) => animation.finished.catch(() => animation)),
    ),
  );
}

function confirmation(page: Page) {
  return page.getByRole("dialog", { name: "Confirm it's you" });
}

/** A wrong credential first, then the right one, by `method`. */
async function confirmBy(page: Page, method: "password" | "totp", name: string, theme: Theme) {
  const dialog = confirmation(page);
  const field = dialog.getByLabel(method === "password" ? "Password" : "Authenticator code");

  const submit = dialog.getByRole("button", {
    name: method === "password" ? "Confirm password" : "Confirm code",
  });

  await expect(dialog).toBeVisible();
  await expect(dialog.getByLabel("Password")).toBeFocused();
  await expectNoHorizontalOverflow(page);
  await settle(page);
  await shot(page, `${name}-asked`, theme);

  await field.fill(method === "password" ? "not-the-password" : "000000");
  await submit.click();
  await expect(dialog.getByText("Confirmation failed. Try again.")).toBeVisible();
  await settle(page);
  await expect(field).toHaveValue("");
  await expect(field).toBeFocused();
  await expect(field).toHaveAttribute("aria-invalid", "true");
  await shot(page, `${name}-refused`, theme);

  await field.fill(method === "password" ? PASSWORD : CODE);
  await field.press("Enter");
  await expect(dialog).toBeHidden();
}

for (const theme of THEMES) {
  for (const [size, viewport] of SIZES) {
    test.describe(`(${theme}, ${size})`, () => {
      test.beforeEach(async ({ page }) => {
        await page.setViewportSize(viewport);
      });

      for (const method of ["password", "totp"] as const) {
        test(`an admin write waits for the ${method} and saves once`, async ({ page, request }) => {
          let puts = 0;

          page.on("request", (sent) => {
            if (sent.method() === "PATCH" && sent.url().endsWith("/api/v1/admin/custom_styles"))
              puts += 1;
          });
          await lapse(request, ["password", "totp", "google"]);
          await open(page, "/app/admin/styles", theme);

          await page.getByRole("textbox", { name: "Custom CSS" }).fill("body { color: red; }");
          await page.getByRole("button", { name: "Save changes" }).click();
          await confirmBy(page, method, `reauth-admin-${method}`, theme);

          await expect(page.getByText("Custom styles saved")).toBeVisible();
          // Refused once, sent again once.
          expect(puts).toBe(2);

          await open(page, "/app/admin/styles", theme);
          await expect(page.getByRole("textbox", { name: "Custom CSS" })).toHaveValue(
            "body { color: red; }",
          );
        });

        test(`an integration write waits for the ${method} and goes through`, async ({
          page,
          request,
        }) => {
          await lapse(request, ["password", "totp"]);
          await open(page, "/app/settings/integrations", theme);

          const github = page.getByRole("region", { name: "GitHub", exact: true });

          await github.getByRole("button", { name: "Disconnect GitHub" }).click();
          await page
            .getByRole("alertdialog", { name: "Disconnect GitHub?" })
            .getByRole("button", { name: "Disconnect" })
            .click();
          await expect(
            confirmation(page).getByRole("button", { name: "Confirm with Google" }),
          ).toHaveCount(0);
          await confirmBy(page, method, `reauth-integration-${method}`, theme);

          await expect(page.getByText("GitHub disconnected.")).toBeVisible();
          await expect(github.getByRole("link", { name: "Connect with GitHub" })).toBeVisible();
          await shot(page, `reauth-integration-${method}-done`, theme);
        });
      }

      test("Cancel leaves the change unsent and the page as it was", async ({ page, request }) => {
        await lapse(request, ["password"]);
        await open(page, "/app/admin/styles", theme);

        await page.getByRole("textbox", { name: "Custom CSS" }).fill("body { color: red; }");
        await page.getByRole("button", { name: "Save changes" }).click();
        await confirmation(page).getByRole("button", { name: "Cancel" }).click();
        await expect(confirmation(page)).toBeHidden();
        await expect(page.getByRole("textbox", { name: "Custom CSS" })).toHaveValue(
          "body { color: red; }",
        );
        await expect(page.getByRole("button", { name: "Save changes" })).toBeFocused();
        await expect(page.getByText("Custom styles saved")).toHaveCount(0);
      });

      test("leaving the page closes the confirmation and drops the change", async ({
        page,
        request,
      }) => {
        let patches = 0;

        page.on("request", (sent) => {
          if (sent.method() === "PATCH" && sent.url().endsWith("/api/v1/admin/custom_styles")) {
            patches += 1;
          }
        });
        await lapse(request, ["password"]);
        await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
        await page.goto("/app/admin");

        const sections = page.getByRole("navigation", { name: "Workspace sections" });

        await sections.getByRole("link", { name: "Custom styles" }).click();
        await page.getByRole("textbox", { name: "Custom CSS" }).fill("body { color: red; }");
        await page.getByRole("button", { name: "Save changes" }).click();
        await expect(confirmation(page)).toBeVisible();

        await page.goBack();
        await expect(page).toHaveURL(/\/app\/admin$/);
        await expect(confirmation(page)).toBeHidden();

        // Confirming something else later doesn't send the abandoned edit.
        await open(page, "/app/admin/styles", theme);
        await expect(page.getByRole("textbox", { name: "Custom CSS" })).not.toHaveValue(
          "body { color: red; }",
        );
        expect(patches).toBe(1);
      });

      test("a failed return from Google keeps the change and tries again", async ({
        page,
        request,
      }) => {
        let failures = 1;

        await page.route("**/api/v1/sudo/continue", (route) =>
          failures-- > 0
            ? route.fulfill({ status: 503, contentType: "application/json", body: "{}" })
            : route.fallback(),
        );
        await lapse(request, ["password", "google"]);
        await open(page, "/app/admin/styles", theme);

        await page.getByRole("textbox", { name: "Custom CSS" }).fill("p { margin: 0; }");
        await page.getByRole("button", { name: "Save changes" }).click();
        await confirmation(page).getByRole("button", { name: "Confirm with Google" }).click();

        const retry = page.getByRole("button", { name: "Try again" });

        await expect(page.getByText("Couldn't finish confirming")).toBeVisible();
        await expect(page).toHaveURL(/\/app\/sudo\/continue$/);
        await expectNoHorizontalOverflow(page);
        await shot(page, "reauth-google-unreachable", theme);

        await retry.click();
        await expect(page).toHaveURL(/\/app\/admin\/styles$/);
        await expect(page.getByText("Confirmed — your change was saved")).toBeVisible();
        await page.locator(".settings-page h1").waitFor();
        await expect(page.getByRole("textbox", { name: "Custom CSS" })).toHaveValue(
          "p { margin: 0; }",
        );
      });

      test("a Google confirmation comes back and finishes the change", async ({
        page,
        request,
      }) => {
        await lapse(request, ["password", "google"]);
        await open(page, "/app/admin/styles", theme);

        await page.getByRole("textbox", { name: "Custom CSS" }).fill("main { color: blue; }");
        await page.getByRole("button", { name: "Save changes" }).click();
        await expect(confirmation(page)).toBeVisible();
        await settle(page);
        await shot(page, "reauth-google-asked", theme);

        const resumed = page.waitForRequest(
          (sent) => sent.method() === "PATCH" && sent.url().endsWith("/api/v1/admin/custom_styles"),
        );

        await confirmation(page).getByRole("button", { name: "Confirm with Google" }).click();
        await resumed;
        await expect(page).toHaveURL(/\/app\/admin\/styles$/);
        await expect(page.getByText("Confirmed — your change was saved")).toBeVisible();
        await page.locator(".settings-page h1").waitFor();
        await expect(page.getByRole("textbox", { name: "Custom CSS" })).toHaveValue(
          "main { color: blue; }",
        );
        await shot(page, "reauth-google-back", theme);

        // Nothing is kept to go again.
        await page.goto("/app/sudo/continue");
        await expect(page.getByText("Try that again.")).toBeVisible();
        await expect(page).not.toHaveURL(/\/sudo\/continue$/);
        await expect(page.getByText("Confirmed — your change was saved")).toHaveCount(0);
      });
    });
  }
}
