import type { Page } from "@playwright/test";
import {
  DESKTOP,
  expect,
  matrix,
  openApp,
  PHONE,
  ROOM_IDS,
  shot,
  type Theme,
  test,
} from "./support.ts";

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

const rowName = (page: Page, name: string) =>
  page.locator(".sidebar-row-name", { hasText: new RegExp(`^${name}$`) });

const sidebarRow = (page: Page, name: string) => sidebar(page).locator(rowName(page, name));

/** A conversation's row, by the name it shows. */
const rowFor = (page: Page, name: string) =>
  sidebar(page).locator(".sidebar-row", { has: rowName(page, name) });

/** Takes the shot once every finite animation (the dialog's entrance, a step's slide) is done. */
declare global {
  interface Window {
    /** History calls recorded by `watchHistory`. */
    smartfireHistoryCalls?: string[];
  }
}

/** Holds room write requests of `method` until the returned release is called. */
async function holdRooms(page: Page, method: "PATCH" | "DELETE"): Promise<() => void> {
  let release: () => void = () => undefined;

  const held = new Promise<void>((resolve) => {
    release = resolve;
  });

  await page.route("**/api/v1/rooms/*", async (route) => {
    if (route.request().method() !== method) {
      return route.continue();
    }

    await held;

    return route.continue();
  });

  return release;
}

/**
 * Records the history calls the app makes from now on (back, forward, go, pushState,
 * replaceState). A completion that navigates has made its call by the time it settles, even
 * when the traversal it asked for hasn't landed yet, so an empty list proves it stayed put.
 */
async function watchHistory(page: Page): Promise<() => Promise<readonly string[]>> {
  await page.evaluate(() => {
    const calls: string[] = [];
    const history = window.history;
    const back = history.back.bind(history);
    const forward = history.forward.bind(history);
    const go = history.go.bind(history);
    const pushState = history.pushState.bind(history);
    const replaceState = history.replaceState.bind(history);

    window.smartfireHistoryCalls = calls;

    history.back = () => {
      calls.push("back");
      back();
    };

    history.forward = () => {
      calls.push("forward");
      forward();
    };

    history.go = (delta) => {
      calls.push("go");
      go(delta);
    };

    history.pushState = (data, unused, url) => {
      calls.push("pushState");
      pushState(data, unused, url);
    };

    history.replaceState = (data, unused, url) => {
      calls.push("replaceState");
      replaceState(data, unused, url);
    };
  });

  return () => page.evaluate(() => [...(window.smartfireHistoryCalls ?? [])]);
}

/** Lets a completion's navigation (which commits asynchronously) reach history. */
async function settleHistory(page: Page): Promise<void> {
  await page.evaluate(
    () =>
      new Promise<void>((resolve) => {
        requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(resolve, 250)));
      }),
  );
}

async function settledShot(page: Page, name: string, theme: Theme) {
  await page.waitForFunction(() =>
    document
      .getAnimations()
      .every(
        (animation) =>
          animation.playState !== "running" ||
          animation.effect?.getTiming().iterations === Number.POSITIVE_INFINITY,
      ),
  );
  await shot(page, name, theme);
}

