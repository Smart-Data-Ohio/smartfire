import { DESKTOP, expect, matrix, openApp, ROOM_IDS, shot, test } from "./support.ts";

const UPCOMING = 8001;

const WEEKLY_SECOND = 8101;

test.describe("a room's events", () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize(DESKTOP);
  });

  test("the header's calendar opens the events; a row opens its page, and Going counts", async ({
    page,
  }) => {
    await openApp(page, `r/${ROOM_IDS.general}`);
    await page.getByRole("link", { name: "Events", exact: true }).click();

    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}/events$`));
    await expect(page.getByRole("heading", { level: 1, name: "Events" })).toBeVisible();

    const tabs = page.getByRole("tablist", { name: "Events" });

    await expect(tabs.getByRole("tab", { name: /Upcoming/ })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await expect(page.getByRole("link", { name: "Cancelled coffee chat" })).toHaveCount(0);
    await tabs.getByRole("tab", { name: /Cancelled/ }).click();
    await expect(page.getByRole("link", { name: "Cancelled coffee chat" })).toBeVisible();
    await tabs.getByRole("tab", { name: /Upcoming/ }).click();

    await page.getByRole("link", { name: "Team check-in" }).click();
    await expect(page).toHaveURL(new RegExp(`/events/${UPCOMING}$`));
    await expect(page.getByRole("heading", { level: 2, name: "Team check-in" })).toBeVisible();
    await expect(page.getByText("You haven't answered yet.")).toBeVisible();

    const attendees = page.getByRole("region", { name: /Attendees/ });

    await expect(attendees).toContainText("1 going");
    await page.getByRole("button", { name: "Going" }).click();
    await expect(page.getByRole("button", { name: "Going" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await expect(page.getByText("Currently: Going")).toBeVisible();
    await expect(attendees).toContainText("2 going");

    await page.getByRole("link", { name: /All events in/ }).click();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}/events$`));
  });

  test("a new event asks for a title, then opens its page; Back doesn't reopen the form", async ({
    page,
  }) => {
    await openApp(page, `r/${ROOM_IDS.general}/events`);
    await page.getByRole("link", { name: "New event" }).click();

    const dialog = page.getByRole("dialog", { name: "Schedule an event" });
    const title = dialog.getByLabel("Title");

    await expect(title).toBeFocused();
    await expect(dialog.getByText(/Times are in your time zone/)).toBeVisible();
    await dialog.getByRole("button", { name: "Schedule event" }).click();
    await expect(dialog.getByText("Give the event a title.")).toBeVisible();

    await title.fill("Launch retro");
    await dialog.getByLabel("Where").selectOption({ label: "Lounge" });
    await dialog.getByLabel("Repeats").selectOption({ label: "Weekly" });
    await expect(dialog.getByLabel("Until")).toBeVisible();
    await dialog.getByLabel("Repeats").selectOption({ label: "Does not repeat" });
    await dialog.getByRole("button", { name: "Schedule event" }).click();

    await expect(dialog).toBeHidden();
    await expect(page.getByText("Event scheduled.")).toBeVisible();
    await expect(page.getByRole("heading", { level: 2, name: "Launch retro" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Lounge", exact: true })).toBeVisible();

    await page.goBack();
    await expect(page).toHaveURL(new RegExp(`/app/r/${ROOM_IDS.general}/events$`));
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });

  test("editing a series' later event asks how far the change goes", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.engineering}/events/${WEEKLY_SECOND}`);
    await expect(page.getByText(/Repeats weekly/i)).toBeVisible();
    await expect(page.getByRole("link", { name: "Previous" })).toBeVisible();
    await expect(page.getByLabel("Apply to all future occurrences")).toBeVisible();

    await page.getByRole("link", { name: "Edit" }).click();

    const dialog = page.getByRole("dialog", { name: "Edit event" });

    await expect(dialog.getByRole("radio", { name: "This event" })).toBeChecked();
    await expect(dialog.getByLabel("Repeats")).toHaveCount(0);
    await dialog.getByLabel("Title").fill("Engineering weekly (moved)");
    await expect(dialog.getByText("Unsaved changes")).toBeVisible();
    await dialog.getByRole("button", { name: "Save changes" }).click();

    await expect(dialog).toBeHidden();
    await expect(page.getByText("Event updated.")).toBeVisible();
    await expect(
      page.getByRole("heading", { level: 2, name: "Engineering weekly (moved)" }),
    ).toBeVisible();
    await expect(page).toHaveURL(new RegExp(`/events/${WEEKLY_SECOND}$`));

    await page.goBack();
    await expect(page).not.toHaveURL(/\/edit$/);
  });

  test("cancelling asks first, then closes responses", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.general}/events/${UPCOMING}`);
    await page.getByRole("button", { name: "Cancel event" }).click();

    const confirm = page.getByRole("alertdialog", { name: "Cancel this event?" });

    await expect(confirm.getByRole("button", { name: "Keep event" })).toBeFocused();
    await confirm.getByRole("button", { name: "Cancel event" }).click();

    await expect(confirm).toBeHidden();
    await expect(page.getByText("Event cancelled.")).toBeVisible();
    await expect(page.getByText("This event was cancelled.", { exact: true })).toBeVisible();
    await expect(
      page.getByText("Responses are closed because this event was cancelled."),
    ).toBeVisible();
    await expect(page.getByRole("link", { name: "Edit" })).toHaveCount(0);
  });

  test("the attendance URL opens the event on the viewer's response", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.general}/events/${UPCOMING}/attendance`);

    await expect(page.getByRole("button", { name: "Going" })).toBeFocused();
  });

  test("an event that isn't there says so", async ({ page }) => {
    await openApp(page, `r/${ROOM_IDS.general}/events/987654`);

    await expect(page.getByText("This event isn't here")).toBeVisible();
  });
});

matrix("the events list and an event's page", async ({ page, theme }) => {
  await openApp(page, `r/${ROOM_IDS.engineering}/events`, theme);
  await expect(page.getByRole("link", { name: "Engineering weekly" }).first()).toBeVisible();
  await shot(page, "events-list", theme);

  await page.goto(`/app/r/${ROOM_IDS.engineering}/events/${WEEKLY_SECOND}`);
  await expect(page.getByRole("heading", { level: 2, name: "Engineering weekly" })).toBeVisible();
  await shot(page, "event-page", theme);

  await page.goto(`/app/r/${ROOM_IDS.engineering}/events/new`);
  await expect(
    page.getByRole("dialog", { name: "Schedule an event" }).getByLabel("Title"),
  ).toBeVisible();
  await shot(page, "event-form", theme);
});
