import {
  DESKTOP,
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  openApp,
  PHONE_SMALL,
  PHONE_TOUCH,
  ROOM_IDS,
  saveAccountAppearance,
  shot,
  simulateKeyboard,
  simulateSafeAreas,
  test,
} from "./support.ts";

/**
 * The phone foundation every phone screen builds on: the viewport, the safe areas, the keyboard,
 * the touch sizes and the 16 px fields, on a 360 px touch phone, and none of it on a desktop.
 */

const GENERAL = `r/${ROOM_IDS.general}`;

/** A custom property as <html> computes it. */
function rootToken(page: import("@playwright/test").Page, name: string): Promise<string> {
  return page.evaluate(
    (token) => getComputedStyle(document.documentElement).getPropertyValue(token).trim(),
    name,
  );
}

/** Every text field on the page whose computed font size is under 16 px. */
function smallFields(page: import("@playwright/test").Page): Promise<string[]> {
  return page.evaluate(() =>
    [
      ...document.querySelectorAll<HTMLElement>(
        'input:not([type="checkbox"], [type="radio"], [type="range"], [type="color"], [type="hidden"]), textarea, select, [contenteditable="true"]',
      ),
    ].flatMap((field) => {
      const size = Number.parseFloat(getComputedStyle(field).fontSize);
      const name = field.getAttribute("aria-label") ?? field.getAttribute("name") ?? "";

      return field.getClientRects().length > 0 && size < 16
        ? [`${field.tagName.toLowerCase()}.${field.className} "${name}" ${size}px`]
        : [];
    }),
  );
}

