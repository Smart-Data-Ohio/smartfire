import { mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import {
  type APIRequestContext,
  test as base,
  expect,
  type Locator,
  type Page,
} from "@playwright/test";

/** The seeded ids (mock/seed.ts). */
export { ROOM_IDS, USER_IDS } from "../../mock/seed.ts";

export const DESKTOP = { width: 1440, height: 900 } as const;

export const PHONE = { width: 390, height: 844 } as const;

/** The narrowest phone the layout is held to: every phone spec checks its screens here. */
export const PHONE_SMALL = { width: 360, height: 740 } as const;

/**
 * `test.use(PHONE_TOUCH)`: a touch phone at PHONE_SMALL. Touch and mobile emulation make
 * `(pointer: coarse)` and `(hover: none)` match, so the touch sizes and 16 px fields apply.
 */
export const PHONE_TOUCH = { viewport: PHONE_SMALL, hasTouch: true, isMobile: true } as const;

/** Between the phone and desktop layouts: the right pane floats over the room as a sheet. */
export const TABLET = { width: 900, height: 1000 } as const;

export type Theme = "light" | "dark";

/** Wait for a native wheel gesture to move the list and finish scrolling. */
export async function scrollByWheel(page: Page, list: Locator, delta: number): Promise<void> {
  if (delta === 0) return;

  await list.hover();
  await list.evaluate((element) => {
    const offset = element.scrollTop;

    element.setAttribute("data-wheel-settled", "false");
    element.addEventListener(
      "wheel",
      () => {
        // Passive wheel listeners can run after the compositor has already scrolled.
        let moved = element.scrollTop !== offset;

        const scroll = () => {
          moved ||= element.scrollTop !== offset;
        };

        const end = (event: Event) => {
          if (event.target !== element || !moved) return;

          element.setAttribute("data-wheel-settled", "true");
          element.removeEventListener("scroll", scroll);
          element.removeEventListener("scrollend", end);
        };

        element.addEventListener("scroll", scroll);
        element.addEventListener("scrollend", end);
      },
      { capture: true, passive: true, once: true },
    );
  });
  await page.mouse.wheel(0, delta);
  await expect(list).toHaveAttribute("data-wheel-settled", "true");
}

/** Each test starts on a fresh seed: `/__mock/reset` (it also drops sync connections). */
export const test = base.extend<{ resetMock: undefined }>({
  resetMock: [
    async ({ request }, use) => {
      const state = await (await request.get("/__mock/state")).json();

      await request.post("/__mock/reset", { headers: { "X-CSRF-Token": state.csrfToken } });
      await use(undefined);
    },
    { auto: true },
  ],
});

export { expect };

interface HoldOptions {
  /**
   * Drops the frames other than the `welcome` that the server sends while held, as a socket that
   * was down never sees them: the welcome's refetch is all that brings their changes in.
   */
  readonly missed?: boolean;
}

/**
 * Holds every frame the sync socket sends until the returned function is called, so a test can
 * arrange the timeline before the first `welcome` (and the refetch it starts) arrives.
 */
export async function holdSync(
  page: Page,
  { missed = false }: HoldOptions = {},
): Promise<() => void> {
  const held = Promise.withResolvers<void>();
  let holding = true;

  await page.routeWebSocket(/\/api\/v1\/sync/, (socket) => {
    const server = socket.connectToServer();

    server.onMessage(async (message) => {
      if (missed && holding && !String(message).includes('"t":"welcome"')) {
        return;
      }

      await held.promise;
      socket.send(message);
    });
  });

  return () => {
    holding = false;
    held.resolve();
  };
}

interface MockPost {
  readonly roomId: number;
  readonly userId: number;
  readonly markdown: string;
}

/** Has someone post in a room through the mock's `/__mock/post` control. */
export async function postMessage(request: APIRequestContext, body: MockPost): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/post", { headers: { "X-CSRF-Token": state.csrfToken }, data: body });
}

/** Restores an ordinary thread for scenarios that need its full conversation viewport. */
export async function stopTrackingThread(
  request: APIRequestContext,
  threadId: number,
): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.patch(`/api/v1/threads/${threadId}/work`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { status: null, ownerId: null },
  });

  expect(response.ok()).toBe(true);
}

/** Opens the app at `path` (under /app/) in `theme`, with motion reduced so shots are settled. */
/** Saves the account's appearance through the API, as another device (or person) would. */
export async function saveAccountAppearance(
  page: Page,
  change: Partial<Record<"theme" | "textSize" | "timeZone", string>>,
): Promise<void> {
  const state = await (await page.request.get("/__mock/state")).json();

  const response = await page.request.patch("/api/v1/settings/appearance", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { theme: null, textSize: null, timeZone: null, ...change },
  });

  if (!response.ok()) {
    throw new Error(`saving the appearance answered ${response.status()}`);
  }
}

