import type { APIRequestContext, Page } from "@playwright/test";
import { seededReplyId, THREAD_IDS } from "../../mock/s2/seed.ts";
import {
  expect,
  expectTouchTargets,
  matrix,
  PHONE_TOUCH,
  ROOM_IDS,
  shot,
  type Theme,
  test,
  USER_IDS,
} from "./support.ts";

/**
 * Huddles against the mock backend and the fake LiveKit transport (`mock://` credentials): the
 * dock, the call view, the device check, voice rooms in the sidebar, moderation, a host's mute,
 * a dropped connection and remote screen shares. `window.__smartfireHuddle` steers the fake.
 */

const GENERAL = ROOM_IDS.general;

const LOUNGE = ROOM_IDS.lounge;

async function control(
  request: APIRequestContext,
  action: string,
  data: Record<string, number | string | boolean>,
): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.post(`/__mock/${action}`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data,
  });

  expect(response.ok()).toBe(true);
}

/** Opens a room with the microphone already allowed (no device check). */
async function open(page: Page, roomId: number, theme: Theme = "light"): Promise<void> {
  await page.context().grantPermissions(["microphone", "camera"]);
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/r/${roomId}`);
  await page.getByRole("main").waitFor();
}

function launcher(page: Page) {
  return page.locator(".room-header .huddle-launcher");
}

function dock(page: Page, phone = false) {
  return page.locator(phone ? ".app-main-dock .huddle-dock" : ".sidebar .huddle-dock");
}

async function join(page: Page, phone = false): Promise<void> {
  await launcher(page).click();
  await expect(dock(page, phone).getByRole("status")).toContainText("Huddle active");
}

/** Calls a hook on the fake transport. */
async function hook(page: Page, script: string): Promise<void> {
  await page.waitForFunction(() => window.__smartfireHuddle !== undefined);
  await page.evaluate(script);
}

matrix("a voice room lists who's in its call", async ({ page, theme, phone }) => {
  await page.context().grantPermissions(["microphone", "camera"]);
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto("/app/");
  await page.getByRole("complementary", { name: "Conversations" }).waitFor();

  const lounge = page.locator(".sidebar-rows li").filter({ hasText: "Lounge" });

  await expect(lounge.getByRole("list", { name: "In the call" })).toContainText("Maya");
  await expect(lounge.getByRole("list", { name: "In the call" })).toContainText("Jonah");
  await expect(page.locator(".sidebar-rows").getByText("Town Hall")).toBeVisible();

  if (!phone) {
    await page.locator(".sidebar-row", { hasText: "Lounge" }).click();
    await expect(launcher(page)).toHaveText("Join voice");
    await expect(page.locator(".huddle-stack")).toBeVisible();
  }

  await page.mouse.move(0, 0);
  await shot(page, "huddle-sidebar", theme);
});

matrix("in a call: the dock and the call view", async ({ page, theme, phone }) => {
  await open(page, LOUNGE, theme);
  await join(page, phone);

  // A phone keeps the conversation on joining; the call bar opens the call view.
  if (phone) {
    await dock(page, true).getByRole("button", { name: "Show call" }).click();
  }

  const view = page.getByRole("region", { name: "Call" }).last();

  await expect(page.locator(".call-tile")).toHaveCount(3);
  await expect(launcher(page)).toHaveText("Leave voice");

  await hook(page, `window.__smartfireHuddle.speak(${USER_IDS.maya}, true)`);
  await expect(page.locator(".call-tile[data-speaking]")).toHaveCount(1);

  if (!phone) {
    await expect(
      page.locator(".voice-participant[data-speaking]", { hasText: "Maya" }),
    ).toBeVisible();
  }

  await expect(view).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "huddle-call", theme);

  if (!phone) {
    await dock(page).getByRole("button", { name: "Call settings" }).click();
    await expect(dock(page).getByRole("region", { name: "Call settings" })).toBeVisible();
    await dock(page).getByRole("button", { name: "Connection details" }).click();
    await expect(dock(page).getByText("42 ms")).toBeVisible();
    await shot(page, "huddle-settings", theme);
  }
});

test("joining and leaving a huddle in a channel", async ({ page }) => {
  await open(page, GENERAL);
  await expect(launcher(page)).toHaveText("Join huddle");
  await join(page);
  await expect(launcher(page)).toHaveText("In huddle");

  const mute = dock(page).getByRole("button", { name: "Mute microphone" });

  await mute.click();
  await expect(dock(page).getByRole("button", { name: "Unmute microphone" })).toBeVisible();

  await dock(page).getByRole("button", { name: "Deafen" }).click();
  await expect(dock(page).getByRole("button", { name: "Undeafen" })).toBeVisible();
  // Unmuting undeafens.
  await dock(page).getByRole("button", { name: "Unmute microphone" }).click();
  await expect(dock(page).getByRole("button", { name: "Deafen" })).toBeVisible();

  await dock(page).getByRole("button", { name: "Turn camera on" }).click();
  await expect(page.locator(".call-tile[data-camera] video")).toHaveCount(1);

  await dock(page).getByRole("button", { name: "Leave call" }).click();
  await expect(dock(page)).toHaveCount(0);
  await expect(launcher(page)).toHaveText("Join huddle");
});

test("the first join checks devices", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${GENERAL}`);
  await page.getByRole("main").waitFor();
  await launcher(page).click();

  const dialog = page.getByRole("dialog", { name: `Join general` });

  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("meter", { name: "Microphone level" })).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Join" })).toBeEnabled();
  await shot(page, "huddle-prejoin", "light");
  await dialog.getByRole("button", { name: "Join" }).click();
  await expect(dock(page).getByRole("status")).toContainText("Huddle active");
});

