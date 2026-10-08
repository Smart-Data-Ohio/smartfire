import type { Page } from "@playwright/test";
import { expect, openApp, ROOM_IDS, test } from "./support.ts";

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

const composer = (page: Page) => page.getByRole("textbox", { name: "Message #general" });

const DRAFT = "stay put while it loads";

const SCROLL = 320;

/** Records any moment `[aria-label="Loading page"]` is actually visible, including a CSS reveal. */
async function watchLoadingPage(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const seen: number[] = [];

    const shown = (element: Element) => {
      if (!(element instanceof HTMLElement)) {
        return false;
      }

      const style = getComputedStyle(element);
      const box = element.getBoundingClientRect();

      return (
        style.display !== "none" &&
        style.visibility !== "hidden" &&
        Number(style.opacity) > 0 &&
        box.width > 0 &&
        box.height > 0
      );
    };

    const scan = () => {
      if (seen.length > 0) {
        return;
      }

      for (const element of document.querySelectorAll('[aria-label="Loading page"]')) {
        if (shown(element)) {
          seen.push(performance.now());
          document.documentElement.dataset.loadingPageSeen = "1";
        }
      }
    };

    const observer = new MutationObserver(scan);

    const arm = () => {
      document.documentElement.dataset.loadingPageSeen ??= "0";
      observer.observe(document.documentElement, {
        attributes: true,
        attributeFilter: ["class", "hidden", "style"],
        childList: true,
        subtree: true,
      });
      scan();
    };

    if (document.documentElement) {
      arm();
    } else {
      document.addEventListener("DOMContentLoaded", arm, { once: true });
    }

    const pump = () => {
      scan();
      requestAnimationFrame(pump);
    };

    requestAnimationFrame(pump);
  });
}

/** The openApp helper emulates reduced motion, which shows the placeholder at once. */
async function allowMotion(page: Page): Promise<void> {
  await page.evaluate(() => {
    document.documentElement.dataset.motion = "full";
  });

  await page.evaluate(
    () =>
      new Promise((resolve) => {
        requestAnimationFrame(() => requestAnimationFrame(() => resolve(undefined)));
      }),
  );
}

/** Tags the sidebar node and parks its scroller, so a remount is visible later. */
async function markShell(page: Page): Promise<void> {
  await sidebar(page).evaluate((node, scroll) => {
    if (!(node instanceof HTMLElement)) {
      throw new Error("sidebar is not an element");
    }

    // SAFETY: shellMark is an expando on this live node; a remounted sidebar does not have it.
    const shell = node as HTMLElement & { shellMark?: string };

    shell.shellMark = "kept";

    const scroller = node.querySelector(".sidebar-scroll");

    if (!(scroller instanceof HTMLElement)) {
      throw new Error("sidebar scroller missing");
    }

    scroller.style.paddingBottom = "4000px";
    scroller.scrollTop = scroll;
  }, SCROLL);
}

async function expectShellKept(page: Page): Promise<void> {
  await sidebar(page).evaluate((node, scroll) => {
    if (!(node instanceof HTMLElement)) {
      throw new Error("sidebar remounted");
    }

    // SAFETY: shellMark is an expando on this live node; a remounted sidebar does not have it.
    const shell = node as HTMLElement & { shellMark?: string };

    if (shell.shellMark !== "kept") {
      throw new Error("sidebar remounted");
    }

    const scroller = node.querySelector(".sidebar-scroll");

    if (!(scroller instanceof HTMLElement) || scroller.scrollTop !== scroll) {
      throw new Error("sidebar scroll was lost");
    }
  }, SCROLL);
}

test("a slow lazy route keeps the shell mounted until its screen arrives", async ({ page }) => {
  await watchLoadingPage(page);
  await openApp(page, `r/${ROOM_IDS.general}`);
  await allowMotion(page);
  await expect(page.getByRole("heading", { name: "general" })).toBeVisible();
  await composer(page).fill(DRAFT);
  await markShell(page);

  const eventsChunk = page.waitForRequest((request) =>
    new URL(request.url()).pathname.endsWith("/events-page.tsx"),
  );

  await page.getByRole("link", { name: "Events" }).hover();
  await eventsChunk;
  await expect(sidebar(page)).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));

  const gate = Promise.withResolvers<void>();

  await page.route(
    (url) => url.pathname.endsWith("/activity-route.tsx"),
    async (route) => {
      await gate.promise;
      await route.continue();
    },
  );

  await page.getByRole("button", { name: "Activity" }).click();

  await expect(composer(page)).toHaveValue(DRAFT);
  await expectShellKept(page);
  await expect(page.getByRole("status", { name: "Loading page" })).toBeVisible();
  await expect(sidebar(page)).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Destinations" })).toBeVisible();
  await expectShellKept(page);

  gate.resolve();

  await expect(page.getByRole("heading", { name: "Activity" })).toBeVisible();
  await expect(sidebar(page)).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Destinations" })).toBeVisible();
  await expectShellKept(page);
});

test("a fast lazy route never shows the loading skeleton", async ({ page }) => {
  await watchLoadingPage(page);
  await openApp(page, `r/${ROOM_IDS.general}`);
  await allowMotion(page);
  await expect(page.getByRole("heading", { name: "general" })).toBeVisible();
  await markShell(page);

  await page.route(
    (url) => url.pathname.endsWith("/activity-route.tsx"),
    async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 40));
      await route.continue();
    },
  );

  await page.getByRole("button", { name: "Activity" }).click();

  await expect(page.getByRole("heading", { name: "Activity" })).toBeVisible();
  await page.waitForTimeout(500);
  await expectShellKept(page);

  await expect(page.locator("html")).toHaveAttribute("data-loading-page-seen", "0");
});