export async function openApp(page: Page, path: string, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path.replace(/^\//, "")}`);
  // Phones show one column: the conversation list, or the open conversation.
  await page
    .getByRole("complementary", { name: "Conversations" })
    .or(page.getByRole("main"))
    .first()
    .waitFor();
}

/**
 * Opens a room tool from the room header: its button, or on phones (under 720 px) its item in the
 * ⋯ menu. A menu ignores a click on its trigger just after it closed, so the phone path tries again.
 */
export async function openHeaderTool(page: Page, name: RegExp | string): Promise<void> {
  const header = page.locator(".room-header");

  if ((page.viewportSize()?.width ?? 0) >= 720) {
    await header.getByRole("button", { name }).first().click();

    return;
  }

  const menu = page.getByRole("menu", { name: "More" });

  await expectBase(async () => {
    await header.getByRole("button", { name: "More" }).click();
    await expectBase(menu).toBeVisible({ timeout: 1000 });
  }).toPass();
  await menu.getByRole("menuitem", { name }).click();
}

/**
 * Resolves once the page's sync socket is welcomed. Call it before the page opens; await it
 * before a step that publishes an event the page must receive live. Until the welcome, a change
 * made elsewhere arrives only through the catch-up reload, not as an event.
 */
export function syncWelcomed(page: Page): Promise<void> {
  return page
    .waitForEvent("websocket", (socket) => socket.url().includes("/api/v1/sync"))
    .then((socket) =>
      socket.waitForEvent("framereceived", (frame) =>
        String(frame.payload).includes('"t":"welcome"'),
      ),
    )
    .then(() => undefined);
}

const SHOTS = process.env.SMARTFIRE_SHOTS === "1";

const SHOTS_DIR = process.env.SMARTFIRE_SHOTS_DIR ?? join(homedir(), ".cache/frontend-s2/shots");

/**
 * Saves `<name>-<theme>-<desktop|tablet|phone>.png` when `SMARTFIRE_SHOTS=1`; otherwise a no-op, so the
 * specs double as the screenshot script without writing files on every run.
 */
export async function shot(page: Page, name: string, theme: Theme): Promise<void> {
  if (!SHOTS) return;

  mkdirSync(SHOTS_DIR, { recursive: true });

  const width = page.viewportSize()?.width ?? DESKTOP.width;
  const size = width < 720 ? "phone" : width < 1100 ? "tablet" : "desktop";

  await page.screenshot({ path: join(SHOTS_DIR, `${name}-${theme}-${size}.png`) });
}

/** Runs `body` once per theme and viewport: four combinations, each its own test. */
export function matrix(
  title: string,
  body: (context: { page: Page; theme: Theme; phone: boolean }) => Promise<void>,
): void {
  for (const theme of ["light", "dark"] as const) {
    for (const [label, viewport] of [
      ["desktop", DESKTOP],
      ["phone", PHONE],
    ] as const) {
      test(`${title} (${theme}, ${label})`, async ({ page }) => {
        await page.setViewportSize(viewport);
        await body({ page, theme, phone: viewport === PHONE });
      });
    }
  }
}

/**
 * Asserts nothing scrolls sideways: not the page, and no box inside it (a pane body wider than the
 * screen, say), naming the culprits if so. Code blocks and text fields may; pass `allowScroll` (a
 * selector) for any other deliberate sideways scroller, such as a chip strip.
 */
export async function expectNoHorizontalOverflow(
  page: Page,
  { allowScroll }: { readonly allowScroll?: string | undefined } = {},
): Promise<void> {
  const overflow = await page.evaluate((allowed) => {
    const width = window.innerWidth;
    const exempt = ["pre", "textarea", allowed].filter(Boolean).join(", ");

    const wide = [...document.querySelectorAll("body *")]
      .flatMap((element) => {
        const right = element.getBoundingClientRect().right;

        return right > width + 1
          ? [{ right, name: `${element.tagName.toLowerCase()}.${element.className} ${right}` }]
          : [];
      })
      .sort((a, b) => b.right - a.right)
      .slice(0, 5)
      .map(({ name }) => name);

    const scrollers = [...document.querySelectorAll<HTMLElement>("body *")].flatMap((element) => {
      const { overflowX } = getComputedStyle(element);

      const scrolls =
        (overflowX === "auto" || overflowX === "scroll") &&
        element.scrollWidth > element.clientWidth + 1 &&
        element.closest(exempt) === null;

      return scrolls
        ? [
            `${element.tagName.toLowerCase()}.${element.className} ${element.scrollWidth}/${element.clientWidth}`,
          ]
        : [];
    });

    return { scrollWidth: document.documentElement.scrollWidth, width, wide, scrollers };
  }, allowScroll);

  expect(
    overflow.scrollWidth,
    `scrollWidth ${overflow.scrollWidth} > ${overflow.width}: ${overflow.wide.join(", ")}`,
  ).toBeLessThanOrEqual(overflow.width);
  expect(overflow.scrollers, "boxes that scroll sideways").toEqual([]);
}

const TAPPABLE = [
  "a[href]",
  "button",
  'input:not([type="hidden"])',
  "select",
  "textarea",
  "summary",
  '[role="button"]',
  '[role="link"]',
  '[role="tab"]',
  '[role="menuitem"]',
  '[role="menuitemradio"]',
  '[role="menuitemcheckbox"]',
  '[role="option"]',
  '[role="checkbox"]',
  '[role="radio"]',
  '[role="switch"]',
].join(", ");

/**
 * Asserts every visible tappable thing under `selector` is at least `min` px (--touch-target) both
 * ways. It measures the element's own box, so a pseudo-element hit area doesn't count: size the
 * element itself. Visually hidden elements and links in running text (exempt in WCAG 2.5.8) are
 * skipped; pass `ignore` (a selector) for anything else deliberately small.
 */
export async function expectTouchTargets(
  page: Page,
  selector = "body",
  { min = 44, ignore }: { readonly min?: number; readonly ignore?: string } = {},
): Promise<void> {
  const small = await page.locator(selector).evaluate(
    (element, { tappable, min, ignore }) =>
      [...element.querySelectorAll<HTMLElement>(tappable)].flatMap((target) => {
        const box = target.getBoundingClientRect();
        const style = getComputedStyle(target);

        // Visually hidden (the .visually-hidden clip) is skipped; a visible 1 px target is not.
        const skipped =
          box.width === 0 ||
          box.height === 0 ||
          style.visibility === "hidden" ||
          target.closest(".visually-hidden") !== null ||
          style.clipPath === "inset(50%)" ||
          (target.tagName === "A" && style.display === "inline") ||
          (ignore !== undefined && target.matches(ignore));

        if (skipped || (box.width >= min - 0.5 && box.height >= min - 0.5)) {
          return [];
        }

        const name =
          target.getAttribute("aria-label") ?? target.textContent?.trim().slice(0, 32) ?? "";

        return [`${target.tagName.toLowerCase()} "${name}" ${box.width}x${box.height}`];
      }),
    { tappable: TAPPABLE, min, ignore },
  );

  expect(small, `tap targets under ${min}px`).toEqual([]);
}

/**
 * Raises an on-screen keyboard `height` px tall that overlays the page, as iOS Safari's does: the
 * visual viewport shrinks and the layout viewport holds. `offsetTop` pans the visible area down
 * the layout viewport, as iOS does to bring a field above the keyboard (`pageTop` follows). 0
 * lowers it. The app reads it as a keyboard only while a text field has focus. Resolves once the
 * page has had a frame to respond. For a keyboard that resizes the page instead (Android), shrink
 * the viewport with `page.setViewportSize`.
 */
export async function simulateKeyboard(
  page: Page,
  height: number,
  { offsetTop = 0 }: { readonly offsetTop?: number } = {},
): Promise<void> {
  await page.evaluate(
    async ({ keyboard, pan }) => {
      const viewport = window.visualViewport;

      if (viewport === null) {
        throw new Error("this browser has no visualViewport");
      }

      const readings = {
        height: () => document.documentElement.clientHeight - keyboard,
        offsetTop: () => pan,
        pageTop: () => window.scrollY + pan,
      };

      for (const [name, read] of Object.entries(readings)) {
        if (keyboard === 0) {
          Reflect.deleteProperty(viewport, name);
        } else {
          Object.defineProperty(viewport, name, { configurable: true, get: read });
        }
      }

      viewport.dispatchEvent(new Event("resize"));
      viewport.dispatchEvent(new Event("scroll"));

      await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
    },
    { keyboard: height, pan: offsetTop },
  );
}

/**
 * Gives the page a notch and a home indicator (an iPhone's, by default). Chromium reports no safe
 * areas, so this sets the --safe-* tokens that env() would, in the tokens layer, where an open
 * keyboard still zeroes the bottom one.
 */
export async function simulateSafeAreas(
  page: Page,
  { top = 47, right = 0, bottom = 34, left = 0 } = {},
): Promise<void> {
  await page.addStyleTag({
    content: `@layer tokens { :root { --safe-top: ${top}px; --safe-right: ${right}px; --safe-bottom: ${bottom}px; --safe-left: ${left}px; } }`,
  });
}
