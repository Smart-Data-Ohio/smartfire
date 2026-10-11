import type { APIRequestContext, Page } from "@playwright/test";
import { JOINABLE_OPEN_ROOM } from "../../mock/seed.ts";
import type { WelcomeSettings } from "../../src/gen/WelcomeSettings.ts";
import {
  DESKTOP,
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  openApp,
  PHONE_TOUCH,
  ROOM_IDS,
  syncWelcomed,
  test,
} from "./support.ts";

/**
 * The welcome card and server guide (U1, U4): administrators write them in the workspace
 * settings; everyone sees the card on Home (the top of the list on phones) until they dismiss it,
 * and reads the guide at /app/guide.
 */

const WELCOME: WelcomeSettings = {
  message: "Say **hi** in #general.",
  rooms: [
    { roomId: ROOM_IDS.general, emoji: "👋", description: "Everyone's here" },
    { roomId: JOINABLE_OPEN_ROOM.id, emoji: null, description: "Stories by the fire" },
  ],
  guide: [
    { title: "Start here", emoji: "📌", roomId: ROOM_IDS.general, body: null },
    { title: "Office hours", emoji: null, roomId: null, body: "Nine to *five*, Monday to Friday." },
  ],
};

async function control(
  request: APIRequestContext,
  action: "viewer-role" | "welcome",
  data: WelcomeSettings | { readonly role: "member" | "administrator" },
): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.post(`/__mock/${action}`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data,
  });

  expect(response.ok()).toBe(true);
}

const card = (page: Page) => page.getByRole("region", { name: "Welcome to Smart Data" });

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