test("a host's mute rejoins without the microphone and says why", async ({ page, request }) => {
  await open(page, LOUNGE);
  await join(page);
  await control(request, "huddle-mute", { roomId: LOUNGE, userId: 1, muted: true });
  await expect(dock(page).getByText("A host muted you")).toBeVisible();
  await expect(dock(page).getByRole("button", { name: "You can’t speak here" })).toBeDisabled();

  await control(request, "huddle-mute", { roomId: LOUNGE, userId: 1, muted: false });
  await expect(dock(page).getByText("A host muted you")).toHaveCount(0);
  await expect(dock(page).getByRole("button", { name: "Mute microphone" })).toBeEnabled();
});

test("an administrator mutes and removes someone from a voice call", async ({ page }) => {
  await open(page, LOUNGE);
  await join(page);
  await page.getByRole("button", { name: /^Maya Okafor(, speaking)?$/ }).click();

  const menu = page.getByRole("dialog", { name: "Maya Okafor in the call" });

  await expect(menu.getByRole("slider")).toBeVisible();
  await menu.getByRole("button", { name: "Mute for everyone" }).click();
  await expect(
    page.locator(".voice-participant", { hasText: "Maya" }).locator(".voice-participant-muted"),
  ).toBeVisible();

  await page.getByRole("button", { name: /^Maya Okafor(, speaking)?$/ }).click();
  await page
    .getByRole("dialog", { name: "Maya Okafor in the call" })
    .getByRole("button", { name: "Remove from call" })
    .click();
  await expect(page.locator(".voice-participant", { hasText: "Maya" })).toHaveCount(0);
  await expect(page.locator(".call-tile")).toHaveCount(2);
});

test("a dropped connection ends the call with Retry", async ({ page }) => {
  await open(page, GENERAL);
  await join(page);
  await hook(page, "window.__smartfireHuddle.reconnecting()");
  await expect(dock(page).getByRole("status")).toContainText("Reconnecting");
  await hook(page, "window.__smartfireHuddle.reconnected()");
  await expect(dock(page).getByRole("status")).toContainText("Huddle active");

  await hook(page, "window.__smartfireHuddle.disconnect('lost')");
  await expect(dock(page).getByRole("alert")).toContainText("Huddle ended");
  await shot(page, "huddle-failed", "light");
  await dock(page).getByRole("button", { name: "Retry" }).click();
  await expect(dock(page).getByRole("status")).toContainText("Huddle active");
});