test.describe("on a 360 px touch phone", () => {
  test.use(PHONE_TOUCH);

  test("the page draws under the notch, and Android resizes it for the keyboard", async ({
    page,
  }) => {
    await openApp(page, "");

    await expect(page.locator('meta[name="viewport"]')).toHaveAttribute(
      "content",
      /viewport-fit=cover.*interactive-widget=resizes-content/,
    );
    expect(await page.evaluate(() => matchMedia("(pointer: coarse)").matches)).toBe(true);
  });

  // The settings and workspace section strips scroll sideways on purpose until they become lists.
  for (const [screen, path, allowScroll] of [
    ["home", "", undefined],
    ["room", GENERAL, undefined],
    ["settings", "settings", ".settings-nav"],
    ["workspace admin", "admin/people", ".settings-nav"],
  ] as const) {
    test(`nothing on ${screen} scrolls sideways`, async ({ page }) => {
      await openApp(page, path);
      await expect(page.locator(".app-shell")).toBeVisible();
      await expectNoHorizontalOverflow(page, { allowScroll });
    });
  }

  test("text fields are at least 16 px, so iOS never zooms in on focus", async ({ page }) => {
    // The smallest account text size: fields hold at 16 px even when the rest of the text shrinks.
    await openApp(page, "");
    await saveAccountAppearance(page, { textSize: "smaller" });

    for (const path of [GENERAL, `${GENERAL}/events/new`, "settings", "search", "people"]) {
      await openApp(page, path);
      await expect(
        page.locator("input, textarea").locator("visible=true").first(),
        path,
      ).toBeVisible();
      expect(await smallFields(page), path).toEqual([]);
    }
  });

  test("controls take the touch size", async ({ page }) => {
    await openApp(page, "settings");

    expect(await rootToken(page, "--control-md")).toBe("44px");
    expect(await rootToken(page, "--control-lg")).toBe("44px");
    await expectTouchTargets(page, ".settings-form");
  });

  test("the shell keeps clear of the notch and the home indicator", async ({ page }) => {
    await openApp(page, "");
    await simulateSafeAreas(page, { top: 47, bottom: 34 });

    // The list: its header under the status bar, the tab bar above the home indicator.
    const header = await page.locator(".sidebar-header").boundingBox();

    expect(header?.y).toBe(47);
    expect(
      await page.locator(".rail").evaluate((rail) => getComputedStyle(rail).paddingBottom),
    ).toBe("34px");

    // A room: its header under the status bar, the composer above the home indicator.
    await openApp(page, GENERAL);
    await simulateSafeAreas(page, { top: 47, bottom: 34 });

    const room = await page.locator(".room-header").boundingBox();
    const composer = await page.locator(".composer").boundingBox();

    expect(room?.y).toBe(47);
    expect((composer?.y ?? 0) + (composer?.height ?? 0)).toBeLessThanOrEqual(
      PHONE_SMALL.height - 34,
    );
    await shot(page, "phone-safe-areas", "light");
  });

  test("an overlaid keyboard shrinks the shell, keeping the header and the composer", async ({
    page,
  }) => {
    await openApp(page, GENERAL);
    await simulateSafeAreas(page, { bottom: 34 });
    await page.getByRole("textbox", { name: "Message #general" }).focus();
    await simulateKeyboard(page, 300);

    const html = page.locator("html");

    await expect(html).toHaveAttribute("data-keyboard", "open");
    expect(await rootToken(page, "--keyboard-inset")).toBe("300px");
    // The keyboard covers the home indicator, so nothing keeps clear of it any more.
    expect(await rootToken(page, "--safe-bottom")).toBe("0px");
    await expect
      .poll(async () => (await page.locator(".app-shell").boundingBox())?.height)
      .toBe(PHONE_SMALL.height - 300);
    expect((await page.locator(".room-header").boundingBox())?.y).toBe(47);

    const composer = await page.locator(".composer").boundingBox();

    expect((composer?.y ?? 0) + (composer?.height ?? 0)).toBe(PHONE_SMALL.height - 300);
    await shot(page, "phone-keyboard-overlaid", "light");

    await simulateKeyboard(page, 0);
    await expect(html).not.toHaveAttribute("data-keyboard");
    expect(await rootToken(page, "--keyboard-inset")).toBe("0px");
    await expect
      .poll(async () => (await page.locator(".app-shell").boundingBox())?.height)
      .toBe(PHONE_SMALL.height);
  });

  test("when iOS pans the page up to a field, the shell follows the visible area", async ({
    page,
  }) => {
    await openApp(page, GENERAL);
    await page.getByRole("textbox", { name: "Message #general" }).focus();
    await simulateKeyboard(page, 300, { offsetTop: 200 });

    // The visible area is layout px 200–640: the shell fills it, header at its top edge.
    expect(await rootToken(page, "--viewport-top-inset")).toBe("200px");
    expect(await rootToken(page, "--keyboard-inset")).toBe("100px");
    await expect
      .poll(async () => await page.locator(".app-shell").boundingBox())
      .toMatchObject({ y: 200, height: PHONE_SMALL.height - 300 });

    const header = await page.locator(".room-header").boundingBox();
    const composer = await page.locator(".composer").boundingBox();

    expect(header?.y).toBe(200);
    expect((composer?.y ?? 0) + (composer?.height ?? 0)).toBe(200 + PHONE_SMALL.height - 300);

    await simulateKeyboard(page, 0);
    await expect
      .poll(async () => await page.locator(".app-shell").boundingBox())
      .toMatchObject({ y: 0, height: PHONE_SMALL.height });
  });

  test("pinch zoom isn't taken for a keyboard", async ({ page }) => {
    await openApp(page, GENERAL);
    await page.evaluate(async () => {
      const viewport = window.visualViewport;

      if (viewport === null) {
        return;
      }

      Object.defineProperty(viewport, "scale", { configurable: true, get: () => 2 });
      Object.defineProperty(viewport, "height", {
        configurable: true,
        get: () => window.innerHeight / 2,
      });
      Object.defineProperty(viewport, "offsetTop", { configurable: true, get: () => 100 });
      viewport.dispatchEvent(new Event("resize"));
      await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
    });

    expect(await rootToken(page, "--keyboard-inset")).toBe("0px");
    expect(await rootToken(page, "--viewport-top-inset")).toBe("0px");
    await expect(page.locator("html")).not.toHaveAttribute("data-keyboard");
    expect((await page.locator(".app-shell").boundingBox())?.height).toBe(PHONE_SMALL.height);
  });

  test("a resizing keyboard shrinks the shell with the viewport", async ({ page }) => {
    await openApp(page, GENERAL);
    await page.getByRole("textbox", { name: "Message #general" }).focus();
    await page.setViewportSize({ width: PHONE_SMALL.width, height: 420 });

    await expect(page.locator("html")).toHaveAttribute("data-keyboard", "open");
    expect(await rootToken(page, "--keyboard-inset")).toBe("0px");
    await expect
      .poll(async () => (await page.locator(".app-shell").boundingBox())?.height)
      .toBe(420);
    await expect(page.getByRole("textbox", { name: "Message #general" })).toBeInViewport();
    await expect(page.locator(".room-header")).toBeInViewport();
  });

  test("toasts keep clear of the keyboard", async ({ page }) => {
    await openApp(page, GENERAL);
    await simulateKeyboard(page, 300);

    expect(
      await page.locator(".toaster").evaluate((toaster) => getComputedStyle(toaster).bottom),
    ).toBe("316px");
  });

  test("the browser bar follows a theme pinned against the system's", async ({ page }) => {
    await openApp(page, "");
    await saveAccountAppearance(page, { theme: "dark" });
    await openApp(page, "", "light");

    const color = () =>
      page.evaluate(
        () =>
          [...document.querySelectorAll<HTMLMetaElement>('meta[name="theme-color"]')].find(
            (meta) => matchMedia(meta.media || "all").matches,
          )?.content,
      );

    expect(await color()).toBe("#141619");

    await saveAccountAppearance(page, { theme: "system" });
    await openApp(page, "", "dark");
    expect(await color()).toBe("#141619");
    await page.emulateMedia({ colorScheme: "light" });
    expect(await color()).toBe("#f1f4f6");
  });

  test("the touch-target check flags a small target and passes a large one", async ({ page }) => {
    await openApp(page, "");
    await page.evaluate(() => {
      const fixture = document.createElement("div");

      fixture.id = "touch-fixture";
      fixture.innerHTML =
        '<button style="width:44px;height:44px">Big</button><p>Read <a href="#x">inline</a></p>';
      document.body.append(fixture);
    });
    await expectTouchTargets(page, "#touch-fixture");

    await page.evaluate(() => {
      document
        .querySelector("#touch-fixture")
        ?.insertAdjacentHTML("beforeend", '<button style="width:30px;height:30px">Small</button>');
    });
    await expect(expectTouchTargets(page, "#touch-fixture")).rejects.toThrow(/Small.*30x30/);
  });

  test("the touch-target check skips visually hidden things but not a visible sliver", async ({
    page,
  }) => {
    await openApp(page, "");
    await page.evaluate(() => {
      document.body.insertAdjacentHTML(
        "beforeend",
        '<div id="sliver-fixture"><button class="visually-hidden">Skip</button></div>',
      );
    });
    await expectTouchTargets(page, "#sliver-fixture");

    await page.evaluate(() => {
      document
        .querySelector("#sliver-fixture")
        ?.insertAdjacentHTML("beforeend", '<button style="width:1px;height:44px">Sliver</button>');
    });
    await expect(expectTouchTargets(page, "#sliver-fixture")).rejects.toThrow(/Sliver.*1x44/);
  });

  test("the overflow check catches a box scrolling sideways inside the page", async ({ page }) => {
    await openApp(page, "");
    await page.evaluate(() => {
      document.body.insertAdjacentHTML(
        "beforeend",
        '<div id="wide-fixture" style="position:fixed;inset:0 auto auto 0;width:200px;overflow-x:auto"><div style="width:600px;height:10px"></div></div>',
      );
    });

    await expect(expectNoHorizontalOverflow(page)).rejects.toThrow(/wide-fixture|600\/200/);
    await expectNoHorizontalOverflow(page, { allowScroll: "#wide-fixture" });
  });
});

