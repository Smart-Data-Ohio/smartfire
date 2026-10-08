import { BOARD_POST_IDS, BOARD_ROOM_ID } from "../../mock/s6/seed.ts";
import { DESKTOP, expect, openApp, PHONE, ROOM_IDS, shot, test } from "./support.ts";

const POST = BOARD_POST_IDS.apiPagination;

test("Work lists tracked threads by filter and hands a post off to an agent", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${ROOM_IDS.general}`);
  await expect(page.getByRole("heading", { name: "general", level: 1 })).toBeVisible();

  await page.getByRole("link", { name: "Work", exact: true }).click();
  await expect(page).toHaveURL(/\/app\/work$/);
  await expect(page.getByRole("heading", { name: "Work", level: 1 })).toBeVisible();

  const rows = page.getByRole("list", { name: "Work threads" });

  await expect(page.getByRole("tab", { name: "Open" })).toHaveAttribute("aria-selected", "true");
  await expect(rows.getByRole("link", { name: /^Cursor pagination for the API/ })).toBeVisible();
  await expect(rows.getByRole("link", { name: /^Pricing page refresh/ })).toHaveCount(0);
  await page.mouse.move(0, 0);
  await shot(page, "work-open", "light");

  await page.getByRole("tab", { name: "Done" }).click();
  await expect(page).toHaveURL(/state=done/);
  await expect(rows.getByRole("link", { name: /^Pricing page refresh/ })).toBeVisible();
  await expect(rows.getByRole("link", { name: /^Cursor pagination/ })).toHaveCount(0);

  await page.getByRole("tab", { name: "Agents" }).click();
  await expect(page).toHaveURL(/state=agents/);
  // One row each (a row lists its linked items too).
  await expect(rows.locator(".list-row")).toHaveCount(2);
  await shot(page, "work-agents", "light");

  await page.getByRole("tab", { name: "Open" }).click();
  await expect(page).not.toHaveURL(/state=/);
  await rows.getByRole("link", { name: /^Cursor pagination for the API/ }).click();
  await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD_ROOM_ID}/t/${POST}`));

  const pane = page.locator("aside.right-pane");

  await pane.getByRole("link", { name: "Hand off" }).click();
  await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD_ROOM_ID}/t/${POST}/handoff$`));

  const dialog = page.getByRole("dialog", { name: "Hand off “Cursor pagination for the API”" });

  await expect(dialog).toBeVisible();
  // Ember is the only agent here, so it's already chosen.
  await expect(dialog.getByLabel("Receiving agent")).toHaveValue(/\d+/);
  await expect(dialog.getByRole("option", { name: "Ember" })).toHaveCount(1);

  await dialog.getByRole("button", { name: "Hand off" }).click();
  await expect(dialog.getByText("Summary can't be blank.")).toBeVisible();

  await dialog.getByLabel("Summary").fill("The cursor shape is agreed; the endpoint is next.");
  await dialog.getByLabel("Links").fill("https://example.com/spec");
  await dialog.getByLabel("Open questions").fill("What page size do we default to?");
  await shot(page, "work-handoff", "light");
  await dialog.getByRole("button", { name: "Hand off" }).click();

  await expect(page.getByText("Work handed off to Ember.")).toBeVisible();
  await expect(dialog).toHaveCount(0);
  await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD_ROOM_ID}/t/${POST}$`));
  await expect(pane.getByText("Ember").first()).toBeVisible();
});

test("the classic handoff URL opens the SPA dialog on its post", async ({ page }) => {
  await page.setViewportSize(PHONE);
  await openApp(page, `t/${POST}/handoff`, "dark");

  await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD_ROOM_ID}/t/${POST}/handoff$`));
  await expect(page.getByRole("dialog", { name: /^Hand off/ })).toBeVisible();
  await shot(page, "work-handoff", "dark");
});