test("a remote screen share opens in theater mode", async ({ page }) => {
  await open(page, LOUNGE);
  await join(page);
  await hook(page, `window.__smartfireHuddle.remoteScreen(${USER_IDS.jonah}, true)`);

  const share = page.locator(".call-share");

  await expect(share).toHaveCount(1);
  await share.getByRole("button", { name: "Theater mode" }).click();
  await expect(page.locator(".call-view[data-theater]")).toBeVisible();
  await shot(page, "huddle-theater", "dark");
  await page.getByRole("button", { name: "Exit theater mode" }).click();
  await expect(page.locator(".call-view[data-theater]")).toHaveCount(0);
});

test("Smartfire's motion setting overrides the system one in a call", async ({ page }) => {
  await open(page, LOUNGE);
  await join(page);

  const tile = page.locator(".call-tile").first();

  const transition = () => tile.evaluate((element) => getComputedStyle(element).transitionProperty);

  // The OS asks for reduced motion (open() emulates it), so the tiles do not animate...
  await expect.poll(transition).toBe("none");

  // ...unless the user chose full motion in Smartfire.
  await page.evaluate(() => {
    document.documentElement.dataset.motion = "full";
  });
  await expect.poll(transition).toBe("box-shadow");

  // Choosing reduced motion in Smartfire wins over an OS that allows motion.
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.evaluate(() => {
    document.documentElement.dataset.motion = "reduce";
  });
  await expect.poll(transition).toBe("none");
});