test.describe("on a touch screen with the desktop layout", () => {
  test.use({ viewport: DESKTOP, hasTouch: true, isMobile: true });

  test("fields and controls keep their desktop sizes", async ({ page }) => {
    await openApp(page, "settings");

    expect(await page.evaluate(() => matchMedia("(pointer: coarse)").matches)).toBe(true);
    expect(await rootToken(page, "--control-md")).toBe("32px");
    expect(
      await page
        .locator("input.input")
        .first()
        .evaluate((field) => getComputedStyle(field).fontSize),
    ).toBe("14px");
  });
});

test.describe("on a desktop", () => {
  test.use({ viewport: DESKTOP });

  test("the phone tokens are inert", async ({ page }) => {
    await openApp(page, "settings");
    await simulateKeyboard(page, 0);

    expect(await rootToken(page, "--control-md")).toBe("32px");
    expect(await rootToken(page, "--keyboard-inset")).toBe("0px");
    expect(await rootToken(page, "--safe-bottom")).toBe("0px");
    await expect(page.locator("html")).not.toHaveAttribute("data-keyboard");
    expect((await page.locator(".app-shell").boundingBox())?.height).toBe(DESKTOP.height);
    expect(
      await page
        .locator("input.input")
        .first()
        .evaluate((field) => getComputedStyle(field).fontSize),
    ).toBe("14px");
  });

  test("the window shrinking isn't taken for a keyboard", async ({ page }) => {
    await openApp(page, GENERAL);
    await page.setViewportSize({ width: DESKTOP.width, height: 500 });

    await expect
      .poll(async () => (await page.locator(".app-shell").boundingBox())?.height)
      .toBe(500);
    await expect(page.locator("html")).not.toHaveAttribute("data-keyboard");
  });
});
