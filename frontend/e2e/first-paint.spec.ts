import { execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { fileURLToPath } from "node:url";
import { expect, type Page, test } from "@playwright/test";

/**
 * The production build's first paint, before any of the SPA's JavaScript runs, on the page the
 * Rust shell renders (`campfire_spa::render_shell` over this build, through the crate's
 * `render_shell` example) and under the production policy: a script without the request's nonce
 * doesn't run. The entry module is held until the test has looked; the stylesheet is a `<link>`,
 * so what's painted here is what a person on a slow connection sees first. Released, the SPA must
 * keep the same appearance.
 */

const workspace = fileURLToPath(new URL("../..", import.meta.url));

const cargo = (args: readonly string[]) =>
  execFileSync("cargo", ["-q", ...args], {
    cwd: workspace,
    env: { ...process.env, SPA_DIST: "frontend/dist" },
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  });

const example = ["-p", "campfire_spa", "--example", "render_shell"];

/** `crates/app/src/security.rs`'s `content_security_policy`, enforced, with `nonce`. */
const policy = (nonce: string) =>
  [
    "default-src 'self'",
    "base-uri 'self'",
    "object-src 'none'",
    `script-src 'self' 'wasm-unsafe-eval' https://accounts.google.com/gsi/ https://apis.google.com 'nonce-${nonce}'`,
    "style-src 'self' 'unsafe-inline' https://accounts.google.com/gsi/style",
    "img-src 'self' data: blob: https:",
    "font-src 'self' data: https://fonts.gstatic.com",
    "media-src 'self' data: blob:",
    "connect-src 'self'",
    "worker-src 'self' blob:",
    "manifest-src 'self'",
  ].join("; ");

test.beforeAll(() => {
  test.setTimeout(600_000);
  // The webServer has just built the dist; the example embeds it.
  cargo(["build", "-j", "4", ...example]);
});

interface Opened {
  /** Lets the SPA's entry module load. */
  readonly release: () => void;
  /** The console's Content Security Policy refusals so far. */
  readonly refusals: readonly string[];
}

async function openHeld(
  page: Page,
  boot: { theme: string; textSize: string },
  edit: (html: string, nonce: string) => string = (html) => html,
): Promise<Opened> {
  const entry = Promise.withResolvers<void>();
  const nonce = randomBytes(16).toString("base64");
  const shell = cargo(["run", "-j", "4", ...example, "--", boot.theme, boot.textSize, nonce]);
  const refusals: string[] = [];

  page.on("console", (message) => {
    if (message.text().includes("Content Security Policy")) refusals.push(message.text());
  });
  await page.route(/\/app\/assets\/index-[\w-]+\.js$/, async (route) => {
    await entry.promise;
    await route.continue();
  });
  await page.route(/\/app\/(\?.*)?$/, (route) =>
    route.fulfill({
      status: 200,
      headers: {
        "content-type": "text/html; charset=utf-8",
        "content-security-policy": policy(nonce),
      },
      body: edit(shell, nonce),
    }),
  );
  await page.goto("/app/", { waitUntil: "commit" });
  // The stylesheet has applied: the body has a painted background.
  await page.waitForFunction(
    () =>
      document.body !== null &&
      getComputedStyle(document.body).backgroundColor !== "rgba(0, 0, 0, 0)",
  );
  expect(
    await page.evaluate(() => document.getElementById("root")?.childElementCount),
    "the SPA hasn't rendered yet",
  ).toBe(0);

  return { release: () => entry.resolve(), refusals };
}

/** Releases the entry module and waits for the SPA's first render. */
async function hydrate(page: Page, opened: Opened): Promise<void> {
  opened.release();
  await page.waitForFunction(() => (document.getElementById("root")?.childElementCount ?? 0) > 0);
}

/** WCAG relative luminance of the body's painted background. */
function backgroundLuminance(page: Page): Promise<number> {
  return page.evaluate(() => {
    const canvas = document.createElement("canvas").getContext("2d");

    if (canvas === null) throw new Error("no 2d canvas");
    canvas.fillStyle = getComputedStyle(document.body).backgroundColor;
    canvas.fillRect(0, 0, 1, 1);

    const [r = 0, g = 0, b = 0] = canvas.getImageData(0, 0, 1, 1).data;

    const linear = (channel: number) => {
      const c = channel / 255;

      return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    };

    return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
  });
}

const rootFontSize = (page: Page) =>
  page.evaluate(() => getComputedStyle(document.documentElement).fontSize);

test("the account's dark theme and text size are painted before the SPA's script runs", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });

  const opened = await openHeld(page, { theme: "dark", textSize: "larger" });
  const html = page.locator("html");

  await expect(html).toHaveAttribute("data-theme", "dark");
  expect(await backgroundLuminance(page)).toBeLessThan(0.05);
  expect(await rootFontSize(page)).toBe("18px");

  await hydrate(page, opened);
  await expect(html).toHaveAttribute("data-theme", "dark");
  await expect(html).toHaveAttribute("data-text-size", "larger");
  expect(await backgroundLuminance(page)).toBeLessThan(0.05);
  expect(await rootFontSize(page)).toBe("18px");
  expect(opened.refusals).toEqual([]);
});

