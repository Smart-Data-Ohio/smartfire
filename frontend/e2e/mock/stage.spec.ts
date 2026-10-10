import type { APIRequestContext, Page } from "@playwright/test";
import {
  expect,
  matrix,
  openHeaderTool,
  ROOM_IDS,
  SHOTS,
  shot,
  type Theme,
  test,
  USER_IDS,
} from "./support.ts";

/**
 * Stages, incoming calls and call notices against the mock and the fake LiveKit transport: the
 * stage pane (roster, hands, roles, Go live), the ring for a huddle in a direct message, and the
 * banner, pill and toasts that say who joined and left.
 */

const TOWN_HALL = ROOM_IDS.townHall;

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

  expect(response.ok(), await response.text()).toBe(true);
}

/** Opens a room with the microphone allowed, once sync has said hello (controls publish live). */
async function open(page: Page, roomId: number, theme: Theme = "light"): Promise<void> {
  await page.context().grantPermissions(["microphone", "camera"]);
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });

  const welcomed = page
    .waitForEvent("websocket", (socket) => socket.url().includes("/api/v1/sync"))
    .then((socket) =>
      socket.waitForEvent("framereceived", (frame) =>
        String(frame.payload).includes('"t":"welcome"'),
      ),
    );

  await page.goto(`/app/r/${roomId}`);
  await page.getByRole("main").waitFor();
  await welcomed;
}

function stagePane(page: Page) {
  return page.locator(".right-pane");
}

async function openStage(page: Page): Promise<void> {
  await openHeaderTool(page, /^Stage/);
  await expect(stagePane(page).getByRole("region", { name: "Hosts" })).toBeVisible();
}

function dock(page: Page, phone = false) {
  return page.locator(phone ? ".app-main-dock .huddle-dock" : ".sidebar .huddle-dock");
}

// Screenshots only: stage.test.ts covers the roster, and the next test the pane's groups live.
if (SHOTS) {
  matrix("the stage pane lists hosts, speakers and listeners", async ({ page, theme }) => {
    await open(page, TOWN_HALL, theme);
    await control(page.request, "stage-hand", {
      roomId: TOWN_HALL,
      userId: USER_IDS.jonah,
      raised: true,
    });
    await openStage(page);

    const pane = stagePane(page);

    await expect(pane.getByRole("region", { name: "Hosts" })).toContainText("Riel St. Amand");
    await expect(pane.getByRole("region", { name: "Speakers" })).toContainText("Priya Raman");
    await expect(pane.getByRole("region", { name: "Listeners" })).toContainText(
      "Hand raised · #1 in queue",
    );
    await expect(pane.getByText("You are hosting this stage.")).toBeVisible();
    await page.mouse.move(0, 0);
    await shot(page, "stage-pane", theme);
  });
}

test("a host invites a raised hand to speak and moves people around", async ({ page, request }) => {
  await open(page, TOWN_HALL);
  await openStage(page);
  await control(request, "stage-hand", { roomId: TOWN_HALL, userId: USER_IDS.jonah, raised: true });

  const pane = stagePane(page);
  const listeners = pane.getByRole("region", { name: "Listeners" });
  const speakers = pane.getByRole("region", { name: "Speakers" });

  await expect(pane.getByText("You are hosting this stage.")).toBeVisible();
  await expect(listeners.getByText("Hand raised · #1 in queue")).toBeVisible();
  await expect(page.getByRole("button", { name: "Stage (1 raised hands)" })).toBeVisible();
  await listeners.getByRole("button", { name: "Invite to speak" }).click();
  await expect(speakers).toContainText("Jonah Lindqvist");

  await speakers.getByRole("button", { name: "Actions for Priya Raman" }).click();
  await page.getByRole("menuitem", { name: "Make host" }).click();
  await expect(pane.getByRole("region", { name: "Hosts" })).toContainText("Priya Raman");

  // The last host can't step down; with two hosts, each may.
  await pane
    .getByRole("region", { name: "Hosts" })
    .getByRole("button", { name: "Actions for Riel St. Amand" })
    .click();
  await expect(page.getByRole("menuitem", { name: /Move to audience/ })).not.toHaveAttribute(
    "aria-disabled",
  );
});

test("a listener raises and lowers a hand", async ({ page, request }) => {
  await open(page, TOWN_HALL);
  await openStage(page);
  // The stage keeps a host: Priya takes over before the viewer steps down.
  await control(request, "stage-role", {
    roomId: TOWN_HALL,
    userId: USER_IDS.priya,
    role: "host",
  });
  await control(request, "stage-role", {
    roomId: TOWN_HALL,
    userId: USER_IDS.riel,
    role: "listener",
  });

  const pane = stagePane(page);

  await expect(pane.getByText("You are in the audience.")).toBeVisible();
  await pane.getByRole("button", { name: "Raise hand" }).click();
  await expect(
    pane.getByText("Your hand is raised. A host can invite you to speak."),
  ).toBeVisible();
  await pane.getByRole("button", { name: "Lower hand" }).click();
  await expect(pane.getByText("You are in the audience.")).toBeVisible();
});

