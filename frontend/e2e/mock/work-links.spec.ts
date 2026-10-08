import { BOARD_POST_IDS, BOARD_ROOM_ID } from "../../mock/s6/seed.ts";
import { DESKTOP, expect, openApp, PHONE, shot, test } from "./support.ts";

const POST = BOARD_POST_IDS.apiPagination;

test("a post links a pull request, refuses a bad URL in the server's words, and unlinks it", async ({
  page,
}) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD_ROOM_ID}/t/${POST}`);

  const pane = page.locator("aside.right-pane");
  const linked = pane.getByRole("region", { name: "Linked" });

  await expect(linked).toBeVisible();
  await linked.getByRole("button", { name: "Link", exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD_ROOM_ID}/t/${POST}/links$`));

  const form = linked.getByRole("form", { name: "Link to this work" });

  await form.getByLabel("Pull request URL").fill("https://example.com/not-a-pr");
  await form.getByRole("button", { name: "Link pull request" }).click();
  await expect(
    form.getByText("Enter a GitHub pull request URL, like https://github.com/owner/repo/pull/123."),
  ).toBeVisible();
  await expect(form.getByLabel("Pull request URL")).toHaveAttribute("aria-invalid", "true");

  await form.getByLabel("Pull request URL").fill("https://github.com/acme/api/pull/77");
  await shot(page, "work-links-form", "light");
  await form.getByRole("button", { name: "Link pull request" }).click();

  await expect(form).toHaveCount(0);
  await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD_ROOM_ID}/t/${POST}$`));
  await expect(linked.getByRole("link", { name: "acme/api#77" })).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "work-links", "light");

  await linked.getByRole("button", { name: "Remove link acme/api#77" }).click();
  await expect(linked.getByRole("link", { name: "acme/api#77" })).toHaveCount(0);
});

test("the classic links URL opens the link form on its post", async ({ page }) => {
  await page.setViewportSize(PHONE);
  await openApp(page, `t/${POST}/links`, "dark");

  await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD_ROOM_ID}/t/${POST}/links$`));
  await expect(page.getByRole("form", { name: "Link to this work" })).toBeVisible();
  await shot(page, "work-links-form", "dark");
});
