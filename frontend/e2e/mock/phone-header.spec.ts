import type { APIRequestContext, Page } from "@playwright/test";
import {
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  openApp,
  PHONE,
  PHONE_SMALL,
  PHONE_TOUCH,
  ROOM_IDS,
  shot,
  syncWelcomed,
  test,
  USER_IDS,
} from "./support.ts";

/**
 * The phone page header: back, the title, the call and a ⋯ menu for the rest. The room's name
 * keeps its width on a 360 px screen, every tool the wide header shows is in the menu and opens
 * what its button did, the name opens the room's details, and the event pages lead back from
 * the header.
 */

const UPCOMING = 8001;

interface RoomCase {
  readonly label: string;
  readonly id: number;
  readonly name: string;
  /** What the ⋯ menu lists, in order (the notification levels are a submenu). */
  readonly items: readonly RegExp[];
}

const PANES = [/^Members \(\d+\)$/, /^Threads$/, /^Pinned messages/, /^Files$/];

const REST = [/^Notifications$/, /^Events$/, /^Search$/];

const ROOMS: readonly RoomCase[] = [
  { label: "general", id: ROOM_IDS.general, name: "general", items: [...PANES, ...REST] },
  {
    label: "engineering",
    id: ROOM_IDS.engineering,
    name: "engineering",
    items: [...PANES, ...REST],
  },
  {
    label: "a DM",
    id: ROOM_IDS.dmMaya,
    name: "Maya Okafor",
    items: [/^Add people$/, /^Pinned messages/, /^Files$/, ...REST],
  },
  { label: "lounge", id: ROOM_IDS.lounge, name: "Lounge", items: [...PANES, ...REST] },
  {
    label: "Town Hall",
    id: ROOM_IDS.townHall,
    name: "Town Hall",
    items: [/^Stage$/, ...PANES, ...REST],
  },
];

function header(page: Page) {
  return page.locator(".room-header");
}

function pane(page: Page) {
  return page.locator("aside.right-pane");
}

async function openRoom(page: Page, id: number, theme: "light" | "dark" = "light"): Promise<void> {
  await openApp(page, `r/${id}`, theme);
  await header(page).locator(".room-title-name").waitFor();
}

/** Calls one of the mock's controls with `data` (a stage's raised hand). */
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

/**
 * Opens the header's ⋯ menu. A menu ignores a click on its trigger just after it closed (the
 * press that light-dismissed it), so a test that comes straight back to it tries again.
 */
async function openOverflow(page: Page) {
  const menu = page.getByRole("menu", { name: "More" });

  await expect(async () => {
    await header(page).getByRole("button", { name: "More" }).click();
    await expect(menu).toBeVisible({ timeout: 1000 });
  }).toPass();

  return menu;
}

for (const viewport of [PHONE_SMALL, PHONE]) {
  test.describe(`on a ${viewport.width} px touch phone`, () => {
    test.use({ ...PHONE_TOUCH, viewport });

    for (const room of ROOMS) {
      test(`${room.label}: the name keeps its width; the rest is in the ⋯ menu`, async ({
        page,
      }) => {
        await openRoom(page, room.id);

        // The name, in full, with room to spare.
        const title = header(page).locator(".room-title-name");

        await expect(title).toHaveText(room.name);

        const fit = await title.evaluate((element) => ({
          width: element.getBoundingClientRect().width,
          clipped: element.scrollWidth > element.clientWidth,
        }));

        expect(fit.width).toBeGreaterThanOrEqual(120);
        expect(fit.clipped).toBe(false);

        // Back, the title, and at most the call and ⋯ beside it; all at the touch size.
        await expect(
          header(page).getByRole("link", { name: "Back to conversations" }),
        ).toBeVisible();
        expect(
          await header(page).locator(".page-header-actions button").count(),
        ).toBeLessThanOrEqual(2);
        await expectTouchTargets(page, ".room-header");
        await expectNoHorizontalOverflow(page);

        // The ⋯ menu holds every tool the wide header shows.
        const menu = await openOverflow(page);

        await expect(menu.getByRole("menuitem")).toHaveText(room.items);
        await page.keyboard.press("Escape");
        await expect(menu).toBeHidden();
      });

      test(`${room.label}: the name opens the room's details`, async ({ page }) => {
        await openRoom(page, room.id);
        await header(page).locator(".room-title-button").click();

        await expect(pane(page).getByRole("heading", { name: "Details" })).toBeVisible();
        await expect(pane(page).locator(".details-name")).toHaveText(room.name);
        await expect(pane(page).getByRole("button", { name: /^Notifications/ })).toBeVisible();
        await expect(pane(page).getByRole("button", { name: /^Pinned messages/ })).toBeVisible();
        await expect(pane(page).getByRole("button", { name: /^Files/ })).toBeVisible();
        await expectTouchTargets(page, "aside.right-pane");
        await expectNoHorizontalOverflow(page);

        await pane(page)
          .getByRole("button", { name: /^Back to/ })
          .click();
        await expect(pane(page)).toHaveCount(0);
        await expect(header(page).locator(".room-title-name")).toBeInViewport();
      });
    }
  });
}