test("a host goes live from the stage and stops the stream", async ({ page }) => {
  await open(page, TOWN_HALL);
  await openStage(page);

  const pane = stagePane(page);
  const goLive = pane.getByRole("button", { name: "Go live" });

  await expect(goLive).toBeDisabled();
  await page.locator(".room-header .huddle-launcher").click();
  await expect(dock(page).getByRole("status")).toContainText("Huddle active");
  await expect(pane.getByLabel("Stream quality")).toHaveValue("1080p15");
  await pane.getByLabel("Stream quality").selectOption("1080p60");
  await goLive.click();
  await expect(pane.locator(".stage-live")).toContainText("Riel St. Amand");
  await expect(page.locator(".call-share[data-stream]")).toHaveCount(1);

  await pane.getByRole("button", { name: "Stop stream" }).click();
  await expect(pane.locator(".stage-live")).toHaveCount(0);
  await expect(page.locator(".call-share")).toHaveCount(0);
});

test("an administrator who isn't a host can stop someone's stream", async ({ page, request }) => {
  await open(page, TOWN_HALL);
  await openStage(page);
  // Priya hosts and presents; the viewer (an administrator) sits in the audience.
  await control(request, "stage-role", { roomId: TOWN_HALL, userId: USER_IDS.priya, role: "host" });
  await control(request, "stage-role", {
    roomId: TOWN_HALL,
    userId: USER_IDS.riel,
    role: "listener",
  });
  await control(request, "huddle-join", { roomId: TOWN_HALL, userId: USER_IDS.priya });
  await control(request, "stage-live", { roomId: TOWN_HALL, userId: USER_IDS.priya });

  const pane = stagePane(page);

  await expect(pane.getByText("You are in the audience.")).toBeVisible();
  await expect(pane.locator(".stage-live")).toContainText("Priya Raman");
  await pane.getByRole("button", { name: "Stop stream" }).click();
  await expect(pane.locator(".stage-live")).toHaveCount(0);
});

matrix("an incoming huddle rings, and Join takes you there", async ({ page, theme, phone }) => {
  await open(page, ROOM_IDS.general, theme);
  await control(page.request, "huddle-join", { roomId: ROOM_IDS.dmMaya, userId: USER_IDS.maya });
  await control(page.request, "huddle-ring", { roomId: ROOM_IDS.dmMaya, userId: USER_IDS.maya });

  const ring = page.getByRole("alertdialog", { name: "Maya Okafor started a huddle" });

  await expect(ring).toBeVisible();
  await shot(page, "huddle-ring", theme);
  await ring.getByRole("button", { name: "Join" }).click();
  await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.dmMaya}$`));
  await expect(dock(page, phone).getByRole("status")).toContainText("Huddle active");
  await expect(ring).toHaveCount(0);
});

test("Smartfire's reduced-motion setting stills the ring", async ({ page, request }) => {
  await open(page, ROOM_IDS.general);
  // The OS allows motion; the user asked Smartfire for less.
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.evaluate(() => {
    document.documentElement.dataset.motion = "reduce";
  });
  await control(request, "huddle-ring", { roomId: ROOM_IDS.dmMaya, userId: USER_IDS.maya });

  const ring = page.getByRole("alertdialog", { name: "Maya Okafor started a huddle" });

  await expect(ring).toBeVisible();
  expect(await ring.evaluate((element) => getComputedStyle(element).animationName)).toBe("none");

  const pulse = await page
    .locator(".huddle-ring-icon")
    .evaluate((element) => getComputedStyle(element, "::after").animationName);

  expect(pulse).toBe("none");
});

test("a call elsewhere shows a banner and a sidebar pill with Join", async ({ page, request }) => {
  await open(page, LOUNGE);
  await control(request, "huddle-join", { roomId: LOUNGE, userId: USER_IDS.priya });

  const banner = page.locator(".huddle-join-banner");

  await expect(banner).toContainText("Priya Raman is in your huddle");
  await expect(page.locator(".sidebar .huddle-join-pill")).toContainText("Priya Raman");
  await shot(page, "huddle-join-banner", "light");

  await control(request, "huddle-leave", { roomId: LOUNGE, userId: USER_IDS.priya });
  await expect(banner).toHaveCount(0);

  await control(request, "huddle-join", { roomId: LOUNGE, userId: USER_IDS.priya });
  await banner.getByRole("button", { name: "Join" }).click();
  await expect(dock(page).getByRole("status")).toContainText("Huddle active");
  await expect(banner).toHaveCount(0);
});

// Screenshots only: notices.test.ts covers the batched join toast and the delayed leave toast.
if (SHOTS) {
  test("in a call, joins and leaves toast", async ({ page, request }) => {
    await open(page, LOUNGE);
    await page.locator(".room-header .huddle-launcher").click();
    await expect(dock(page).getByRole("status")).toContainText("Huddle active");

    await control(request, "huddle-join", { roomId: LOUNGE, userId: USER_IDS.priya });
    await control(request, "huddle-join", { roomId: LOUNGE, userId: USER_IDS.sam });
    await expect(page.locator(".huddle-toast")).toHaveText([
      "Priya Raman and Sam Whitfield joined",
    ]);
    await shot(page, "huddle-join-toast", "light");

    await page.clock.install();
    await control(request, "huddle-leave", { roomId: LOUNGE, userId: USER_IDS.sam });
    await page.clock.runFor(5_100);
    await expect(page.locator(".huddle-toast", { hasText: "Sam Whitfield left" })).toBeVisible();
  });
}
