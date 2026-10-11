import type { Page } from "@playwright/test";
import {
  DESKTOP,
  expect,
  expectNoHorizontalOverflow,
  openApp,
  PHONE,
  ROOM_IDS,
  test,
} from "./support.ts";

const NEWS = ROOM_IDS.announcements;

const ENGINEERING = ROOM_IDS.engineering;

const TEXT = "Launch moves to Thursday";

async function announcementsTab(page: Page, roomId: number) {
  await openApp(page, `r/${roomId}/settings`);

  const dialog = page.getByRole("dialog", { name: "Channel settings" });

  await dialog.getByRole("tab", { name: "Announcements" }).click();

  return dialog;
}

test("an announcement room is followed, publishes, and the copy names its source", async ({
  page,
}, testInfo) => {
  await page.setViewportSize(DESKTOP);

  // Make #announcements an announcement room; the header picks it up from sync.
  const news = await announcementsTab(page, NEWS);
  const flag = news.getByRole("switch", { name: /Announcement channel/ });

  await expect(flag).toHaveAttribute("aria-checked", "false");
  await flag.click();
  await expect(flag).toHaveAttribute("aria-checked", "true");
  await page.keyboard.press("Escape");
  await expect(news).toBeHidden();
  await expect(page.locator(".room-header .room-announcement-tag")).toHaveText("Announcement");

  // #engineering follows it.
  const engineering = await announcementsTab(page, ENGINEERING);
  const following = engineering.getByRole("region", { name: "Following" });

  await expect(following.getByText("Not following any announcement channels.")).toBeVisible();
  await expect(
    following.getByRole("combobox", { name: "Announcement channel to follow" }),
  ).toHaveValue(`${NEWS}`);
  await following.getByRole("button", { name: "Follow" }).click();
  await expect(following.locator(".room-follow-name")).toHaveText(["announcements"]);
  await expect(following.getByRole("button", { name: "Unfollow announcements" })).toBeVisible();

  // Post in #announcements and publish from the message menu.
  await openApp(page, `r/${NEWS}`);

  const input = page.locator(".composer-input").first();

  await input.fill(TEXT);
  await input.press("Enter");

  const source = page.locator("[data-message-row]", { hasText: TEXT }).last();

  await expect(source).toHaveAttribute("data-message-id", /^\d+$/);

  const sourceId = await source.getAttribute("data-message-id");

  await source.locator(".message-body").first().click({ button: "right" });
  await page.getByRole("menuitem", { name: /^Publish/ }).click();
  await expect(page.getByText("Published to the channels that follow this one")).toBeVisible();
  await expect(source.locator(".message-published")).toHaveText("Published");

  await source.locator(".message-body").first().click({ button: "right" });
  await expect(page.getByRole("menuitem", { name: /^Published/ })).toBeDisabled();
  await page.keyboard.press("Escape");
  await page.screenshot({ path: testInfo.outputPath("published-desktop.png") });

  // The copy in #engineering says where it came from and links back.
  await openApp(page, `r/${ENGINEERING}`);

  const copy = page.locator("[data-message-row]", { hasText: TEXT }).last();
  const origin = copy.getByRole("link", { name: /^#announcements · / });

  await expect(origin).toBeVisible();

  // At phone width the attribution still fits.
  await page.setViewportSize(PHONE);
  await expect(origin).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await page.screenshot({ path: testInfo.outputPath("copy-phone.png") });

  await origin.click();
  await expect(page).toHaveURL(new RegExp(`/app/r/${NEWS}/m/${sourceId}$`));
});

test("phone settings reach the Announcements tab without overflowing", async ({
  page,
}, testInfo) => {
  await page.setViewportSize(PHONE);

  const dialog = await announcementsTab(page, ENGINEERING);

  await expect(dialog.getByRole("region", { name: "Following" })).toBeVisible();
  await expect(
    dialog.getByText("There are no announcement channels you're in to follow."),
  ).toBeVisible();
  await expectNoHorizontalOverflow(page);
  await page.screenshot({ path: testInfo.outputPath("settings-phone.png") });
});
