import type { APIRequestContext, Page } from "@playwright/test";
import { expect, matrix, openApp, SHOTS, shot, test, USER_IDS } from "./support.ts";

/** The directory's row for `name`. */
function row(page: Page, name: string) {
  return page
    .getByRole("list", { name: "People" })
    .getByRole("listitem")
    .filter({ has: page.getByRole("link", { name, exact: true }) });
}

/** Opens the directory and waits for its rows. */
async function openPeople(page: Page, theme: "light" | "dark" = "light") {
  await openApp(page, "people", theme);
  await page.getByRole("list", { name: "People" }).waitFor();
}

/** Opens someone's page and waits for it to load. */
async function openPerson(page: Page, userId: number) {
  await openApp(page, `people/${userId}`);
  await page.locator(".person").waitFor();
}

/** Lapses the password confirmation: guarded writes wait for it in place. */
async function lapseSudo(request: APIRequestContext) {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/lapse-sudo", { headers: { "X-CSRF-Token": state.csrfToken } });
}

// Screenshots only: the tests below check the same pages.
if (SHOTS) {
  matrix("the people pages", async ({ page, theme }) => {
    await openPeople(page, theme);
    await expect(page.getByRole("heading", { level: 1, name: "People" })).toBeVisible();
    await shot(page, "people-directory", theme);

    await row(page, "Maya Okafor").getByRole("checkbox").check();
    await row(page, "Ember").getByRole("checkbox").check();
    await expect(page.getByRole("button", { name: "Message (2)" })).toBeVisible();
    await shot(page, "people-selected", theme);

    await openPerson(page, USER_IDS.priya);
    await expect(page.getByRole("heading", { level: 1, name: "Priya Raman" })).toBeVisible();
    await shot(page, "people-person", theme);
  });
}

test("the user menu opens the people directory in place", async ({ page }) => {
  await openApp(page, "");

  await page.getByRole("button", { name: "Your account" }).click();
  await page.getByRole("menuitem", { name: "People", exact: true }).click();

  await expect(page).toHaveURL(/\/app\/people$/);
  await expect(page.getByRole("heading", { level: 1, name: "People" })).toBeVisible();
  await expect(page).toHaveTitle(/People · Smartfire/);
});

test("the bar counts the selection and says agents won't be rung", async ({ page }) => {
  await openPeople(page);

  const bar = page.locator(".people-bar");

  await expect(bar).toBeHidden();
  await row(page, "Maya Okafor").getByRole("checkbox").check();
  await row(page, "Ember").getByRole("checkbox").check();

  await expect(bar.getByRole("button", { name: "Message (2)" })).toBeEnabled();
  await expect(bar.getByRole("button", { name: "Start huddle (1)" })).toBeEnabled();
  await expect(bar).toContainText("1 agent stays in the DM but won't be rung.");
  await expect(page.getByRole("status").filter({ hasText: "2 selected" })).toBeAttached();

  await row(page, "Maya Okafor").getByRole("checkbox").uncheck();
  await expect(bar.getByRole("button", { name: "Start huddle (0)" })).toBeDisabled();
  await expect(bar).toContainText("Agents can't join huddles.");

  await bar.getByRole("button", { name: "Clear" }).focus();
  await page.keyboard.press("Enter");
  await expect(bar).toBeHidden();
  await expect(page.getByRole("checkbox", { checked: true })).toHaveCount(0);
  await expect(row(page, "Maya Okafor").getByRole("checkbox")).toBeFocused();
});

test("Shift-click selects the people in between", async ({ page }) => {
  await openPeople(page);

  const boxes = page.getByRole("list", { name: "People" }).getByRole("checkbox");

  await boxes.nth(1).click();
  await boxes.nth(4).click({ modifiers: ["Shift"] });

  await expect(page.getByRole("checkbox", { checked: true })).toHaveCount(4);
  await expect(page.getByRole("button", { name: "Message (4)" })).toBeVisible();
});

test("Message opens the group DM with everyone chosen", async ({ page }) => {
  await openPeople(page);

  await row(page, "Maya Okafor").getByRole("checkbox").check();
  await row(page, "Jonah Lindqvist").getByRole("checkbox").check();
  await page.getByRole("button", { name: "Message (2)" }).click();

  await expect(page).toHaveURL(/\/app\/r\/\d+$/);
  await expect(page.getByRole("main")).toContainText("Maya Okafor");
});