test.describe("creating rooms", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize(DESKTOP);
  });

  test("the channels section's + makes a private channel with people and opens it", async ({
    page,
  }) => {
    await openApp(page, "/");
    await sidebar(page).getByRole("button", { name: "Create a channel", exact: true }).click();

    const dialog = page.getByRole("dialog", { name: "Create a channel" });

    await expect(dialog.getByRole("radio", { name: /Text channel/ })).toBeChecked();
    await expect(dialog.getByLabel("Name", { exact: true })).toBeFocused();
    await dialog.getByLabel("Name", { exact: true }).fill("design-crit");

    // An icon from the shared emoji picker.
    await dialog.getByRole("button", { name: "Choose an icon" }).click();
    await page.getByRole("combobox", { name: "Search emoji" }).fill("rocket");
    await page.keyboard.press("Enter");
    await expect(dialog.getByRole("button", { name: "Change icon (:rocket:)" })).toBeVisible();

    await dialog.getByRole("switch", { name: /Private channel/ }).click();
    await dialog.getByRole("button", { name: "Next" }).click();

    const people = page.getByRole("dialog", { name: "Add people" });

    await expect(people.getByRole("combobox")).toBeFocused();
    await people.getByRole("combobox").fill("Maya");
    await people.getByRole("option", { name: /Maya/ }).click();
    await people.getByRole("button", { name: "Create channel" }).click();

    await expect(people).toBeHidden();
    await expect(page).toHaveURL(/\/app\/r\/\d+$/);
    await expect(page.getByRole("heading", { level: 1, name: "design-crit" })).toBeVisible();
    await expect(sidebarRow(page, "design-crit")).toBeVisible();
    await expect(rowFor(page, "design-crit").locator(".room-glyph-emoji")).toHaveText("🚀");
  });

  test("a public channel is made in one step, and Back keeps what was typed", async ({ page }) => {
    await openApp(page, "/");
    await page.locator(".sidebar-workspace").first().click();
    await page.getByRole("menuitem", { name: "Create a channel…" }).click();

    // The title follows the kind, so this finds the dialog by role alone.
    const dialog = page.getByRole("dialog");

    await expect(dialog).toHaveAccessibleName("Create a channel");
    await dialog.getByRole("radio", { name: /Voice channel/ }).check();
    await expect(dialog).toHaveAccessibleName("Create a voice channel");
    await dialog.getByLabel("Name", { exact: true }).fill("Standup");
    await dialog.getByRole("button", { name: "Next" }).click();
    await page.getByRole("button", { name: "Back" }).click();
    await expect(dialog.getByLabel("Name", { exact: true })).toHaveValue("Standup");

    await dialog.getByRole("radio", { name: /Text channel/ }).check();
    await dialog.getByLabel("Name", { exact: true }).fill("watercooler");
    await dialog.getByLabel("Name", { exact: true }).press("Enter");

    await expect(dialog).toBeHidden();
    await expect(page.getByRole("heading", { level: 1, name: "watercooler" })).toBeVisible();
  });

  test("nothing is created until the form loads; a failed load can be retried", async ({
    page,
  }) => {
    // Every read fails until the test lets them through (development mode may read twice).
    let failing = true;

    await page.route("**/api/v1/rooms/new?*", async (route) => {
      if (failing) {
        await route.fulfill({
          status: 500,
          contentType: "application/json",
          body: JSON.stringify({ error: { code: "server_error", message: "Try again later" } }),
        });
      } else {
        await route.continue();
      }
    });
    await openApp(page, "rooms/new/open");

    const dialog = page.getByRole("dialog", { name: "Create a channel" });
    const create = dialog.getByRole("button", { name: "Create channel" });

    await expect(dialog.getByRole("alert")).toContainText("Couldn't load the form");
    await expect(create).toBeDisabled();
    await dialog.getByLabel("Name", { exact: true }).press("Enter");
    await expect(dialog).toBeVisible();

    failing = false;
    await dialog.getByRole("button", { name: "Try again" }).click();
    await expect(create).toBeEnabled();
    await expect(dialog.getByRole("alert")).toHaveCount(0);
    await expect(dialog.getByLabel("Name", { exact: true })).not.toHaveAttribute("placeholder", "");
  });

  test("trying again after a lost reply opens the room it made instead of making another", async ({
    page,
  }) => {
    const keys: string[] = [];
    let lost = 1;

    await page.route("**/api/v1/rooms", async (route) => {
      if (route.request().method() !== "POST") {
        return route.continue();
      }

      keys.push(route.request().postDataJSON().clientRoomId);

      if (lost > 0) {
        lost -= 1;
        // The server makes the room; its reply never arrives.
        await route.fetch();

        return route.abort("connectionreset");
      }

      return route.continue();
    });
    await openApp(page, "rooms/new/open");

    const dialog = page.getByRole("dialog", { name: "Create a channel" });
    const name = dialog.getByLabel("Name", { exact: true });
    const create = dialog.getByRole("button", { name: "Create channel" });

    await expect(create).toBeEnabled();
    await name.fill("only-once");
    await create.click();
    await expect(dialog).toBeVisible();
    await expect(create).toBeEnabled();

    await create.click();
    await expect(dialog).toBeHidden();
    await expect(page.getByRole("heading", { level: 1, name: "only-once" })).toBeVisible();
    expect(keys).toHaveLength(2);
    expect(keys[1]).toBe(keys[0]);
    await expect(page.locator(".sidebar").getByText("only-once", { exact: true })).toHaveCount(1);
  });

  test("a retry that asks for something else is a new attempt with its own key", async ({
    page,
  }) => {
    const keys: string[] = [];
    let lost = 1;

    await page.route("**/api/v1/rooms", async (route) => {
      if (route.request().method() !== "POST") {
        return route.continue();
      }

      keys.push(route.request().postDataJSON().clientRoomId);

      if (lost > 0) {
        lost -= 1;

        return route.abort("connectionreset");
      }

      return route.continue();
    });
    await openApp(page, "rooms/new/open");

    const dialog = page.getByRole("dialog", { name: "Create a channel" });
    const name = dialog.getByLabel("Name", { exact: true });
    const create = dialog.getByRole("button", { name: "Create channel" });

    await expect(create).toBeEnabled();
    await name.fill("first-try");
    await create.click();
    await expect(create).toBeEnabled();

    await name.fill("second-try");
    await create.click();
    await expect(dialog).toBeHidden();
    await expect(page.getByRole("heading", { level: 1, name: "second-try" })).toBeVisible();
    expect(keys).toHaveLength(2);
    expect(keys[1]).not.toBe(keys[0]);
  });

  test("the classic new page and the app shortcut open the dialog on their kind", async ({
    page,
  }) => {
    await openApp(page, "rooms/new/voice");

    const dialog = page.getByRole("dialog", { name: "Create a voice channel" });

    await expect(dialog.getByRole("radio", { name: /Voice channel/ })).toBeChecked();
    // The dialog sits over the home screen; the new-room URL doesn't stay in history.
    await expect(page).not.toHaveURL(/rooms\/new/);
    await dialog.getByRole("button", { name: "Cancel" }).click();
    await expect(dialog).toBeHidden();
  });
});