test.describe("the welcome card and server guide", () => {
  test.use({ viewport: DESKTOP });

  test("an administrator writes the welcome and the guide, and Home shows them", async ({
    page,
  }) => {
    await openApp(page, "admin/workspace");

    const welcome = page.getByRole("region", { name: "Welcome screen" });

    await welcome.getByRole("textbox", { name: "Welcome message" }).fill("Glad you're **here**.");
    await welcome.getByRole("button", { name: "Add a room" }).click();

    const row = welcome.getByRole("group", { name: "Suggested room 1" });

    // The new row's room picker takes focus.
    await expect(row.getByRole("combobox", { name: "Room" })).toBeFocused();
    await row.getByRole("combobox", { name: "Room" }).selectOption({ label: "#campfire" });
    await row.getByRole("textbox", { name: "Emoji" }).fill("🔥");
    await row.getByRole("textbox", { name: "Description" }).fill("Stories by the fire");
    await welcome.getByRole("button", { name: "Save welcome" }).click();
    await expect(page.getByText("Welcome saved")).toBeVisible();

    const guide = page.getByRole("region", { name: "Server guide" });

    await guide.getByRole("button", { name: "Add an entry" }).click();

    const entry = guide.getByRole("group", { name: "Guide entry 1" });

    await entry.getByRole("textbox", { name: "Title" }).fill("Start here");
    await entry.getByRole("radio", { name: "A room link" }).check();
    await entry.getByRole("combobox", { name: "Room" }).selectOption({ label: "#general" });
    await guide.getByRole("button", { name: "Save guide" }).click();
    await expect(page.getByText("Server guide saved")).toBeVisible();

    // The save reached this client as workspace.updated: the guide link is in the sidebar now.
    const guideLink = sidebar(page).getByRole("link", { name: "Server guide" });

    await expect(guideLink).toBeVisible();

    await page.goto("/app/");
    await expect(card(page)).toBeVisible();
    await expect(card(page).getByText("here", { exact: true })).toHaveJSProperty(
      "tagName",
      "STRONG",
    );
    await expect(card(page).getByRole("button", { name: "Join #campfire" })).toBeVisible();
    await expect(card(page).getByRole("link", { name: "Edit welcome" })).toHaveAttribute(
      "href",
      "/app/admin/workspace#welcome",
    );
  });

  test("a member sees the welcome on Home and joins a suggested room", async ({ page }) => {
    await control(page.request, "welcome", WELCOME);
    await control(page.request, "viewer-role", { role: "member" });
    await openApp(page, "/");

    await expect(card(page).getByRole("heading", { level: 2 })).toHaveText("Welcome to Smart Data");
    await expect(card(page).getByText("Everyone's here")).toBeVisible();
    await expect(card(page).getByRole("link", { name: "Open #general" })).toHaveAttribute(
      "href",
      `/app/r/${ROOM_IDS.general}`,
    );
    // Members get no edit link.
    await expect(card(page).getByRole("link", { name: "Edit welcome" })).toHaveCount(0);

    await card(page).getByRole("button", { name: "Join #campfire" }).click();
    await expect(page).toHaveURL(`/app/r/${JOINABLE_OPEN_ROOM.id}`);
    await expect(
      sidebar(page).locator(".sidebar-row-name", { hasText: /^campfire$/ }),
    ).toBeVisible();

    // Back on Home, the room it suggested is one to open now.
    await page.goto("/app/");
    await expect(card(page).getByRole("link", { name: "Open #campfire" })).toBeVisible();
  });

  test("the guide lists its entries in order; administrators get an Edit link", async ({
    page,
  }) => {
    await control(page.request, "welcome", WELCOME);
    await openApp(page, "r/1");
    await sidebar(page).getByRole("link", { name: "Server guide" }).click();

    await expect(page).toHaveURL("/app/guide");
    await expect(page.getByRole("heading", { level: 1, name: "Server guide" })).toBeVisible();
    await expect(page.locator(".guide-entry-title")).toHaveText(["📌Start here", "Office hours"]);
    await expect(page.getByText("five", { exact: true })).toHaveJSProperty("tagName", "EM");
    await expect(page.getByRole("link", { name: "Open #general" })).toBeVisible();
    await expect(page).toHaveTitle(/Server guide/);

    await page.getByRole("link", { name: "Edit the server guide" }).click();
    await expect(page).toHaveURL("/app/admin/workspace#guide");
    await expect(page.getByRole("region", { name: "Server guide" })).toBeInViewport();

    await control(page.request, "viewer-role", { role: "member" });
    await page.goto("/app/guide");
    await expect(page.getByRole("heading", { level: 1, name: "Server guide" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Edit the server guide" })).toHaveCount(0);
  });

  test("no guide and no welcome leave Home and the sidebar as they were", async ({ page }) => {
    await openApp(page, "/");
    await expect(page).toHaveURL(`/app/r/${ROOM_IDS.general}`);
    await expect(sidebar(page).getByRole("link", { name: "Server guide" })).toHaveCount(0);
    await expect(card(page)).toHaveCount(0);
  });

  test("dismissing puts the card away for good; the menu and the guide bring it back", async ({
    page,
  }) => {
    await control(page.request, "welcome", WELCOME);
    await openApp(page, "/");
    await card(page).getByRole("button", { name: "Dismiss welcome" }).click();

    // Home goes on to the last conversation, and stays there after a reload.
    await expect(page).toHaveURL(`/app/r/${ROOM_IDS.general}`);
    await page.goto("/app/");
    await expect(page).toHaveURL(`/app/r/${ROOM_IDS.general}`);

    await page.locator(".sidebar-workspace").click();
    await page.getByRole("menuitem", { name: "Show welcome" }).click();
    await expect(page).toHaveURL("/app/");
    await expect(card(page)).toBeVisible();

    await card(page).getByRole("button", { name: "Dismiss welcome" }).click();
    await page.goto("/app/guide");
    await page.getByRole("button", { name: "Show the welcome on Home again" }).click();
    await expect(page.getByRole("button", { name: "Show the welcome on Home again" })).toHaveCount(
      0,
    );
    await page.goto("/app/");
    await expect(card(page)).toBeVisible();
  });

  test("an administrator's change reaches an open Home live", async ({ page }) => {
    await control(page.request, "welcome", WELCOME);

    const welcomed = syncWelcomed(page);

    await openApp(page, "/");
    await welcomed;
    await expect(card(page).getByText("Everyone's here")).toBeVisible();

    await control(page.request, "welcome", {
      ...WELCOME,
      message: "New *rules* today.",
      rooms: [{ roomId: ROOM_IDS.design, emoji: null, description: "Pixels" }],
    });

    await expect(card(page).getByText("Pixels")).toBeVisible();
    await expect(card(page).getByText("Everyone's here")).toHaveCount(0);
    await expect(card(page).getByText("rules", { exact: true })).toHaveJSProperty("tagName", "EM");
  });
});

test.describe("on a phone", () => {
  test.use(PHONE_TOUCH);

  test("the card tops the conversation list, fits, and the guide is a full screen", async ({
    page,
  }) => {
    await control(page.request, "welcome", WELCOME);
    await control(page.request, "viewer-role", { role: "member" });
    await openApp(page, "/");

    const list = sidebar(page);

    await expect(list.getByRole("region", { name: "Welcome to Smart Data" })).toBeVisible();
    await expectNoHorizontalOverflow(page);
    await expectTouchTargets(page, ".welcome-card");

    await list.getByRole("link", { name: "Server guide" }).click();
    await expect(page).toHaveURL("/app/guide");
    await expect(page.getByRole("heading", { level: 1, name: "Server guide" })).toBeVisible();
    await expect(list).toBeHidden();
    await expectNoHorizontalOverflow(page);
    await page.getByRole("link", { name: "Back to conversations" }).click();
    await expect(list).toBeVisible();

    await list.getByRole("button", { name: "Dismiss welcome" }).click();
    await expect(list.getByRole("region", { name: "Welcome to Smart Data" })).toHaveCount(0);
    // Focus moves on to what followed the card, not lost to the page.
    await expect(list.getByRole("link", { name: "Server guide" })).toBeFocused();

    // From the DMs tab, Show welcome goes back to the Home list, where the card is.
    await page.getByRole("button", { name: "DMs" }).click();
    await expect(page.locator(".sidebar-workspace")).toContainText("Direct messages");
    await page.locator(".sidebar-workspace").click();
    await page.getByRole("menuitem", { name: "Show welcome" }).click();
    await expect(list.getByRole("region", { name: "Welcome to Smart Data" })).toBeVisible();
  });
});
