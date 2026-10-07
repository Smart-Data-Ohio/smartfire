import type { APIRequestContext, Page } from "@playwright/test";
import { expect, matrix, openApp, shot, test } from "./support.ts";

/** Opens a settings section (`""` for the profile) with motion reduced. */
async function openSettings(page: Page, section: string, theme: "light" | "dark" = "light") {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/settings${section === "" ? "" : `/${section}`}`);
  await page.locator(".settings-page h1").waitFor();
}

function nav(page: Page) {
  return page.getByRole("navigation", { name: "Settings sections" });
}

/** Lets entrance animations finish so a shot is settled. */
async function settle(page: Page): Promise<void> {
  await page.mouse.move(0, 0);
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
        .map((animation) => animation.finished.catch(() => animation)),
    ),
  );
}

matrix("the settings sections", async ({ page, theme }) => {
  await openSettings(page, "", theme);

  await expect(page.getByRole("heading", { level: 1, name: "Profile" })).toBeVisible();
  await expect(nav(page).getByRole("link", { name: "Profile" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  await settle(page);
  await shot(page, "settings-profile", theme);

  for (const [link, heading, name] of [
    ["Status", "Status", "settings-status"],
    ["Notifications", "Notifications", "settings-notifications"],
    ["Rooms", "Rooms", "settings-rooms"],
    ["Appearance", "Appearance", "settings-appearance"],
    ["Security", "Security", "settings-security"],
    ["Sessions", "Sessions", "settings-sessions"],
    ["Push devices", "Push devices", "settings-devices"],
    ["Integrations", "Integrations", "settings-integrations"],
  ] as const) {
    await nav(page).getByRole("link", { name: link }).click();
    await expect(page.getByRole("heading", { level: 1, name: heading })).toBeVisible();
    await settle(page);
    await shot(page, name, theme);
  }
});

test("the user menu opens settings in place", async ({ page }) => {
  await openApp(page, "");

  await page.getByRole("button", { name: "Your account" }).click();
  await page.getByRole("menuitem", { name: "Profile and settings" }).click();

  await expect(page).toHaveURL(/\/app\/settings$/);
  await expect(page.getByRole("heading", { level: 1, name: "Profile" })).toBeVisible();
  // The shell stayed: no full page load.
  await expect(page.getByRole("complementary", { name: "Conversations" })).toBeVisible();
});

test("an email change asks for the current password", async ({ page }) => {
  await openSettings(page, "");

  await page.getByRole("textbox", { name: "Email address" }).fill("riel@new.example");

  const current = page.getByLabel("Current password");

  await expect(current).toBeVisible();
  await page.getByRole("button", { name: "Save profile" }).click();
  await expect(
    page.getByText("Current password is required to change your email address."),
  ).toBeVisible();

  await current.fill("secret123456");
  await page.getByRole("button", { name: "Save profile" }).click();
  await expect(page.getByText("Profile saved")).toBeVisible();
  await expect(current).toBeHidden();
});

test("a notification switch saves as it flips", async ({ page }) => {
  await openSettings(page, "notifications");

  const dnd = page.getByRole("switch", { name: /^Do not disturb Silence/ });

  await expect(dnd).toHaveAttribute("aria-checked", "false");
  await dnd.click();
  await expect(dnd).toHaveAttribute("aria-checked", "true");

  await page.reload();
  await expect(page.getByRole("switch", { name: /^Do not disturb Silence/ })).toHaveAttribute(
    "aria-checked",
    "true",
  );
});

test("someone can be let through DND", async ({ page }) => {
  await openSettings(page, "notifications");

  const exceptions = page.locator(".settings-exceptions");

  await exceptions.getByRole("button", { name: "Add someone" }).click();
  await exceptions.getByRole("searchbox", { name: "Add someone" }).fill("Jonah");
  await exceptions.getByRole("button", { name: /Jonah/ }).click();

  await expect(
    exceptions.getByRole("button", { name: /^Remove Jonah .* from DND exceptions$/ }),
  ).toBeVisible();
});

test("a theme choice shows at once", async ({ page }) => {
  await openSettings(page, "appearance");

  await page.getByRole("radio", { name: "Dark" }).check();

  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.getByRole("radio", { name: "Dark" })).toBeChecked();
});

test("signing out every other session", async ({ page }) => {
  await openSettings(page, "sessions");

  const rows = page.locator(".settings-session");

  await expect(rows).toHaveCount(3);
  await page.getByRole("button", { name: "Sign out of all other sessions" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Sign out" }).click();

  await expect(page.getByText("Signed out 2 other sessions.")).toBeVisible();
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText("This device");
});

test("an out-of-office end in the past is refused in place", async ({ page }) => {
  await openSettings(page, "status");

  await page.getByLabel("Out until").selectOption("custom");
  await page.getByLabel("Custom end").fill("2020-01-01T09:00");
  await page.getByRole("button", { name: "Save out of office" }).click();

  await expect(page.getByText("Out of office needs a future date and time.")).toBeVisible();
});

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

/** The integrations page's group for `title`. */
function group(page: Page, title: string) {
  return page.getByRole("region", { name: title, exact: true });
}

test("a pasted Fizzy token connects, a refused one says why, and disconnect asks first", async ({
  page,
}) => {
  await openSettings(page, "integrations");

  const fizzy = group(page, "Fizzy");
  const token = fizzy.getByLabel("Fizzy personal access token");

  await token.fill("bad-token");
  await fizzy.getByRole("button", { name: "Connect Fizzy" }).click();
  await expect(fizzy.getByText("Fizzy rejected that token. Check it and try again.")).toBeVisible();
  await expect(token).toHaveAttribute("aria-invalid", "true");

  await token.fill("fizzy-token");
  await fizzy.getByRole("button", { name: "Connect Fizzy" }).click();
  await expect(page.getByText("Fizzy connected as Riel (Smart Data).")).toBeVisible();
  await expect(fizzy.getByText("Connected as Riel (Smart Data).")).toBeVisible();
  // The form went away: focus lands on what replaced it.
  await expect(fizzy.getByRole("button", { name: "Disconnect Fizzy" })).toBeFocused();

  await fizzy.getByRole("button", { name: "Disconnect Fizzy" }).click();

  const ask = page.getByRole("alertdialog", { name: "Disconnect Fizzy?" });

  await expect(ask).toContainText("Card previews will stop working");
  await ask.getByRole("button", { name: "Disconnect" }).click();
  await expect(page.getByText("Fizzy disconnected.")).toBeVisible();
  await expect(fizzy.getByRole("button", { name: "Connect Fizzy" })).toBeVisible();
  await expect(token).toBeFocused();
});

test("disconnecting GitHub offers the app or a token, and frees the profile's username", async ({
  page,
}) => {
  await openSettings(page, "integrations");

  const github = group(page, "GitHub");

  await github.getByRole("button", { name: "Disconnect GitHub" }).click();
  await page
    .getByRole("alertdialog", { name: "Disconnect GitHub?" })
    .getByRole("button", { name: "Disconnect" })
    .click();
  await expect(page.getByText("GitHub disconnected.")).toBeVisible();

  await expect(github.getByRole("link", { name: "Connect with GitHub" })).toHaveAttribute(
    "href",
    "/github/app/connect",
  );
  await expect(github.getByRole("link", { name: "Connect with GitHub" })).toBeFocused();
  await github.getByText("Or paste a personal access token instead").click();
  await github.getByLabel("GitHub personal access token").fill("github_pat_1");
  await github.getByRole("button", { name: "Connect GitHub" }).click();
  await expect(page.getByText("GitHub connected as riel.")).toBeVisible();
  await expect(github.getByRole("button", { name: "Disconnect GitHub" })).toBeFocused();

  await nav(page).getByRole("link", { name: "Profile" }).click();
  await expect(page.getByLabel("GitHub username")).toBeDisabled();
});

test("Google starts are full page loads with the session's token", async ({ page }) => {
  let posted: URLSearchParams | null = null;

  await page.route("**/google/connect", async (route) => {
    posted = new URLSearchParams(route.request().postData() ?? "");
    await route.fulfill({ status: 200, contentType: "text/html", body: "<p>to Google</p>" });
  });
  await openSettings(page, "integrations");

  await group(page, "Google Calendar")
    .getByRole("button", { name: "Enable Drive previews" })
    .click();
  await expect(page.getByText("to Google")).toBeVisible();
  expect(posted && [...posted]).toEqual([
    ["features[]", "drive"],
    ["authenticity_token", expect.any(String)],
  ]);
});

test("disconnecting Google Calendar asks first and offers to connect again", async ({ page }) => {
  await openSettings(page, "integrations");

  const calendar = group(page, "Google Calendar");

  await calendar.getByRole("button", { name: "Disconnect" }).click();
  await expect(
    page.getByRole("alertdialog", { name: "Disconnect Google Calendar?" }),
  ).toContainText("Your published event entries will be removed.");
  await page
    .getByRole("alertdialog", { name: "Disconnect Google Calendar?" })
    .getByRole("button", { name: "Disconnect" })
    .click();
  await expect(page.getByText("Google Calendar disconnected.")).toBeVisible();
  await expect(calendar.getByRole("button", { name: "Connect Google Calendar" })).toBeFocused();
});

test("dropping Google Calendar ends a calendar out of office on the status page", async ({
  page,
}) => {
  const until = new Date(Date.now() + 2 * 24 * 60 * 60_000).toISOString();
  let calendarOoo = true;

  // Until Google is dropped, loads show an out of office read from the calendar's meeting cache
  // (every load: development mode loads the page twice).
  await page.route("**/api/v1/settings", async (route) => {
    if (!calendarOoo || route.request().method() !== "GET") {
      await route.fallback();

      return;
    }

    const response = await route.fetch();
    const json = await response.json();

    json.status = { ...json.status, oooCalendarEnabled: true, oooUntil: until, oooManual: false };
    await route.fulfill({ response, json });
  });
  await openSettings(page, "status");
  await expect(page.getByText(/from your Google Calendar\./)).toBeVisible();

  await nav(page).getByRole("link", { name: "Integrations" }).click();
  await group(page, "Google Calendar").getByRole("button", { name: "Disconnect" }).click();
  calendarOoo = false;
  await page
    .getByRole("alertdialog", { name: "Disconnect Google Calendar?" })
    .getByRole("button", { name: "Disconnect" })
    .click();
  await expect(page.getByText("Google Calendar disconnected.")).toBeVisible();

  await nav(page).getByRole("link", { name: "Status" }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Status" })).toBeVisible();
  await expect(page.getByText(/from your Google Calendar\./)).toBeHidden();
});

test("a slow reload from an earlier change never undoes a later one", async ({ page }) => {
  await openSettings(page, "integrations");

  const github = group(page, "GitHub");
  const held = Promise.withResolvers<void>();
  let holding = true;
  let heldReload = false;

  // Answer the reload the disconnect starts (GitHub missing), but deliver it only after the
  // reconnect has fully landed.
  await page.route("**/api/v1/settings", async (route) => {
    if (!holding || route.request().method() !== "GET") {
      await route.fallback();

      return;
    }

    holding = false;

    const response = await route.fetch();

    heldReload = true;
    await held.promise;
    await route.fulfill({ response });
  });

  await github.getByRole("button", { name: "Disconnect GitHub" }).click();
  await page
    .getByRole("alertdialog", { name: "Disconnect GitHub?" })
    .getByRole("button", { name: "Disconnect" })
    .click();
  await expect(page.getByText("GitHub disconnected.")).toBeVisible();
  await expect.poll(() => heldReload).toBe(true);

  const reload = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/settings") && response.request().method() === "GET",
  );

  await github.getByText("Or paste a personal access token instead").click();
  await github.getByLabel("GitHub personal access token").fill("github_pat_1");
  await github.getByRole("button", { name: "Connect GitHub" }).click();
  await expect(page.getByText("GitHub connected as riel.")).toBeVisible();
  await reload;

  // The disconnect's reload (GitHub missing) lands last.
  const stale = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/settings") && response.request().method() === "GET",
  );

  held.resolve();
  await stale;
  await expect(github.getByRole("button", { name: "Disconnect GitHub" })).toBeVisible();
  await expect(github.getByText(/^Comments and reviews you post/)).toBeVisible();
});

test("a lapsed password confirmation goes to confirm it", async ({ page, request }) => {
  await lapseSudo(page, request);
  await openSettings(page, "integrations");

  await group(page, "GitHub").getByRole("button", { name: "Disconnect GitHub" }).click();
  await page
    .getByRole("alertdialog", { name: "Disconnect GitHub?" })
    .getByRole("button", { name: "Disconnect" })
    .click();

  await expect(page.getByRole("heading", { name: "Confirm your password" })).toBeVisible();
});

test("a theme the server refuses is put back", async ({ page }) => {
  await page.route("**/api/v1/settings/appearance", (route) =>
    route.fulfill({ status: 500, body: "" }),
  );
  await openSettings(page, "appearance");

  const html = page.locator("html");

  // A click, not check(): check() insists the radio stays checked, and the refusal can put it
  // back before Playwright looks.
  await page.getByRole("radio", { name: "Dark" }).click();

  await expect(page.getByText("Couldn't save your appearance")).toBeVisible();
  await expect(html).not.toHaveAttribute("data-theme", "dark");
  await expect(page.getByRole("radio", { name: "Dark" })).not.toBeChecked();
});

test("saving one status setting keeps what's typed in the others", async ({ page }) => {
  await openSettings(page, "status");

  await page.getByRole("textbox", { name: "Status text" }).fill("On the train");
  await page.getByRole("textbox", { name: "Note" }).fill("Back Monday");
  const presence = page.getByRole("combobox", { name: "Presence" });

  await presence.selectOption({ index: 1 });
  await expect(presence).toBeEnabled();

  await expect(page.getByRole("textbox", { name: "Status text" })).toHaveValue("On the train");
  await expect(page.getByRole("textbox", { name: "Note" })).toHaveValue("Back Monday");
});

test("a room's notification level changes from the rooms list and stays", async ({ page }) => {
  await openSettings(page, "rooms");

  const rooms = page.locator(".settings-page").getByRole("region", { name: "Rooms", exact: true });
  const level = rooms.getByRole("button", { name: /^Notifications for / }).first();
  const label = (await level.getAttribute("aria-label")) ?? "";
  const room = label.replace(/^Notifications for /, "").replace(/: .*$/, "");

  await level.click();
  await page.getByRole("menuitemradio", { name: /No notifications/ }).click();
  await expect(level).toHaveAccessibleName(`Notifications for ${room}: No notifications`);

  await page.reload();
  await expect(
    page.getByRole("button", { name: `Notifications for ${room}: No notifications` }),
  ).toBeVisible();
  await expect(
    page.locator(".settings-page").getByRole("region", { name: "Direct messages" }),
  ).toBeVisible();
});

test("new backup codes ask for a code first, then show once", async ({ page }) => {
  await openSettings(page, "security");

  await page.getByRole("button", { name: "New backup codes" }).click();

  const ask = page.getByRole("dialog", { name: "New backup codes?" });
  const field = ask.getByLabel("Authenticator code or password");

  await field.fill("000000");
  await ask.getByRole("button", { name: "Make new codes" }).click();
  await expect(
    ask.getByText("Enter your authenticator code or password to continue."),
  ).toBeVisible();
  await expect(field).toHaveAttribute("aria-invalid", "true");

  await field.fill("123456");
  await ask.getByRole("button", { name: "Make new codes" }).click();

  const codes = page.getByRole("dialog", { name: "Your new backup codes" });

  await expect(codes.getByRole("list", { name: "Backup codes" }).getByRole("listitem")).toHaveCount(
    10,
  );
  await codes.getByRole("button", { name: "Done" }).click();
  await expect(codes).toBeHidden();
});

test("too many tries are refused in place", async ({ page }) => {
  await openSettings(page, "security");

  await page.getByRole("button", { name: "Forget all browsers" }).click();

  const ask = page.getByRole("dialog", { name: "Forget every browser?" });

  await ask.getByLabel("Authenticator code or password").fill("limit");
  await ask.getByRole("button", { name: "Forget all" }).click();
  await expect(ask.getByText("Too many attempts. Try again in a few minutes.")).toBeVisible();
});

test("forgetting one remembered browser", async ({ page }) => {
  await openSettings(page, "security");

  const rows = page.getByRole("region", { name: "Two-step sign-in" }).getByRole("listitem");

  await expect(rows).toHaveCount(2);
  await rows.first().getByRole("button", { name: "Forget" }).click();

  const ask = page.getByRole("dialog", { name: "Forget this browser?" });

  await ask.getByLabel("Authenticator code or password").fill("123456");
  await ask.getByRole("button", { name: "Forget" }).click();
  await expect(
    page.getByText("Device forgotten. It will ask for a code at next sign-in."),
  ).toBeVisible();
  await expect(rows).toHaveCount(1);
  await expect(page.getByRole("button", { name: "Forget all browsers" })).toBeHidden();
});

test("turning two-step sign-in off goes to the setup page", async ({ page }) => {
  await page.route("**/two_factor_setup", (route) =>
    route.fulfill({ status: 200, contentType: "text/html", body: "<h1>Set up two-step</h1>" }),
  );
  await openSettings(page, "security");

  await page.getByRole("button", { name: "Turn off" }).click();

  const ask = page.getByRole("dialog", { name: "Turn off two-step sign-in?" });

  await ask.getByLabel("Authenticator code or password").fill("123456");
  await ask.getByRole("button", { name: "Turn off" }).click();
  await expect(page.getByRole("heading", { name: "Set up two-step" })).toBeVisible();
});

test("the sign-in link shows its QR code from the classic image", async ({ page }) => {
  await openSettings(page, "security");

  const link = page.getByRole("region", { name: "Sign in on another device" });

  await expect(link.getByLabel("Your sign-in link")).toHaveValue(/\/session\/transfers\//);
  await link.getByRole("button", { name: "Show QR code" }).click();
  await expect(
    page.getByRole("dialog", { name: "Scan to sign in" }).getByRole("img"),
  ).toHaveAttribute("src", /^\/qr_code\/[A-Za-z0-9_-]+=*$/);
});

test("a push device gets a test notification", async ({ page }) => {
  await openSettings(page, "devices");

  await page.getByRole("button", { name: "Send a test" }).first().click();
  await expect(page.getByText("Test notification sent")).toBeVisible();
});