test.describe("room settings", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize(DESKTOP);
  });

  test("the header's name opens the settings; a rename saves in place", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.launchPlanning}`);
    await page.getByRole("link", { name: /room settings$/ }).click();

    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.launchPlanning}/settings$`));
    await expect(dialog.getByRole("button", { name: "Save changes" })).toBeDisabled();
    await dialog.getByLabel("Name", { exact: true }).fill("launch-war-room");
    await expect(dialog.getByText("Unsaved changes")).toBeVisible();
    await dialog.getByRole("button", { name: "Save changes" }).click();

    await expect(dialog).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.launchPlanning}$`));
    await expect(page.getByRole("heading", { level: 1, name: "launch-war-room" })).toBeVisible();
    await expect(sidebarRow(page, "launch-war-room")).toBeVisible();
  });

  test("members are added and removed in the draft, then saved together", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.launchPlanning}/settings`);

    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    await dialog.getByRole("tab", { name: /Members/ }).click();

    const before = await dialog.getByRole("tab", { name: /Members/ }).textContent();

    await dialog.getByRole("combobox", { name: "Add people" }).fill("Grace");
    await dialog.getByRole("option", { name: /Grace/ }).click();
    await expect(dialog.getByRole("listitem").filter({ hasText: "Grace" })).toContainText("New");
    await dialog.getByRole("button", { name: "Remove Grace" }).click();
    await expect(dialog.getByRole("tab", { name: /Members/ })).toHaveText(before ?? "");
    await expect(dialog.getByRole("button", { name: "Save changes" })).toBeDisabled();

    await dialog.getByRole("combobox", { name: "Add people" }).fill("Grace");
    await page.keyboard.press("Enter");
    await dialog.getByRole("button", { name: "Save changes" }).click();
    await expect(dialog).toBeHidden();

    await page.goto(`/app/r/${ROOM_IDS.launchPlanning}/settings`);
    await page.getByRole("tab", { name: /Members/ }).click();
    await expect(
      page.getByRole("dialog", { name: "Channel settings" }).getByRole("listitem").filter({
        hasText: "Grace",
      }),
    ).toBeVisible();
  });

  test("a channel can go private, and Delete asks first", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.quiet}`);
    await rowFor(page, "quiet").click({ button: "right" });
    await page.getByRole("menuitem", { name: "Channel settings" }).click();

    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    await dialog.getByRole("switch", { name: /Private channel/ }).click();
    await expect(dialog.getByRole("tab", { name: /Members/ })).toBeVisible();
    await dialog.getByRole("switch", { name: /Private channel/ }).click();
    await expect(dialog.getByRole("tab", { name: /Members/ })).toBeHidden();

    await dialog.getByRole("button", { name: "Delete…" }).click();

    const confirm = page.getByRole("alertdialog", { name: "Delete #quiet?" });

    await confirm.getByRole("button", { name: "Keep it" }).click();
    await expect(confirm).toBeHidden();
    await dialog.getByRole("button", { name: "Delete…" }).click();
    await confirm.getByRole("button", { name: "Delete channel" }).click();

    await expect(dialog).toBeHidden();
    await expect(page).not.toHaveURL(new RegExp(`/r/${ROOM_IDS.quiet}`));
    await expect(sidebarRow(page, "quiet")).toBeHidden();
  });

  test("settings take focus at the name once loaded; going private keeps focus on the switch", async ({
    page,
  }) => {
    await openApp(page, `r/${ROOM_IDS.quiet}`);
    // A slow form: the dialog's first focus lands before the fields exist.
    await page.route("**/api/v1/rooms/*/edit", async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 400));
      await route.continue();
    });
    await page.getByRole("link", { name: /room settings$/ }).focus();
    await page.keyboard.press("Enter");

    const dialog = page.getByRole("dialog", { name: "Channel settings" });
    const toggle = dialog.getByRole("switch", { name: /Private channel/ });

    await expect(dialog.getByLabel("Name", { exact: true })).toBeFocused();

    await toggle.focus();
    await page.keyboard.press("Space");
    await expect(dialog.getByRole("tab", { name: /Members/ })).toBeVisible();
    await expect(toggle).toBeFocused();
    await page.keyboard.press("Space");
    await expect(dialog.getByRole("tab", { name: /Members/ })).toBeHidden();
    await expect(toggle).toBeFocused();
  });

  test("closing settings steps back, so Back doesn't reopen them", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.design}`);
    await rowFor(page, "general").click();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}$`));
    await page.getByRole("link", { name: /room settings$/ }).click();

    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    await expect(dialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}$`));

    await page.goBack();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.design}$`));
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });

  test("a save that lands after settings were closed doesn't close them again", async ({
    page,
  }) => {
    await openApp(page, `r/${ROOM_IDS.design}`);
    await rowFor(page, "general").click();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));
    await page.getByRole("link", { name: /room settings$/ }).click();

    const dialog = page.getByRole("dialog", { name: "Channel settings" });
    let release: () => void = () => undefined;

    const held = new Promise<void>((resolve) => {
      release = resolve;
    });

    await page.route("**/api/v1/rooms/*", async (route) => {
      if (route.request().method() !== "PATCH") {
        return route.continue();
      }

      await held;

      return route.continue();
    });
    await dialog.getByLabel("Name", { exact: true }).fill("general-renamed");
    await dialog.getByRole("button", { name: "Save changes" }).click();
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));

    release();
    await expect(page.getByText("Changes saved")).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));
  });

  test("a save that lands after Back left the settings doesn't step back again", async ({
    page,
  }) => {
    await openApp(page, `r/${ROOM_IDS.design}`);
    await rowFor(page, "general").click();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));
    await rowFor(page, "quiet").click({ button: "right" });
    await page.getByRole("menuitem", { name: "Channel settings" }).click();

    const dialog = page.getByRole("dialog", { name: "Channel settings" });
    const release = await holdRooms(page, "PATCH");

    await dialog.getByLabel("Name", { exact: true }).fill("quiet-renamed");
    await dialog.getByRole("button", { name: "Save changes" }).click();
    await page.goBack();
    await expect(dialog).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));

    const historyCalls = await watchHistory(page);

    release();
    await expect(page.getByText("Changes saved")).toBeVisible();
    await settleHistory(page);
    expect(await historyCalls()).toEqual([]);
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));
  });

  test("a delete that lands after Back left the settings stays where Back went", async ({
    page,
  }) => {
    await openApp(page, `r/${ROOM_IDS.design}`);
    await rowFor(page, "general").click();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));
    await rowFor(page, "quiet").click({ button: "right" });
    await page.getByRole("menuitem", { name: "Channel settings" }).click();

    const dialog = page.getByRole("dialog", { name: "Channel settings" });
    const release = await holdRooms(page, "DELETE");

    await dialog.getByRole("button", { name: "Delete…" }).click();
    await page
      .getByRole("alertdialog", { name: "Delete #quiet?" })
      .getByRole("button", { name: "Delete channel" })
      .click();
    await page.goBack();
    await expect(dialog).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));

    const historyCalls = await watchHistory(page);

    release();
    await expect(page.getByText(/Deleted .*quiet/)).toBeVisible();
    await settleHistory(page);
    expect(await historyCalls()).toEqual([]);
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));
    await expect(rowFor(page, "quiet")).toHaveCount(0);
  });

  test("leaving a room by a save that lands after Back stays where Back went", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.design}`);
    await rowFor(page, "general").click();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));
    await rowFor(page, "launch-planning").click({ button: "right" });
    await page.getByRole("menuitem", { name: "Channel settings" }).click();

    const dialog = page.getByRole("dialog", { name: "Channel settings" });
    const release = await holdRooms(page, "PATCH");

    await dialog.getByRole("tab", { name: /Members/ }).click();
    await dialog.getByRole("button", { name: "Leave (remove yourself)" }).click();
    await dialog.getByRole("button", { name: "Save changes" }).click();
    await page.goBack();
    await expect(dialog).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));

    const historyCalls = await watchHistory(page);

    release();
    await expect(page.getByText(/You left .*launch-planning/)).toBeVisible();
    await settleHistory(page);
    expect(await historyCalls()).toEqual([]);
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));
    await expect(rowFor(page, "launch-planning")).toHaveCount(0);
  });

  test("settings opened as the first page close onto the room in place", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.design}`);
    await page.goto(`/app/r/${ROOM_IDS.quiet}/settings`);

    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    await dialog.getByRole("button", { name: "Cancel" }).click();
    await expect(dialog).toBeHidden();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.quiet}$`));

    await page.goBack();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.design}$`));
  });

  test("subscribes and unsubscribes a repository, and shows the room's email address", async ({
    page,
  }) => {
    await openApp(page, `r/${ROOM_IDS.launchPlanning}/settings`);

    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    await dialog.getByRole("tab", { name: "GitHub" }).click();
    await expect(dialog.getByText("No repositories subscribed yet.")).toBeVisible();
    await dialog.getByRole("textbox", { name: "Repository" }).fill("Rails/Rails");
    await dialog.getByRole("button", { name: "Subscribe" }).click();
    await expect(dialog.getByText("rails/rails")).toBeVisible();
    await expect(page.getByText("Subscribed to rails/rails.")).toBeVisible();

    await dialog.getByRole("button", { name: "Remove rails/rails" }).click();
    await page
      .getByRole("alertdialog", { name: "Unsubscribe rails/rails?" })
      .getByRole("button", { name: "Remove", exact: true })
      .click();
    await expect(dialog.getByText("No repositories subscribed yet.")).toBeVisible();
    await expect(page.getByText("Unsubscribed from rails/rails.")).toBeVisible();

    await dialog.getByRole("tab", { name: "Email" }).click();
    await expect(
      dialog.getByText("room-a1b2c3d4e5f67890a1b2c3d4e5f67890@mail.campfire.test"),
    ).toBeVisible();
  });

  test("on a phone, Enter in the repository field subscribes without saving the room", async ({
    page,
  }) => {
    await page.setViewportSize(PHONE);
    await openApp(page, `r/${ROOM_IDS.launchPlanning}/settings`);

    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    await dialog.getByRole("tab", { name: "GitHub" }).click();
    await dialog.getByRole("textbox", { name: "Repository" }).fill("campfire/campfire");
    await page.keyboard.press("Enter");
    await expect(dialog.getByText("campfire/campfire")).toBeVisible();
    await expect(dialog).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.launchPlanning}/settings$`));

    await dialog.getByRole("tab", { name: "Email" }).click();
    await expect(dialog.getByText(/room-a1b2c3d4e5f67890/)).toBeVisible();
    await expect(dialog.getByRole("button", { name: "Rotate address" })).toBeVisible();
  });

  test("a board's settings offer GitHub and not inbound email", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.launchPlanning}/settings`);
    // The seeded board is not a sidebar row this spec opens by id.
    await page.goto("/app/r/900/settings");

    const dialog = page.getByRole("dialog", { name: "Board settings" });

    await expect(dialog.getByRole("tab", { name: "GitHub" })).toBeVisible();
    await expect(dialog.getByRole("tab", { name: "Email" })).toHaveCount(0);
    await dialog.getByRole("tab", { name: "GitHub" }).click();
    await expect(dialog.getByText("No repositories subscribed yet.")).toBeVisible();
  });

  test("a direct message's settings URL opens the conversation", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.dmMaya}/settings`);
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.dmMaya}$`));
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });
});

matrix("the create dialog and the settings", async ({ page, theme }) => {
  await openApp(page, "rooms/new/closed", theme);

  const dialog = page.getByRole("dialog", { name: "Create a channel" });

  await dialog.getByLabel("Name", { exact: true }).fill("design-crit");
  await settledShot(page, "rooms-create", theme);
  await dialog.getByRole("button", { name: "Next" }).click();
  await page.getByRole("dialog", { name: "Add people" }).getByRole("combobox").fill("a");
  await settledShot(page, "rooms-create-people", theme);
  await page.keyboard.press("Escape");

  await page.goto(`/app/r/${ROOM_IDS.lounge}/settings`);

  const settings = page.getByRole("dialog", { name: "Voice channel settings" });

  await expect(settings.getByLabel("Name", { exact: true })).toHaveValue("Lounge");
  await settledShot(page, "rooms-settings", theme);
  await settings.getByRole("tab", { name: /Members/ }).click();
  await expect(settings.getByRole("listitem").first()).toBeVisible();
  await settledShot(page, "rooms-settings-members", theme);
});