test("a theme pinned on this device is painted first, over the account's", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await page.addInitScript(() =>
    localStorage.setItem("smartfire.appearance", JSON.stringify({ themeOverride: "light" })),
  );

  const opened = await openHeld(page, { theme: "dark", textSize: "default" });
  const html = page.locator("html");

  await expect(html).toHaveAttribute("data-theme", "light");
  expect(await backgroundLuminance(page)).toBeGreaterThan(0.8);

  await hydrate(page, opened);
  await expect(html).toHaveAttribute("data-theme", "light");
  expect(await backgroundLuminance(page)).toBeGreaterThan(0.8);
  expect(opened.refusals).toEqual([]);
});

test("this device's palette and font are painted first, before the SPA derives them", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.addInitScript(() =>
    localStorage.setItem(
      "smartfire.appearance",
      JSON.stringify({
        palette: "ember",
        font: "serif",
        paletteTokens: { "--bg-app": "light-dark(oklch(30% 0.05 50), oklch(20% 0.02 50))" },
      }),
    ),
  );

  const opened = await openHeld(page, { theme: "light", textSize: "default" });
  const html = page.locator("html");

  await expect(html).toHaveAttribute("data-palette", "ember");
  await expect(html).toHaveAttribute("data-font", "serif");
  // The stored --bg-app (a dark brown, even in the light theme) is what the body paints.
  expect(await backgroundLuminance(page)).toBeLessThan(0.1);

  // The SPA derives Ember's own tokens, a light surface in the light theme, over the stored ones.
  await hydrate(page, opened);
  await expect(html).toHaveAttribute("data-palette", "ember");
  await expect(html).toHaveAttribute("data-font", "serif");
  await expect.poll(() => backgroundLuminance(page)).toBeGreaterThan(0.8);
  expect(opened.refusals).toEqual([]);
});

test("corrupted choices on this device are ignored, at first paint and once the SPA runs", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.addInitScript(() =>
    localStorage.setItem(
      "smartfire.appearance",
      JSON.stringify({
        themeOverride: ["light"],
        density: ["compact"],
        palette: ["ember"],
        font: ["serif"],
        paletteTokens: { "--bg-app": "oklch(100% 0 0)" },
      }),
    ),
  );

  const opened = await openHeld(page, { theme: "dark", textSize: "default" });
  const html = page.locator("html");

  // The account's dark theme, and none of the corrupted choices.
  const accountOnly = async (stage: string) => {
    await expect(html, stage).toHaveAttribute("data-theme", "dark");

    for (const name of ["data-density", "data-palette", "data-font"]) {
      await expect(html, `${stage}: ${name}`).not.toHaveAttribute(name, /.*/);
    }

    expect(await backgroundLuminance(page), stage).toBeLessThan(0.05);
  };

  await accountOnly("first paint");
  await hydrate(page, opened);
  await accountOnly("hydrated");
  expect(opened.refusals).toEqual([]);
});

test("the policy is enforced: without its nonce, the initializer doesn't run", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });

  // The shell as it would be had it left the blocking initializer (its first script) unsigned.
  const opened = await openHeld(page, { theme: "dark", textSize: "larger" }, (html, nonce) =>
    html.replace(`<script nonce="${nonce}">`, "<script>"),
  );

  await expect(page.locator("html")).not.toHaveAttribute("data-theme", /.*/);
  expect(await rootFontSize(page)).toBe("16px");
  expect(opened.refusals.length).toBeGreaterThan(0);
  opened.release();
});
