import type { Page } from "@playwright/test";
import {
  expect,
  expectNoHorizontalOverflow,
  matrix,
  shot as save,
  type Theme,
  test,
} from "./support.ts";

/**
 * About, Privacy and Terms at their SPA paths (`/app/about`, ...), signed out and signed in, against
 * the mock's copy of the Rust contract (mock/s2/public-pages.json). The mock always inlines the
 * signed-in boot, so a signed-out visit swaps in the signed-out boot the Rust shell renders for a
 * visitor without a session, and answers the signed-in boot endpoint 401.
 */

const PAGES = [
  { path: "about", heading: "About Smartfire", title: "Smartfire | About" },
  { path: "privacy", heading: "Privacy Policy", title: "Smartfire | Privacy Policy" },
  { path: "terms", heading: "Terms of Service", title: "Smartfire | Terms of Service" },
] as const;

type Visitor = "signed-out" | "signed-in";

const BOOT_SCRIPT = /<script type="application\/json" id="boot">[\s\S]*?<\/script>/;

/** Serves `/app/*` documents as the Rust shell does for a visitor without a session. */
async function signOut(page: Page): Promise<void> {
  const boot = await (await page.request.get("/api/v1/session/boot")).text();

  await page.route("**/api/v1/boot", (route) => route.fulfill({ status: 401, body: "" }));
  await page.route(/\/app\/(about|privacy|terms|session\/new)$/, async (route) => {
    if (route.request().resourceType() !== "document") return route.fallback();

    const response = await route.fetch();
    const html = (await response.text()).replace(
      BOOT_SCRIPT,
      `<script type="application/json" id="boot">${boot.replaceAll("<", "\\u003c")}</script>`,
    );

    return route.fulfill({ response, body: html });
  });
}

async function open(page: Page, path: string, theme: Theme, visitor: Visitor): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });

  if (visitor === "signed-out") await signOut(page);

  await page.goto(`/app/${path}`);
  await page.getByRole("article").waitFor();
  await page.evaluate(() => document.fonts.ready);
}

/** A shot with the pointer out of the way. */
async function shot(page: Page, name: string, theme: Theme): Promise<void> {
  await page.mouse.move(0, 0);
  await save(page, name, theme);
}

for (const visitor of ["signed-out", "signed-in"] as const) {
  for (const { path, heading, title } of PAGES) {
    matrix(`draws /app/${path} ${visitor}`, async ({ page, theme }) => {
      const sockets: string[] = [];

      // The app's sync socket; Vite's own (`/app/?token=`) is the dev server's.
      page.on("websocket", (socket) => {
        if (socket.url().includes("/api/v1/sync")) sockets.push(socket.url());
      });
      await open(page, path, theme, visitor);

      await expect(page.getByRole("heading", { level: 1, name: heading })).toBeVisible();
      await expect(page).toHaveTitle(title);
      await expect(page.locator('meta[name="description"]')).toHaveCount(1);

      // The retained layout's header, on the SPA's pages, and nothing of the app around it.
      const header = page.getByRole("navigation", { name: "Public pages" });

      await expect(header.getByRole("link", { name: "Privacy" })).toHaveAttribute(
        "href",
        "/app/privacy",
      );
      await expect(header.getByRole("link", { name: "Sign in" })).toHaveAttribute(
        "href",
        "/app/session/new",
      );
      await expect(page.getByRole("complementary", { name: "Conversations" })).toHaveCount(0);

      // The policy the contract carries: who runs the workspace and how to write to them.
      const article = page.getByRole("article");

      if (path === "about") {
        await expect(article.getByText("Example <&> Labs")).toBeVisible();
      } else {
        await expect(article.getByText("Last updated: 2026-10-07")).toBeVisible();
      }

      await expectNoHorizontalOverflow(page);
      await shot(page, `public-${path}-${visitor}`, theme);

      // The long-form end: the contact lines and the footer, readable all the way down.
      const footer = page.getByRole("navigation", { name: "Footer" });

      await footer.scrollIntoViewIfNeeded();
      await expect(footer.getByRole("link", { name: "Terms of Service" })).toBeVisible();
      await expect(
        article.getByRole("link", { name: "team+auth@example.test" }).first(),
      ).toBeInViewport();
      await expectNoHorizontalOverflow(page);
      await shot(page, `public-${path}-${visitor}-end`, theme);

      if (visitor === "signed-out") expect(sockets).toEqual([]);
    });
  }
}

test("the SPA sign-in page opens the SPA's public pages in a new tab", async ({ page }) => {
  await signOut(page);
  await page.goto("/app/session/new");

  const links = page.getByRole("navigation", { name: "About this workspace" });

  await expect(links.getByRole("link", { name: "Privacy Policy" })).toBeVisible();

  const [popup] = await Promise.all([
    page.waitForEvent("popup"),
    links.getByRole("link", { name: "Privacy Policy" }).click(),
  ]);

  await popup.waitForURL(/\/app\/privacy$/);
  await expect(popup.getByRole("heading", { level: 1, name: "Privacy Policy" })).toBeVisible();

  // In the article, the other pages and sign-in stay in the SPA.
  await expect(
    popup.getByRole("article").getByRole("link", { name: "Google Account connections page" }),
  ).toHaveAttribute("href", "https://myaccount.google.com/connections");
  await popup.goto("/app/about");
  await expect(
    popup.getByRole("article").getByRole("link", { name: "terms of service" }),
  ).toHaveAttribute("href", "/app/terms");
  await expect(
    popup.getByRole("article").getByRole("link", { name: "Sign in to this workspace" }),
  ).toHaveAttribute("href", "/app/session/new");
});

test("skips to the article from the keyboard", async ({ page }) => {
  await page.goto("/app/terms");
  await page.getByRole("article").waitFor();
  await page.keyboard.press("Tab");

  const skip = page.getByRole("link", { name: "Skip to main content" });

  await expect(skip).toBeFocused();
  await expect(skip).toBeInViewport();
  await page.keyboard.press("Enter");
  await expect(page.locator("#public-main")).toBeFocused();
});
