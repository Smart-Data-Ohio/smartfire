import type { Page } from "@playwright/test";
import { expect, openApp, ROOM_IDS, test } from "./support.ts";

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

// A cold chunk under load can take longer than the default 30s. The assertions do not depend on that.
test.setTimeout(120_000);

const composer = (page: Page) => page.getByRole("textbox", { name: "Message #general" });

const DRAFT = "stay put while it loads";

const SCROLL = 320;

/** Records any moment `selector` is actually visible, including a CSS reveal. */
async function watchShown(page: Page, selector: string, datasetKey: string): Promise<void> {
  await page.addInitScript(
    ({ selector, datasetKey }) => {
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

        for (const element of document.querySelectorAll(selector)) {
          if (shown(element)) {
            seen.push(performance.now());
            document.documentElement.dataset[datasetKey] = "1";
          }
        }
      };

      const observer = new MutationObserver(scan);

      const arm = () => {
        document.documentElement.dataset[datasetKey] ??= "0";
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
    },
    { selector, datasetKey },
  );
}

/** Records any moment `[aria-label="Loading page"]` is actually visible, including a CSS reveal. */
function watchLoadingPage(page: Page): Promise<void> {
  return watchShown(page, '[aria-label="Loading page"]', "loadingPageSeen");
}

/** The suspense bones in the right pane, not a loaded pane's own data skeleton. */
function watchPaneSkeleton(page: Page): Promise<void> {
  return watchShown(page, ".right-pane .pane-body > .pane-skeleton", "paneSkeletonSeen");
}

/**
 * The openApp helper emulates reduced motion, which shows the placeholder at once. Full motion
 * is read on the next render; the shell's 150ms wait is applied in an effect after paint, so
 * the timer waits past that effect. Page time must still be running here.
 */
async function allowMotion(page: Page): Promise<void> {
  await page.evaluate(() => {
    document.documentElement.dataset.motion = "full";
  });

  await page.evaluate(
    () =>
      new Promise((resolve) => {
        setTimeout(resolve, 50);
      }),
  );
}

/** The clock `page.clock.install()` puts on the page. */
interface PageClock {
  readonly controller: {
    now: () => number;
    pauseAt: (time: number) => Promise<void>;
  };
}

/**
 * Stops page time so a hold cannot elapse. Install `page.clock` before the page opens. The pause
 * is read and applied in the page: a round trip can outrun a cushion and land in the past. The
 * jump is shorter than either hold, and neither hold has been scheduled yet.
 */
async function freezeTime(page: Page): Promise<void> {
  await page.evaluate(async () => {
    // SAFETY: page.clock.install() defines __pwClock on this page before it opens.
    const clock = (globalThis as { __pwClock?: PageClock }).__pwClock;

    if (clock === undefined) {
      throw new Error("page clock is not installed");
    }

    await clock.controller.pauseAt(clock.controller.now() + 20);
  });
}

/**
 * Waits until `ready`. Page time advances at most `budget` ms, then waits on the wall clock, so a
 * longer hold still cannot elapse. React reveals a resolved fallback after about 300ms; the pane
 * bones wait 400ms, and `budget` sits between them.
 */
async function untilReady(
  page: Page,
  ready: () => Promise<boolean>,
  budget: number,
): Promise<void> {
  let advanced = 0;

  while (!(await ready())) {
    if (advanced >= budget) {
      await page.waitForTimeout(50);
      continue;
    }

    const step = Math.min(20, budget - advanced);

    await page.clock.runFor(step);
    advanced += step;
  }
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

  await sidebar(page)
    .getByRole("link", { name: /^general\b/ })
    .click();
  await expect(page.getByRole("heading", { name: "general" })).toBeVisible();
  await expect(composer(page)).toHaveValue(DRAFT);
});

test("a fast lazy route never shows the loading skeleton", async ({ page }) => {
  await page.clock.install();
  await watchLoadingPage(page);

  // Held across the wall-clock wait below. Page time is stopped, so the chunk can be slow.
  const gate = Promise.withResolvers<void>();

  await page.route(
    (url) => url.pathname.endsWith("/activity-route.tsx"),
    async (route) => {
      await gate.promise;
      await route.continue();
    },
  );

  await openApp(page, `r/${ROOM_IDS.general}`);
  await allowMotion(page);
  await expect(page.getByRole("heading", { name: "general" })).toBeVisible();
  await markShell(page);
  await freezeTime(page);

  await page.getByRole("button", { name: "Activity" }).click();

  // Longer than the shell's 150ms hold. That timer cannot run while the clock is stopped.
  await page.waitForTimeout(400);
  await expect(page.locator("html")).toHaveAttribute("data-loading-page-seen", "0");

  gate.resolve();

  await expect(page.getByRole("heading", { name: "Activity" })).toBeVisible();

  // The navigation has committed, so advancing past the hold must not mount the placeholder.
  await page.clock.fastForward(1_000);
  await expectShellKept(page);
  await expect(page.locator("html")).toHaveAttribute("data-loading-page-seen", "0");
});

test("a fast right-pane chunk names the pane at once and skips the skeleton", async ({ page }) => {
  await page.clock.install();
  await watchPaneSkeleton(page);

  // Held until the name is read. Page time is stopped, so the bone delay cannot elapse.
  const named = Promise.withResolvers<void>();

  await page.route(
    (url) => url.pathname.endsWith("/members-pane.tsx"),
    async (route) => {
      await named.promise;
      await route.continue();
    },
  );

  await openApp(page, `r/${ROOM_IDS.general}`);
  await allowMotion(page);
  await expect(page.getByRole("heading", { name: "general" })).toBeVisible();
  await freezeTime(page);

  await page
    .locator(".room-header")
    .getByRole("button", { name: /^Members/ })
    .click();

  const pane = page.locator("aside.right-pane");

  await expect
    .poll(() =>
      pane.evaluate((node) => {
        const id = node.getAttribute("aria-labelledby");
        const heading = id === null ? null : document.getElementById(id);

        return heading instanceof HTMLElement ? heading.textContent : "";
      }),
    )
    .toBe("Members");

  // Longer than the 400ms bone delay. That timer cannot run while the clock is stopped.
  await page.waitForTimeout(500);
  await expect(page.locator("html")).toHaveAttribute("data-pane-skeleton-seen", "0");

  named.resolve();

  const search = pane.getByRole("searchbox", { name: "Find a member" });

  await untilReady(page, () => search.isVisible(), 320);
  await expect(search).toBeVisible();

  // The chunk has committed, so advancing past the bone delay must not mount the skeleton.
  await page.clock.fastForward(1_000);
  await expect(page.locator("html")).toHaveAttribute("data-pane-skeleton-seen", "0");
});
