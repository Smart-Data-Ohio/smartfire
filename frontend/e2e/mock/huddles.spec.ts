import type { APIRequestContext, Page } from "@playwright/test";
import { expect, matrix, ROOM_IDS, shot, type Theme, test, USER_IDS } from "./support.ts";

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
