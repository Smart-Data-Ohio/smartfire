import type { APIRequestContext, Locator, Page } from "@playwright/test";
import { seededReplyId, THREAD_IDS } from "../../mock/s2/seed.ts";
import {
  expect,
  expectTouchTargets,
  matrix,
  PHONE_TOUCH,
  postMessage,
  ROOM_IDS,
  shot,
  simulateKeyboard,
  TABLET,
  type Theme,
  test,
  USER_IDS,
} from "./support.ts";

/**
 * Huddles against the mock backend and the fake LiveKit transport (`mock://` credentials): the
 * dock, the call view, the device check, voice rooms in the sidebar, moderation, a host's mute
 * and remote screen shares. `window.__smartfireHuddle` steers the fake.
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

type Box = {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
};

async function box(locator: Locator): Promise<Box> {
  const found = await locator.boundingBox();

  expect(found, "the element is laid out").not.toBeNull();

  return found ?? { x: 0, y: 0, width: 0, height: 0 };
}

/** `inner` lies wholly inside `outer`, give or take a pixel's rounding. */
function expectInside(inner: Box, outer: Box, what: string): void {
  expect(inner.x, `${what}: left edge`).toBeGreaterThanOrEqual(outer.x - 1);
  expect(inner.y, `${what}: top edge`).toBeGreaterThanOrEqual(outer.y - 1);
  expect(inner.x + inner.width, `${what}: right edge`).toBeLessThanOrEqual(
    outer.x + outer.width + 1,
  );
  expect(inner.y + inner.height, `${what}: bottom edge`).toBeLessThanOrEqual(
    outer.y + outer.height + 1,
  );
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

test("connection details include the active video's sampled fields", async ({ page }) => {
  await open(page, GENERAL);
  await join(page);
  await dock(page).getByRole("button", { name: "Turn camera on", exact: true }).click();
  await dock(page).getByRole("button", { name: "Call settings" }).click();
  await dock(page).getByRole("button", { name: "Connection details", exact: true }).click();
  const row = dock(page).locator(".huddle-stats div").filter({ hasText: "sending camera" });
  await expect(row).toContainText("Resolution 640×360");
  await expect(row).toContainText("FPS 15");
  await expect(row).toContainText("Codec video/VP8");
  await expect(row).toContainText("RID –");
  await expect(row).toContainText("Freezes –");
  await expect(row).toContainText("Limit –");
});

test("an ordinary screen share starts at the quality picked in call settings", async ({ page }) => {
  await open(page, GENERAL);
  await join(page);
  await dock(page).getByRole("button", { name: "Call settings" }).click();

  const quality = dock(page).getByLabel("Screen share quality");

  await expect(quality).toHaveValue("1080p15");
  await quality.selectOption("1080p60");
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("campfire.huddle.shareQuality")))
    .toBe("1080p60");
  await dock(page).getByRole("button", { name: "Share your screen" }).click();
  await expect(dock(page).getByRole("button", { name: "Stop sharing" })).toBeVisible();
  // A running share keeps the quality it started with.
  await expect(quality).toBeDisabled();
  await dock(page).getByRole("button", { name: "Stop sharing" }).click();
  await expect(quality).toBeEnabled();
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

test("a channel's voice bitrate is what its next call's microphone encodes at", async ({
  page,
}, testInfo) => {
  const settings = async () => {
    await page.goto(`/app/r/${GENERAL}/settings`);

    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    return { dialog, bitrate: dialog.getByRole("combobox", { name: "Voice bitrate" }) };
  };

  const encoded = () => page.evaluate(() => window.__smartfireHuddle?.audioBitrate());

  await open(page, GENERAL);

  let { dialog, bitrate } = await settings();

  await expect(bitrate).toHaveValue("");
  await expect(bitrate.locator("option:checked")).toHaveText("Default (48 kbps)");
  await bitrate.selectOption({ label: "256 kbps" });
  await page.screenshot({ path: testInfo.outputPath("voice-bitrate.png") });
  await dialog.getByRole("button", { name: "Save changes" }).click();
  await expect(dialog).toBeHidden();

  await join(page);
  await page.waitForFunction(() => window.__smartfireHuddle !== undefined);
  expect(await encoded()).toBe(256_000);
  await dock(page).getByRole("button", { name: "Leave call" }).click();
  await expect(dock(page)).toHaveCount(0);

  ({ dialog, bitrate } = await settings());
  await expect(bitrate).toHaveValue("256000");
  await bitrate.selectOption({ label: "Default (48 kbps)" });
  await dialog.getByRole("button", { name: "Save changes" }).click();
  await expect(dialog).toBeHidden();

  await join(page);
  expect(await encoded()).toBe(48_000);
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

test("theater mode keeps the room's chat beside the share", async ({ page, request }) => {
  await open(page, LOUNGE);
  await join(page);
  await hook(page, `window.__smartfireHuddle.remoteScreen(${USER_IDS.jonah}, true)`);
  await page.locator(".call-share").getByRole("button", { name: "Theater mode" }).click();

  const view = page.locator("section.call-view[data-theater]");
  const composer = page.getByRole("textbox", { name: /^Message/ });

  await expect(view).toHaveAttribute("data-chat", "true");
  await expect(composer).toBeVisible();

  // The chat is a column beside the stage, not under it.
  const stage = await view.boundingBox();
  const field = await composer.boundingBox();

  expect(field?.x ?? 0, "the composer sits right of the stage").toBeGreaterThanOrEqual(
    (stage?.x ?? 0) + (stage?.width ?? 0) - 1,
  );

  await composer.fill("Watching along in theater mode");
  await composer.press("Enter");
  await expect(
    page.locator("[data-message-row]", { hasText: "Watching along in theater mode" }),
  ).toHaveAttribute("data-message-id", /^\d+$/);

  // Someone else's message arrives live beside the share.
  await postMessage(request, {
    roomId: LOUNGE,
    userId: USER_IDS.maya,
    markdown: "Nice stream, Jonah",
  });
  await expect(page.locator("[data-message-row]", { hasText: "Nice stream, Jonah" })).toBeVisible();
  await expect(view.locator(".call-share[data-expanded] video")).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "huddle-theater-chat", "dark");

  // Hidden, the stage covers the room again; the choice holds when the share expands again.
  await view.getByRole("button", { name: "Hide chat" }).click();
  await expect(view).not.toHaveAttribute("data-chat");
  await view.getByRole("button", { name: "Exit theater mode" }).click();
  await page.locator(".call-share").getByRole("button", { name: "Theater mode" }).click();
  await expect(view).toBeVisible();
  await expect(view).not.toHaveAttribute("data-chat");
  await view.getByRole("button", { name: "Show chat" }).click();
  await expect(view).toHaveAttribute("data-chat", "true");
});

test("on a tablet, theater mode stacks the share over the room's chat", async ({ page }) => {
  await page.setViewportSize(TABLET);
  await open(page, LOUNGE);
  await join(page);
  await hook(page, `window.__smartfireHuddle.remoteScreen(${USER_IDS.jonah}, true)`);
  await page.locator(".call-share").getByRole("button", { name: "Theater mode" }).click();

  const view = page.locator("section.call-view[data-theater]");
  const share = view.locator(".call-share[data-expanded]");
  const composer = page.getByRole("textbox", { name: /^Message/ });

  await expect(view).toHaveAttribute("data-chat", "true");

  const video = await share.boundingBox();
  const field = await composer.boundingBox();

  expect(field?.y ?? 0, "the composer sits under the share").toBeGreaterThanOrEqual(
    (video?.y ?? 0) + (video?.height ?? 0),
  );
  await expect(page.locator(".timeline")).toBeVisible();
});

test("beside an open thread, theater mode still fits the share and the chat", async ({
  page,
  request,
}) => {
  // 1100 px leaves the room under 400 px once the thread pane is open.
  await page.setViewportSize({ width: 1100, height: 800 });
  await postMessage(request, {
    roomId: LOUNGE,
    userId: USER_IDS.maya,
    markdown: "A thread about the stream",
  });
  await open(page, LOUNGE);
  await join(page);
  await hook(page, `window.__smartfireHuddle.remoteScreen(${USER_IDS.jonah}, true)`);
  await page.locator(".call-share").getByRole("button", { name: "Theater mode" }).click();

  const view = page.locator("section.call-view[data-theater]");
  const chat = page.locator(".room > .room-part").last();
  const message = chat.locator("[data-message-row]", { hasText: "A thread about the stream" });
  const actions = message.getByRole("toolbar", { name: "Message actions" });

  await expect(view).toHaveAttribute("data-chat", "true");
  await expect(async () => {
    await page.mouse.move(0, 0);
    await message.hover();
    await expect(actions).toBeVisible({ timeout: 1000 });
  }).toPass();
  await actions.getByRole("button", { name: "Reply in thread" }).click();
  await expect(page.locator("aside.right-pane")).toBeVisible();
  await expect(view).toHaveAttribute("data-chat", "true");

  const room = await box(page.locator(".room"));
  const video = await box(view.locator(".call-share[data-expanded] video"));

  expect(video.width, "the share keeps a usable width").toBeGreaterThanOrEqual(240);
  expectInside(video, room, "the share");
  expectInside(
    await box(view.getByRole("button", { name: "Exit theater mode" })),
    room,
    "the share's controls",
  );

  const composer = chat.getByRole("textbox", { name: /^Message/ });
  const send = chat.getByRole("button", { name: "Send message", exact: true });

  await composer.fill("Sent with the button beside a thread");
  expectInside(await box(send), await box(chat), "the send button");
  await send.click({ timeout: 5000 });
  await expect(
    chat.locator("[data-message-row]", { hasText: "Sent with the button beside a thread" }),
  ).toHaveAttribute("data-message-id", /^\d+$/);
});

test("stacked theater mode keeps the composer's suggestions over the share", async ({ page }) => {
  // A short landscape window: the share and the composer leave the timeline little room.
  await page.setViewportSize({ width: 1024, height: 480 });
  await open(page, LOUNGE);
  await join(page);
  await hook(page, `window.__smartfireHuddle.remoteScreen(${USER_IDS.jonah}, true)`);
  await page.locator(".call-share").getByRole("button", { name: "Theater mode" }).click();

  const view = page.locator("section.call-view[data-theater]");
  const chat = page.locator(".room > .room-part").last();
  const composer = chat.getByRole("textbox", { name: /^Message/ });

  await expect(view).toHaveAttribute("data-chat", "true");
  await composer.click();
  await composer.pressSequentially("Over to @ma");

  const option = page.getByRole("listbox").getByRole("option", { name: /Maya Okafor/ });

  await expect(option).toBeVisible();

  // The list reaches up over the share, where the call used to cover it.
  const list = await box(page.getByRole("listbox"));
  const share = await box(view.locator(".call-share[data-expanded]"));

  expect(list.y, "the suggestions overlap the share").toBeLessThan(share.y + share.height);
  await option.click({ timeout: 5000 });
  await expect(composer).toHaveValue(`Over to <@${USER_IDS.maya}> `);
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

  test("theater mode on a phone: the share on top, the chat under it", async ({ page }) => {
    await open(page, LOUNGE);
    await join(page, true);
    await hook(page, `window.__smartfireHuddle.remoteScreen(${USER_IDS.jonah}, true)`);
    await dock(page, true).getByRole("button", { name: "Show call" }).click();
    await page.locator(".call-share").getByRole("button", { name: "Theater mode" }).click();

    const view = page.locator("section.call-view[data-theater]");
    const share = view.locator(".call-share[data-expanded]");
    const chat = page.locator(".room-part").last();
    const composer = page.getByRole("textbox", { name: /^Message/ });

    await expect(view).toHaveAttribute("data-chat", "true");
    await expect(chat).not.toHaveAttribute("inert");
    await expect(composer).toBeVisible();

    const video = await share.boundingBox();
    const field = await composer.boundingBox();

    expect(video?.height ?? 0, "the share keeps a usable size").toBeGreaterThanOrEqual(150);
    expect(field?.y ?? 0, "the composer sits under the share").toBeGreaterThanOrEqual(
      (video?.y ?? 0) + (video?.height ?? 0),
    );

    await composer.fill("Sent from the phone's theater");
    await page.getByRole("button", { name: "Send message", exact: true }).tap();
    await expect(
      page.locator("[data-message-row]", { hasText: "Sent from the phone's theater" }),
    ).toHaveAttribute("data-message-id", /^\d+$/);
    await expect(share.locator("video")).toBeVisible();
    await shot(page, "huddle-theater-chat-phone", "light");

    // Hidden, the share has the screen to itself and the covered chat is out of reach again.
    await share.getByRole("button", { name: "Hide chat" }).tap();
    await expect(view).not.toHaveAttribute("data-chat");
    await expect(chat).toHaveAttribute("inert", "");

    const viewBox = await view.boundingBox();
    const room = await page.locator(".room").boundingBox();

    expect(viewBox?.height ?? 0, "the call fills the room").toBeGreaterThanOrEqual(
      (room?.height ?? 999) - 1,
    );
  });

  test("theater mode on a phone keeps the composer above an overlaid keyboard", async ({
    page,
  }) => {
    // An iPhone SE whose keyboard leaves 350 px of the page visible.
    const visible = 350;

    await page.setViewportSize({ width: 375, height: 667 });
    await open(page, LOUNGE);
    await join(page, true);
    await hook(page, `window.__smartfireHuddle.remoteScreen(${USER_IDS.jonah}, true)`);
    await dock(page, true).getByRole("button", { name: "Show call" }).click();
    await page.locator(".call-share").getByRole("button", { name: "Theater mode" }).click();

    const view = page.locator("section.call-view[data-theater]");
    const chat = page.locator(".room > .room-part").last();
    const composer = chat.getByRole("textbox", { name: /^Message/ });
    const card = chat.locator(".composer-card");

    await expect(view).toHaveAttribute("data-chat", "true");
    await composer.focus();
    await simulateKeyboard(page, 667 - visible);
    await expect(page.locator("html")).toHaveAttribute("data-keyboard", "open");

    // A draft of a few lines grows the composer; it still ends above the keyboard.
    for (const draft of [
      "Watching on the bus",
      "Watching on the bus\nwith the sound off\nfor now",
    ]) {
      await composer.fill(draft);

      const field = await box(card);
      const send = await box(chat.getByRole("button", { name: "Send message", exact: true }));
      const share = await view.locator(".call-share[data-expanded]").boundingBox();

      expect(field.y + field.height, "the composer ends above the keyboard").toBeLessThanOrEqual(
        visible,
      );
      expect(send.y + send.height, "the send button is above the keyboard").toBeLessThanOrEqual(
        visible,
      );

      if (share !== null && share.height > 0) {
        expect(share.y + share.height, "the share stays above the chat").toBeLessThanOrEqual(
          field.y,
        );
      }
    }
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

test("a full voice room explains why a member cannot join", async ({ page, request }) => {
  const state = await (await request.get("/__mock/state")).json();
  const headers = { "X-CSRF-Token": state.csrfToken };
  const form = await (await request.get(`/api/v1/rooms/${LOUNGE}/edit`)).json();

  const saved = await request.patch(`/api/v1/rooms/${LOUNGE}`, {
    headers,
    data: { type: "voice", userIds: form.userIds, voiceUserLimit: 2 },
  });

  expect(saved.ok()).toBe(true);
  await control(request, "viewer-role", { role: "member" });
  await open(page, LOUNGE);
  await expect(
    page.locator(".room-header").getByRole("status", { name: "2 of 2 voice participants" }),
  ).toHaveText("2/2");
  await launcher(page).click();
  await expect(
    page.getByText("This voice room is full. Try again when someone leaves.").first(),
  ).toBeVisible();
});

test("an AFK move leaves the source call, joins the destination and explains why", async ({
  page,
  request,
}) => {
  await open(page, GENERAL);
  const state = await (await request.get("/__mock/state")).json();

  const created = await request.post("/api/v1/rooms", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { type: "voice", name: "Source", userIds: [USER_IDS.riel], clientRoomId: "afk-source" },
  });

  expect(created.ok()).toBe(true);
  const source = (await created.json()).room.id;

  const saved = await request.patch("/api/v1/admin/workspace", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { afkPolicy: { roomId: LOUNGE, timeoutMinutes: 1 } },
  });

  expect(saved.ok()).toBe(true);
  await open(page, source);
  await join(page);
  await dock(page).getByRole("button", { name: "Mute microphone", exact: true }).click();
  await control(request, "huddle-idle", { roomId: source, seconds: 60 });
  await expect(dock(page).getByText("Lounge", { exact: true })).toBeVisible();
  await expect(
    page.getByText("You were moved to Lounge after 1 minute of inactivity", { exact: true }),
  ).toBeVisible();
  await expect(dock(page).getByRole("status")).toContainText("Huddle active");
  await expect(
    dock(page).getByRole("button", { name: "Unmute microphone", exact: true }),
  ).toBeVisible();
  expect(await page.evaluate(() => window.__smartfireHuddle?.snapshot().microphoneEnabled)).toBe(
    false,
  );
});
