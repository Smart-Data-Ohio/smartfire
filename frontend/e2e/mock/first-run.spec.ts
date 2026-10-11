import { join } from "node:path";
import type { APIRequestContext, Locator, Page } from "@playwright/test";
import { field, type Json, parseJson } from "../../mock/json.ts";
import { UNREADABLE_SUBMISSION } from "../../mock/s2/sign-in.ts";
import {
  expect,
  expectNoHorizontalOverflow,
  matrix,
  shot as save,
  type Theme,
  test,
} from "./support.ts";

/**
 * The SPA's first run (slice 73) at `/app/first_run`, against the mock's contract
 * (mock/s2/sign-in.ts): a fresh install's sign-in page hands over to it, and setting up leaves for
 * the app with a full page load, which the always-signed-in mock then serves.
 */

const AVATAR = join(
  import.meta.dirname,
  "..",
  "..",
  "..",
  "fixtures",
  "files",
  "workspace_icons",
  "square_64.png",
);

async function configure(
  request: APIRequestContext,
  change: { readonly firstRunPending: boolean },
): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/sign-in", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: change,
  });
}

/** The last first-run submission as the mock received it. */
async function received(request: APIRequestContext): Promise<Json | undefined> {
  return field(parseJson(await (await request.get("/__mock/state")).text()), "firstRun");
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

async function top(locator: Locator): Promise<number> {
  return (await locator.boundingBox())?.y ?? Number.NaN;
}

matrix("sets up the first administrator from the sign-in page", async ({ page, theme }) => {
  await configure(page.request, { firstRunPending: true });
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto("/app/session/new");

  // The sign-in page hands a fresh install to first run, inside the SPA.
  await page.waitForURL(/\/app\/first_run$/);
  await expect(page.getByRole("heading", { level: 1, name: "Set up Smartfire" })).toBeVisible();
  await page.evaluate(() => document.fonts.ready);

  const name = page.getByRole("textbox", { name: "Name" });
  const create = page.getByRole("button", { name: "Create account" });

  await expect(
    page.getByText("Create the first account. It administers the workspace."),
  ).toBeVisible();
  await expect(name).toBeFocused();
  await expect(page.getByText("Optional")).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await shot(page, "signed-out-first-run", theme);

  // A refusal takes its kept line under the button: nothing above it moves.
  const before = await top(create);

  await name.fill("fail");
  await page.getByRole("textbox", { name: "Email address" }).fill("ada@example.com");
  await page.getByLabel("Password", { exact: true }).fill("secret123456");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("alert")).toHaveText(UNREADABLE_SUBMISSION);
  expect(await top(create)).toBe(before);
  await expect(page.getByLabel("Password", { exact: true })).toHaveValue("secret123456");
  await shot(page, "signed-out-first-run-refused", theme);

  // The avatar: chosen, previewed, named under the picker.
  await page.getByLabel("Add your avatar").setInputFiles(AVATAR);
  await expect(page.locator(".auth-view-avatar-preview")).toBeVisible();
  await expect(page.getByText("square_64.png")).toBeVisible();
  expect(await top(create)).toBe(before);
  await name.fill("Ada Lovelace");
  await expectNoHorizontalOverflow(page);
  await shot(page, "signed-out-first-run-avatar", theme);

  await create.click();
  await page.waitForURL(/\/app\/$/);
  await page
    .getByRole("complementary", { name: "Conversations" })
    .or(page.locator(".room-header"))
    .first()
    .waitFor();

  const form = await received(page.request);

  expect(form).toMatchObject({
    submission: JSON.stringify({
      name: "Ada Lovelace",
      emailAddress: "ada@example.com",
      password: "secret123456",
    }),
    avatar: { filename: "square_64.png", contentType: "image/png" },
  });
});

test("completes first run from the keyboard alone", async ({ page }) => {
  await configure(page.request, { firstRunPending: true });
  await page.goto("/app/first_run");

  const name = page.getByRole("textbox", { name: "Name" });

  await expect(name).toBeFocused();

  // Back past the name's translation list to the avatar picker, whose file chooser opens from
  // the keyboard.
  await page.keyboard.press("Shift+Tab");
  await expect(page.getByRole("button", { name: "Translate name" })).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(page.getByLabel("Add your avatar")).toBeFocused();

  const chooser = page.waitForEvent("filechooser");

  await page.keyboard.press("Space");
  await (await chooser).setFiles(AVATAR);
  await expect(page.getByText("square_64.png")).toBeVisible();

  // Each field follows its label's translation list.
  for (const [field, value] of [
    ["name", "Ada"],
    ["email address", "ada@example.com"],
    ["password", "secret123456"],
  ] as const) {
    await page.keyboard.press("Tab");
    await expect(page.getByRole("button", { name: `Translate ${field}` })).toBeFocused();
    await page.keyboard.press("Tab");
    await page.keyboard.type(value);
  }

  await page.keyboard.press("Enter");

  await page.waitForURL(/\/app\/$/);
  expect(await received(page.request)).toMatchObject({
    avatar: { filename: "square_64.png" },
  });
});

test("goes home once the workspace exists", async ({ page }) => {
  await configure(page.request, { firstRunPending: false });

  const home = page.waitForRequest((request) => new URL(request.url()).pathname === "/");

  await page.goto("/app/first_run");
  await home;
  await expect(page.getByRole("textbox", { name: "Name" })).toHaveCount(0);
});