test.describe("on a 360 px touch phone", () => {
  test.use(PHONE_TOUCH);

  for (const [label, id, panes] of [
    [
      "a channel",
      ROOM_IDS.general,
      [
        ["Members", "Members"],
        ["Threads", "Threads"],
        ["Pinned messages", "Pinned messages"],
        ["Files", "Files"],
      ],
    ],
    [
      "a DM",
      ROOM_IDS.dmMaya,
      [
        ["Pinned messages", "Pinned messages"],
        ["Files", "Files"],
      ],
    ],
    ["a stage", ROOM_IDS.townHall, [["Stage", "Stage"]]],
  ] as const) {
    test(`each ⋯ item in ${label} opens its pane`, async ({ page }) => {
      await openRoom(page, id);

      for (const [item, heading] of panes) {
        const menu = await openOverflow(page);

        await menu.getByRole("menuitem", { name: new RegExp(`^${item}`) }).click();
        await expect(pane(page).getByRole("heading", { name: heading, exact: true })).toBeVisible();
        await pane(page)
          .getByRole("button", { name: /^Back to/ })
          .click();
        await expect(pane(page)).toHaveCount(0);
      }
    });
  }

  test("the ⋯ menu opens the notification levels, the calendar and search", async ({ page }) => {
    await openRoom(page, ROOM_IDS.general);

    let menu = await openOverflow(page);

    await menu.getByRole("menuitem", { name: "Notifications" }).click();
    await expect(page.getByRole("menuitemradio", { name: /^Mentions/ })).toBeVisible();
    await page.getByRole("menuitemradio", { name: /^Mentions/ }).click();
    await expect(menu).toBeHidden();

    menu = await openOverflow(page);
    await menu.getByRole("menuitem", { name: "Events" }).click();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}/events$`));

    await openRoom(page, ROOM_IDS.general);
    menu = await openOverflow(page);
    await menu.getByRole("menuitem", { name: "Search" }).click();
    await expect(page).toHaveURL(/\/search$/);
  });

  test("a DM's ⋯ menu adds people", async ({ page }) => {
    await openRoom(page, ROOM_IDS.dmMaya);

    const menu = await openOverflow(page);

    await menu.getByRole("menuitem", { name: "Add people" }).click();
    await expect(page.getByRole("dialog", { name: /Add people/ })).toBeVisible();
  });

  test("the classic notification URL opens the levels from the ⋯ button", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.general}/notifications`);

    const menu = page.getByRole("menu", { name: "Notifications", exact: true });

    await expect(menu).toBeVisible();
    await expect(menu.getByRole("menuitemradio")).toHaveCount(5);
    await page.keyboard.press("Escape");
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));

    // Closed, the ⋯ menu holds every tool again.
    const more = await openOverflow(page);

    await expect(more.getByRole("menuitem")).toHaveText(ROOMS[0]?.items ?? []);
  });

  test("a raised hand marks the ⋯ button, its Stage item and the details' Stage row", async ({
    page,
    request,
  }) => {
    const welcomed = syncWelcomed(page);

    await openRoom(page, ROOM_IDS.townHall);
    await welcomed;
    await expect(header(page).getByRole("button", { name: "More", exact: true })).toBeVisible();
    await expect(header(page).locator(".page-header-overflow-wrap .badge")).toHaveAttribute(
      "data-open",
      "false",
    );

    await control(request, "stage-hand", {
      roomId: ROOM_IDS.townHall,
      userId: USER_IDS.jonah,
      raised: true,
    });

    // A host sees the dot without opening the menu, and the count inside it.
    // One hand reads in the singular.
    await expect(
      header(page).getByRole("button", { name: "More (1 raised hand)", exact: true }),
    ).toBeVisible();
    await expect(header(page).locator(".page-header-overflow-wrap .badge")).toHaveAttribute(
      "data-open",
      "true",
    );
    await expectTouchTargets(page, ".room-header");
    await shot(page, "phone-header-hands", "light");

    const menu = await openOverflow(page);

    await expect(
      menu.getByRole("menuitem", { name: "Stage 1 raised hand", exact: true }),
    ).toBeVisible();
    await shot(page, "phone-header-hands-overflow", "light");
    await menu.getByRole("menuitem", { name: /^Stage/ }).click();
    await expect(pane(page).getByRole("region", { name: "Listeners" })).toContainText(
      "Hand raised",
    );
    await pane(page)
      .getByRole("button", { name: /^Back to/ })
      .click();

    await header(page).locator(".room-title-button").click();
    await expect(
      pane(page).getByRole("button", { name: "Stage 1 raised hand", exact: true }),
    ).toBeVisible();
    await shot(page, "phone-header-hands-details", "light");

    // Lowered, the dot goes.
    await control(request, "stage-hand", {
      roomId: ROOM_IDS.townHall,
      userId: USER_IDS.jonah,
      raised: false,
    });
    await expect(header(page).locator(".page-header-overflow-wrap .badge")).toHaveAttribute(
      "data-open",
      "false",
    );
    await expect(pane(page).getByRole("button", { name: "Stage", exact: true })).toBeVisible();
  });

  test("a DM's details lead to the agent's or the person's profile", async ({ page }) => {
    await openRoom(page, ROOM_IDS.dmEmber);
    await header(page).locator(".room-title-button").click();
    await pane(page).getByRole("link", { name: "Agent profile" }).click();
    await expect(page).toHaveURL(new RegExp(`/app/agents/${USER_IDS.ember}$`));
    await expect(page.getByRole("heading", { level: 2, name: "Ember" })).toBeVisible();

    await openRoom(page, ROOM_IDS.dmMaya);
    await header(page).locator(".room-title-button").click();
    await pane(page).getByRole("link", { name: "Profile", exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/app/people/${USER_IDS.maya}$`));
  });

  test("the details lead to each pane and back to the details", async ({ page }) => {
    await openRoom(page, ROOM_IDS.general);
    await header(page).locator(".room-title-button").click();

    for (const [row, heading] of [
      ["Members", "Members"],
      ["Threads", "Threads"],
      ["Pinned messages", "Pinned messages"],
      ["Files", "Files"],
    ] as const) {
      await pane(page)
        .getByRole("button", { name: new RegExp(`^${row}`) })
        .click();
      await expect(pane(page).getByRole("heading", { name: heading, exact: true })).toBeVisible();
      await pane(page).getByRole("button", { name: "Back to details" }).click();
      await expect(pane(page).getByRole("heading", { name: "Details" })).toBeVisible();
    }

    await pane(page)
      .getByRole("button", { name: /^Notifications/ })
      .click();
    await expect(page.getByRole("menuitemradio", { name: /^Mentions/ })).toBeVisible();
    await page.keyboard.press("Escape");

    await pane(page).getByRole("link", { name: "Channel settings" }).click();
    await expect(page.getByRole("dialog", { name: /settings/i })).toBeVisible();
  });

  test("the event pages lead back from the header", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.general}/events/${UPCOMING}`);

    const back = page
      .locator(".page-header")
      .getByRole("link", { name: /^All events in general$/ });

    await expect(back).toBeVisible();
    await expect(page.locator(".ev-crumb")).toBeHidden();
    await expectTouchTargets(page, ".page-header");
    await expectNoHorizontalOverflow(page);

    // Edit and Cancel event are in the ⋯ menu, not squeezed beside the title.
    await page.locator(".page-header").getByRole("button", { name: "More" }).click();
    await expect(page.getByRole("menuitem")).toHaveText(["Edit", "Cancel event"]);
    await page.keyboard.press("Escape");

    await back.click();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}/events$`));

    const toRoom = page.locator(".page-header").getByRole("link", { name: "Back to general" });

    await expect(toRoom).toBeVisible();
    await expect(
      page.locator(".page-header").getByRole("link", { name: "New event" }),
    ).toBeVisible();
    await expectTouchTargets(page, ".page-header");
    await toRoom.click();
    await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}$`));
  });

  for (const theme of ["light", "dark"] as const) {
    test(`screenshots (${theme})`, async ({ page }) => {
      await openRoom(page, ROOM_IDS.general, theme);
      await shot(page, "phone-header-room", theme);
      await openOverflow(page);
      await shot(page, "phone-header-overflow", theme);
      await page.keyboard.press("Escape");
      await header(page).locator(".room-title-button").click();
      await expect(pane(page).locator(".details-name")).toHaveText("general");
      await shot(page, "phone-header-details", theme);

      await openRoom(page, ROOM_IDS.dmMaya, theme);
      await shot(page, "phone-header-dm", theme);

      await openApp(page, `r/${ROOM_IDS.general}/events/${UPCOMING}`, theme);
      await page.locator(".ev-detail").waitFor();
      await shot(page, "phone-header-event", theme);
    });
  }
});