test.describe("on a touch phone", () => {
  test.use(PHONE_TOUCH);

  test("a call keeps the conversation: one compact bar, the call a tap away", async ({ page }) => {
    await open(page, GENERAL);
    await join(page, true);

    const bar = dock(page, true);
    const height = PHONE_TOUCH.viewport.height;

    await expect(page.locator(".call-view")).toHaveCount(0);
    expect((await bar.boundingBox())?.height ?? 999, "a 48 px bar").toBeLessThanOrEqual(48.5);

    const timeline = await page.locator(".app-main .timeline").boundingBox();

    expect(timeline?.height ?? 0, "the timeline keeps most of the screen").toBeGreaterThanOrEqual(
      height * 0.6,
    );
    // The room's name stretches its hit area over the bar's text, so its own box is exempt.
    await expectTouchTargets(page, ".app-main-dock .huddle-dock", { ignore: ".huddle-dock-room" });

    await bar.getByRole("button", { name: "Show call" }).click();

    const view = page.locator(".call-view");

    await expect(view.locator(".call-tile")).toHaveCount(1);

    const viewBox = await view.boundingBox();
    const room = await page.locator(".room").boundingBox();

    expect(viewBox?.height ?? 0, "the call fills the room").toBeGreaterThanOrEqual(
      (room?.height ?? 999) - 1,
    );

    await bar.getByRole("button", { name: "Hide call" }).click();
    await expect(view).toHaveCount(0);
  });

  test("the full-screen call is a page: it takes focus, Escape and Back close it", async ({
    page,
  }) => {
    await open(page, GENERAL);
    await join(page, true);

    const bar = dock(page, true);
    const view = page.locator("section.call-view");
    const url = page.url();

    await bar.getByRole("button", { name: "Show call" }).click();
    await expect(view).toBeFocused();

    // The covered conversation is out of reach: Tab never lands in the composer.
    await expect(page.locator(".room-part").first()).toHaveAttribute("inert", "");

    for (let step = 0; step < 12; step += 1) {
      await page.keyboard.press("Tab");

      const inRoom = await page.evaluate(() =>
        Boolean(document.activeElement?.closest(".room-part")),
      );

      expect(inRoom, `Tab ${step + 1} stays out of the covered room`).toBe(false);
    }

    await view.focus();
    await page.keyboard.press("Escape");
    await expect(view).toHaveCount(0);
    await expect(bar.getByRole("button", { name: "Show call" })).toBeFocused();

    await bar.getByRole("button", { name: "Show call" }).click();
    await expect(view).toBeVisible();
    await page.goBack();
    await expect(view).toHaveCount(0);
    expect(page.url(), "Back closes the call, not the room").toBe(url);
    await expect(page.getByRole("textbox", { name: /^Message/ })).toBeVisible();
  });

  test("Escape on the call bar closes the full-screen call", async ({ page }) => {
    await open(page, GENERAL);
    await join(page, true);

    const bar = dock(page, true);
    const view = page.locator("section.call-view");

    await bar.getByRole("button", { name: "Show call" }).click();
    await expect(view).toBeVisible();
    await bar.getByRole("button", { name: "Mute microphone" }).focus();
    await page.keyboard.press("Escape");
    await expect(view).toHaveCount(0);
  });

  test("the full-screen call is in the URL: Back closes it, Forward reopens it", async ({
    page,
  }) => {
    await open(page, GENERAL);
    await join(page, true);

    const view = page.locator("section.call-view");

    await dock(page, true).getByRole("button", { name: "Show call" }).click();
    await expect(view).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/r/${GENERAL}\\?call=1$`));

    await page.goBack();
    await expect(view).toHaveCount(0);
    await expect(page).toHaveURL(new RegExp(`/r/${GENERAL}$`));

    await page.goForward();
    await expect(view).toBeVisible();
  });

  test("switching rooms with the call open keeps Back and Forward ordinary", async ({ page }) => {
    await open(page, LOUNGE);
    await open(page, GENERAL);
    await join(page, true);

    const view = page.locator("section.call-view");
    const covered = new RegExp(`/r/${GENERAL}\\?call=1$`);
    const general = new RegExp(`/r/${GENERAL}$`);

    await dock(page, true).getByRole("button", { name: "Show call" }).click();
    await expect(view).toBeVisible();
    await page.keyboard.press("Alt+ArrowDown");
    await expect(page).not.toHaveURL(covered);

    const next = page.url();

    // Back returns to each page as it was: the call over the room, then the room alone ...
    await page.goBack();
    await expect(page).toHaveURL(covered);
    await expect(view).toBeVisible();
    await page.goBack();
    await expect(page).toHaveURL(general);
    await expect(view).toHaveCount(0);

    // ... and Forward retraces them to the next room.
    await page.goForward();
    await expect(view).toBeVisible();
    await page.goForward();
    await expect(page).toHaveURL(next);
    await expect(view).toHaveCount(0);
  });

  test("closing the call steps back to the room page it opened over", async ({ page }) => {
    await open(page, GENERAL);
    await join(page, true);

    const view = page.locator("section.call-view");
    const length = await page.evaluate(() => window.history.length);

    await dock(page, true).getByRole("button", { name: "Show call" }).click();
    await expect(view).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(view).toHaveCount(0);
    await expect(page).toHaveURL(new RegExp(`/r/${GENERAL}$`));

    // Stepped back, not replaced: the call is still the next page, one entry past the room.
    expect(await page.evaluate(() => window.history.length)).toBe(length + 1);
    await page.goForward();
    await expect(view).toBeVisible();
  });

  test("a call that ends after Back leaves Forward on the plain room", async ({ page }) => {
    await open(page, GENERAL);
    await join(page, true);

    const bar = dock(page, true);
    const view = page.locator("section.call-view");
    const general = new RegExp(`/r/${GENERAL}$`);
    const index = () => page.evaluate(() => Number(window.history.state?.__TSR_index));
    const roomIndex = await index();

    await bar.getByRole("button", { name: "Show call" }).click();
    await expect(view).toBeVisible();
    await page.goBack();
    await expect(view).toHaveCount(0);
    await bar.getByRole("button", { name: "Leave call" }).click();
    await expect(bar).toHaveCount(0);

    // Forward reaches the call's old entry, now just the room: call=1 replaced off, no bounce.
    await page.goForward();
    await expect(page).toHaveURL(general);
    await expect.poll(index).toBe(roomIndex + 1);
    await page.waitForTimeout(500);
    expect(await index(), "Forward stays put").toBe(roomIndex + 1);
    await expect(view).toHaveCount(0);
  });

  test("a call that ends under its open view takes call=1 off in place", async ({ page }) => {
    await open(page, LOUNGE);
    await open(page, GENERAL);
    await join(page, true);

    const bar = dock(page, true);
    const view = page.locator("section.call-view");
    const general = new RegExp(`/r/${GENERAL}$`);
    const index = () => page.evaluate(() => Number(window.history.state?.__TSR_index));

    await bar.getByRole("button", { name: "Show call" }).click();
    await expect(view).toBeVisible();

    const callIndex = await index();

    await bar.getByRole("button", { name: "Leave call" }).click();
    await expect(view).toHaveCount(0);
    await expect(page).toHaveURL(general);
    expect(await index(), "replaced, not stepped back").toBe(callIndex);

    // Back is ordinary: the room page the call opened over, then the room before it.
    await page.goBack();
    await expect(page).toHaveURL(general);
    await page.goBack();
    await expect(page).toHaveURL(new RegExp(`/r/${LOUNGE}$`));
  });

  test("a call ended elsewhere leaves its old call=1 entry to Back and Forward", async ({
    page,
  }) => {
    await open(page, GENERAL);
    await join(page, true);

    const bar = dock(page, true);
    const general = new RegExp(`/r/${GENERAL}$`);

    await bar.getByRole("button", { name: "Show call" }).click();
    await expect(page.locator("section.call-view")).toBeVisible();
    await page.keyboard.press("Alt+ArrowDown");
    await expect(page).not.toHaveURL(/call=1/);

    const next = page.url();

    await bar.getByRole("button", { name: "Leave call" }).click();
    await expect(bar).toHaveCount(0);

    // Back finds the call gone: the parameter comes off in place, not by stepping back again ...
    await page.goBack();
    await expect(page).toHaveURL(general);
    await expect(page.locator("section.call-view")).toHaveCount(0);

    // ... so Forward still reaches the room after it.
    await page.goForward();
    await expect(page).toHaveURL(next);
  });

  test("taking call=1 off a reply's permalink keeps the thread and the reply", async ({ page }) => {
    const permalink = `/r/${GENERAL}/t/${THREAD_IDS.generalActive}?m=${seededReplyId(THREAD_IDS.generalActive, 0)}`;
    const kept = new RegExp(`${permalink.replace("?", "\\?")}$`);

    // Arriving with no call on: the stale parameter is replaced off.
    await open(page, GENERAL);
    await page.goto(`/app${permalink}&call=1`);
    await expect(page).toHaveURL(kept);

    // In a call, closing the view there (Escape on the bar) takes only `call` away.
    await open(page, GENERAL);
    await join(page, true);
    await page.evaluate((url) => {
      const index = Number(window.history.state?.__TSR_index ?? 0) + 1;

      window.history.pushState(
        { __TSR_index: index, __TSR_key: "permalink", key: "permalink" },
        "",
        url,
      );
      window.dispatchEvent(new PopStateEvent("popstate", { state: window.history.state }));
    }, `/app${permalink}&call=1`);
    await expect(page).toHaveURL(/call=1/);

    await dock(page, true).getByRole("button", { name: "Mute microphone" }).focus();
    await page.keyboard.press("Escape");
    await expect(page).toHaveURL(kept);
  });

  test("a rapid double Back from the open call leaves the room, with no extra stop", async ({
    page,
  }) => {
    await open(page, LOUNGE);
    await open(page, GENERAL);
    await join(page, true);
    await dock(page, true).getByRole("button", { name: "Show call" }).click();
    await expect(page.locator("section.call-view")).toBeVisible();

    await page.evaluate(() => {
      window.history.back();
      window.history.back();
    });

    await expect(page).toHaveURL(new RegExp(`/r/${LOUNGE}$`));
    // Nothing steps on afterwards.
    await page.waitForTimeout(500);
    await expect(page).toHaveURL(new RegExp(`/r/${LOUNGE}$`));
  });
});
