import type { APIRequestContext, Page } from "@playwright/test";
import { expect, matrix, openApp, shot, test, USER_IDS } from "./support.ts";

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

/** Lapses the password confirmation and stands in for the classic page that asks for it. */
async function lapseSudo(page: Page, request: APIRequestContext) {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/lapse-sudo", { headers: { "X-CSRF-Token": state.csrfToken } });
  await page.route("**/sudo/new", (route) =>
    route.fulfill({
      status: 200,
      contentType: "text/html",
      body: "<h1>Confirm your password</h1>",
    }),
  );
}

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

test("the user menu opens the people directory in place", async ({ page }) => {
  await openApp(page, "");

  await page.getByRole("button", { name: "Your account" }).click();
  await page.getByRole("menuitem", { name: "People", exact: true }).click();

  await expect(page).toHaveURL(/\/app\/people$/);
  await expect(page.getByRole("heading", { level: 1, name: "People" })).toBeVisible();
  await expect(page).toHaveTitle(/People · Smartfire/);
});

test("the directory lists everyone else with their badges", async ({ page }) => {
  await openPeople(page);

  await expect(row(page, "Riel St. Amand")).toHaveCount(0);
  await expect(row(page, "Dana Kowalski")).toHaveCount(0);
  await expect(row(page, "Ember")).toContainText("Agent");
  await expect(row(page, "Maya Okafor")).toContainText("Online");
  await expect(row(page, "Sam Whitfield")).toContainText("Offline");
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

test("a DND change that finishes after you leave and come back still shows", async ({ page }) => {
  await openPerson(page, USER_IDS.sam);

  let release = () => {};

  const held = new Promise<void>((resolve) => {
    release = resolve;
  });

  await page.route("**/api/v1/settings/dnd_allowances/*", async (route) => {
    await held;
    await route.continue();
  });

  const toggle = page.getByRole("button", { name: /during DND/ });
  const before = await toggle.getAttribute("aria-pressed");

  await toggle.click();
  await page.getByRole("button", { name: "Your account" }).click();
  await page.getByRole("menuitem", { name: "People", exact: true }).click();
  await expect(page).toHaveURL(/\/app\/people$/);
  await row(page, "Sam Whitfield").getByRole("link", { name: "Sam Whitfield" }).click();
  await page.locator(".person").waitFor();
  await expect(page.getByRole("button", { name: /during DND/ })).toHaveAttribute(
    "aria-pressed",
    before ?? "",
  );

  release();
  await expect(page.getByRole("button", { name: /during DND/ })).not.toHaveAttribute(
    "aria-pressed",
    before ?? "",
  );
});

test("a page whose every reply is older than the copy held says so and offers to try again", async ({
  page,
}) => {
  await openPeople(page);

  let requests = 0;

  await page.route(`**/api/v1/people/${USER_IDS.priya}`, async (route) => {
    requests += 1;

    const response = await route.fetch();
    const profile = await response.json();

    profile.user.updatedAt = "2000-01-01T00:00:00.000000Z";
    await route.fulfill({ response, json: profile });
  });
  await row(page, "Priya Raman").getByRole("link", { name: "Priya Raman" }).click();

  await expect(page.getByText("Couldn't load the latest profile")).toBeVisible();
  // One request and three refetches; Strict Mode's rehearsal load adds one, stopped once it leaves.
  expect(requests).toBeGreaterThanOrEqual(4);
  expect(requests).toBeLessThanOrEqual(5);
  await page.unroute(`**/api/v1/people/${USER_IDS.priya}`);
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(page.locator(".person")).toBeVisible();
});

test("Message on a person's page opens your DM with them", async ({ page }) => {
  await openPerson(page, USER_IDS.grace);

  await page.getByRole("button", { name: "Message Grace Adeyemi" }).click();

  await expect(page).toHaveURL(/\/app\/r\/\d+$/);
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

test("a lapsed password confirmation sends a ban to the classic password page", async ({
  page,
  request,
}) => {
  await openPerson(page, USER_IDS.sam);
  await lapseSudo(page, request);

  await page.getByRole("button", { name: "Ban Sam Whitfield" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Ban Sam Whitfield" }).click();

  await expect(page).toHaveURL(/\/sudo\/new$/);
});

test("your own page links to your settings and has no DND toggle or ban button", async ({
  page,
}) => {
  await openPerson(page, USER_IDS.riel);

  await expect(page.getByRole("link", { name: "Edit my profile" })).toBeVisible();
  await expect(page.getByRole("button", { name: /during DND/ })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /^Ban / })).toHaveCount(0);
  await expect(
    page.getByLabel("Use this link to login automatically on another device"),
  ).toBeVisible();
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
  await expect(page.getByRole("heading", { name: "Ember", exact: true })).toBeVisible();
});