test("a failed Message keeps the selection for another try", async ({ page }) => {
  await openPeople(page);
  await page.route("**/api/v1/directs", (route) =>
    route.fulfill({ status: 500, contentType: "application/json", body: "{}" }),
  );

  await row(page, "Maya Okafor").getByRole("checkbox").check();
  await page.getByRole("button", { name: "Message (1)" }).click();

  await expect(page.getByText("Couldn't start the conversation")).toBeVisible();
  await expect(page).toHaveURL(/\/app\/people$/);
  await expect(row(page, "Maya Okafor").getByRole("checkbox")).toBeChecked();
});

test("an administrator sees a person's email, status and sign-in link", async ({ page }) => {
  const qrRequests: string[] = [];

  page.on("request", (request) => {
    if (request.url().includes("/qr_code")) qrRequests.push(request.url());
  });
  await openPerson(page, USER_IDS.priya);

  const person = page.locator(".person");

  await expect(person.getByRole("link", { name: "priya@smartdata.example" })).toBeVisible();
  await expect(person).toContainText("Do not disturb");
  await expect(person).toContainText("Heads down until 3pm");
  await expect(page.getByLabel("Share to get them back into their account")).toHaveValue(
    /mock-person-4$/,
  );

  const show = page.getByRole("button", { name: "Show QR code" });

  await show.click();
  await expect(page.getByRole("img", { name: "QR code for the sign-in link" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(show).toBeFocused();
  expect(qrRequests).toEqual([]);
});

test("the DND exception toggles on a person's page", async ({ page }) => {
  await openPerson(page, USER_IDS.sam);

  const toggle = page.getByRole("button", { name: /during DND/ });
  const before = await toggle.getAttribute("aria-pressed");

  await toggle.click();
  await expect(toggle).not.toHaveAttribute("aria-pressed", before ?? "");
  await page.reload();
  await page.locator(".person").waitFor();
  await expect(page.getByRole("button", { name: /during DND/ })).not.toHaveAttribute(
    "aria-pressed",
    before ?? "",
  );
});

test("banning asks first, then the page offers to remove the ban", async ({ page }) => {
  await openPerson(page, USER_IDS.sam);

  await page.getByRole("button", { name: "Ban Sam Whitfield" }).click();

  const confirm = page.getByRole("alertdialog");

  await expect(confirm).toContainText("block their IP addresses");
  await confirm.getByRole("button", { name: "Cancel" }).click();
  await expect(page.locator(".person")).toHaveAttribute("data-status", "active");

  await page.getByRole("button", { name: "Ban Sam Whitfield" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Ban Sam Whitfield" }).click();

  await expect(page.locator(".person")).toHaveAttribute("data-status", "banned");
  await expect(page.getByText("Sam Whitfield is banned")).toBeVisible();
  await expect(page.getByRole("button", { name: "Message Sam Whitfield" })).toHaveCount(0);

  await page.getByRole("button", { name: "Remove ban" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Remove ban" }).click();
  await expect(page.locator(".person")).toHaveAttribute("data-status", "active");
});

test("a slow ban keeps its button focusable, and a slow DND change blocks nothing else", async ({
  page,
}) => {
  await openPerson(page, USER_IDS.sam);

  let release = () => {};

  const held = new Promise<void>((resolve) => {
    release = resolve;
  });

  await page.route("**/api/v1/people/*/ban", async (route) => {
    await held;
    await route.continue();
  });
  await page.route("**/api/v1/settings/dnd_allowances/*", async (route) => {
    await held;
    await route.continue();
  });

  await page.getByRole("button", { name: /during DND/ }).click();
  await expect(page.getByRole("button", { name: "Message Sam Whitfield" })).toBeEnabled();

  const opener = page.getByRole("button", { name: "Ban Sam Whitfield" });

  await opener.click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Ban Sam Whitfield" }).click();
  await expect(page.getByRole("alertdialog")).toBeHidden();
  await expect(opener).toBeFocused();
  await expect(opener).toHaveAttribute("aria-busy", "true");

  release();
  await expect(page.locator(".person")).toHaveAttribute("data-status", "banned");
  await expect(page.getByRole("button", { name: "Remove ban" })).toBeFocused();
});

test("a lapsed password confirmation is asked for in place, then the ban goes through", async ({
  page,
  request,
}) => {
  await openPerson(page, USER_IDS.sam);
  await lapseSudo(request);

  await page.getByRole("button", { name: "Ban Sam Whitfield" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Ban Sam Whitfield" }).click();

  const confirm = page.getByRole("dialog", { name: "Confirm it's you" });

  await confirm.getByLabel("Password").fill("secret123456");
  await confirm.getByRole("button", { name: "Confirm password" }).click();
  await expect(confirm).toBeHidden();
  await expect(page.locator(".person")).toHaveAttribute("data-status", "banned");
  await expect(page).toHaveURL(new RegExp(`/app/people/${USER_IDS.sam}$`));
});

test("your numeric own page opens your profile settings", async ({ page }) => {
  await openApp(page, `people/${USER_IDS.riel}`);

  await expect(page).toHaveURL(/\/app\/settings$/);
  await expect(page.getByRole("heading", { name: "Profile", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: /during DND/ })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^Ban / })).toHaveCount(0);
});

test("a legacy bot profile stays in the SPA with classic bot actions", async ({
  page,
  request,
}) => {
  await page.route("**/users/900/avatar", (route) =>
    route.fulfill({
      contentType: "image/svg+xml",
      body: '<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96"><rect width="96" height="96" fill="purple"/></svg>',
    }),
  );
  const bots = await request.get("/api/v1/admin/bots");

  expect(bots.ok()).toBe(true);
  await openPeople(page);
  await expect(row(page, "Deploy Bot")).toContainText("Bot");
  await row(page, "Deploy Bot").getByRole("link", { name: "Deploy Bot" }).click();
  await expect(page).toHaveURL(/\/app\/people\/900$/);
  await expect(page.getByRole("heading", { name: "Deploy Bot", exact: true })).toBeVisible();
  await expect(page.locator(".person .people-badge")).toHaveText("Bot");
  await expect(page.locator(".person [role=img]")).toHaveAttribute("aria-label", "Deploy Bot");
  await expect(page.locator(".person .avatar-image")).toHaveAttribute("src", "/users/900/avatar");
  await expect(page.getByRole("button", { name: "Message Deploy Bot" })).toBeEnabled();
  await expect(page.getByRole("button", { name: /during DND|^Ban / })).toHaveCount(0);
  await expect(page.locator(".person-email, .person-status, .person-transfer")).toHaveCount(0);
  await page.reload();
  await expect(page.getByRole("link", { name: "Manage capability grants" })).toHaveAttribute(
    "href",
    "/app/admin/bots/900/grants",
  );
  await page.getByRole("link", { name: "Manage capability grants" }).click();
  await expect(page).toHaveURL(/\/app\/admin\/bots\/900\/grants$/);
  await expect(
    page.getByRole("heading", { name: "Deploy Bot's grants", exact: true }),
  ).toBeVisible();
});

test("a member can message a legacy bot but cannot manage its grants", async ({
  page,
  request,
}) => {
  await request.get("/api/v1/admin/bots");
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/viewer-role", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { role: "member" },
  });
  await openPerson(page, 900);
  await expect(page.getByRole("link", { name: "Manage capability grants" })).toHaveCount(0);
  await page.getByRole("button", { name: "Message Deploy Bot" }).click();
  await expect(page).toHaveURL(/\/app\/r\/\d+$/);
});

test("a deactivated person's page says they've gone, and an unknown one says so", async ({
  page,
}) => {
  await openPerson(page, USER_IDS.dana);
  await expect(page.locator(".person")).toContainText("Dana Kowalski is no longer on this account");

  await openApp(page, "people/999");
  await expect(page.getByText("There's nobody here by that link.")).toBeVisible();
});

test("a bot's page opens its ported agent profile", async ({ page }) => {
  await openPeople(page);

  await expect(row(page, "Ember").getByRole("link", { name: "Ember" })).toHaveAttribute(
    "href",
    "/app/agents/9",
  );

  await page.goto("/app/people/9");
  await expect(page).toHaveURL(/\/app\/agents\/9$/);
  await expect(page.getByRole("heading", { level: 1, name: "Ember", exact: true })).toBeVisible();
});
