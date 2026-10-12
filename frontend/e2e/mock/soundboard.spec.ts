import type { APIRequestContext, Page } from "@playwright/test";
import { tinyWav } from "../../mock/s5/soundboard.ts";
import {
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  matrix,
  openApp,
  PHONE_TOUCH,
  ROOM_IDS,
  test,
  USER_IDS,
} from "./support.ts";

/**
 * The voice soundboard against the mock backend and the fake LiveKit transport: an administrator
 * adds a sound, it shows in the call's picker, and a play is relayed to everyone in the call, each
 * tab playing it on its own speaker (the fake records each clip it plays). Turning soundboard
 * sounds off keeps another person's plays from playing here.
 */

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

/** Joins the Lounge's call, with the microphone already allowed. */
async function joinLounge(page: Page, phone: boolean): Promise<void> {
  await page.context().grantPermissions(["microphone", "camera"]);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${LOUNGE}`);
  await page.getByRole("main").waitFor();
  await page.locator(".room-header .huddle-launcher").click();

  const dock = page.locator(phone ? ".app-main-dock .huddle-dock" : ".sidebar .huddle-dock");

  await expect(dock.getByRole("status")).toContainText("Huddle active");

  // A phone keeps the conversation on joining; its call bar opens the call view.
  if (phone) await dock.getByRole("button", { name: "Show call" }).click();
}

/** Opens the soundboard: a dock control on a desktop, the call view's foot on a phone. */
async function openSoundboard(page: Page, phone: boolean, label = "Soundboard") {
  const button = phone
    ? page.locator(".call-view-actions").getByRole("button", { name: label })
    : page.locator(".sidebar .huddle-dock").getByRole("button", { name: label });

  await button.click();

  const panel = page.getByRole("dialog", { name: "Soundboard" });

  await expect(panel.getByRole("heading", { name: "Soundboard" })).toBeVisible();

  return panel;
}

/** The clips the fake transport played in this tab. */
function clips(page: Page) {
  return page.evaluate(() => window.__smartfireHuddle?.clips() ?? []);
}

matrix("an administrator's new sound plays to the call", async ({ page, phone }) => {
  await openApp(page, "admin/sounds");
  await expect(page.getByRole("heading", { level: 1, name: "Soundboard" })).toBeVisible();
  await expect(page.getByText("Sounds: 3 of 48 used.")).toBeVisible();

  await page.getByRole("textbox", { name: "Name" }).fill("Boing");
  await page.getByRole("textbox", { name: /Emoji/ }).fill("🎈");
  await page.getByLabel("Audio file").setInputFiles({
    name: "boing.wav",
    mimeType: "audio/wav",
    buffer: Buffer.from(tinyWav(330)),
  });
  await page.getByRole("button", { name: "Add sound" }).click();

  const sounds = page.getByRole("list", { name: "Sounds" });

  await expect(sounds.getByText("Boing", { exact: true })).toBeVisible();
  await expect(page.getByText("Sounds: 4 of 48 used.")).toBeVisible();

  if (phone) await expectNoHorizontalOverflow(page);

  await joinLounge(page, phone);

  const panel = await openSoundboard(page, phone);
  const boing = panel.getByRole("button", { name: "Boing" });

  await expect(boing).toContainText("🎈");
  await boing.click();

  // The server relays the play back to this tab too, which plays it on the call's speaker.
  await expect.poll(async () => (await clips(page)).length).toBe(1);

  const [clip] = await clips(page);

  expect(clip?.volume).toBe(1);
  expect(clip?.url).toMatch(/boing\.wav$/);

  if (phone) await expectNoHorizontalOverflow(page);
});

test("someone else's sound plays here unless soundboard sounds are off", async ({
  page,
  request,
}) => {
  await joinLounge(page, false);

  const panel = await openSoundboard(page, false);

  await expect(panel.getByRole("button", { name: "Airhorn" })).toBeEnabled();
  await panel.getByRole("switch", { name: /Play soundboard sounds/ }).click();
  await expect(panel.getByRole("status").filter({ hasText: "sounds are off" })).toBeVisible();
  await expect(panel.getByRole("button", { name: "Airhorn" })).toBeDisabled();

  // Maya plays the Airhorn while sounds are off here: nothing plays.
  await control(request, "soundboard-play", { roomId: LOUNGE, userId: USER_IDS.maya, soundId: 1 });

  await page.keyboard.press("Escape");
  await expect(
    page.locator(".sidebar .huddle-dock").getByRole("button", { name: "Soundboard (sounds off)" }),
  ).toBeVisible();

  const again = await openSoundboard(page, false, "Soundboard (sounds off)");

  await again.getByRole("switch", { name: /Play soundboard sounds/ }).click();
  await again.getByLabel("Soundboard volume").fill("50");

  // Jonah plays the Applause with sounds back on: it plays, at its 80% times this tab's 50%.
  await control(request, "soundboard-play", {
    roomId: LOUNGE,
    userId: USER_IDS.jonah,
    soundId: 2,
  });

  await expect.poll(async () => (await clips(page)).length).toBe(1);

  const [clip] = await clips(page);

  expect(clip?.url).toMatch(/applause\.wav$/);
  expect(clip?.volume).toBeCloseTo(0.4);
});

test.describe("on a touch phone", () => {
  test.use(PHONE_TOUCH);

  test("the soundboard is a sheet of finger-sized sounds", async ({ page }) => {
    await joinLounge(page, true);

    const sheet = await openSoundboard(page, true);

    await expect(sheet).toHaveClass(/action-sheet/);
    await expect(sheet.getByRole("button", { name: "Airhorn" })).toBeVisible();
    // The switch is a small thumb inside a full-width label row, which takes the tap.
    await expectTouchTargets(page, ".action-sheet .soundboard", { ignore: ".toggle" });
    await expectNoHorizontalOverflow(page);
  });
});
