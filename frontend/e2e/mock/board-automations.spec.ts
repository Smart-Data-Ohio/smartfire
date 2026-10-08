import type { Page } from "@playwright/test";
import { BOARD_ROOM_ID } from "../../mock/s6/seed.ts";
import { DESKTOP, expect, matrix, openApp, shot, test } from "./support.ts";

const BOARD = BOARD_ROOM_ID;

function pane(page: Page) {
  return page.locator("aside.right-pane");
}

function rules(page: Page) {
  return pane(page).getByRole("list", { name: "Auto-assign by tag" });
}

matrix("a board's automations open from its toolbar", async ({ page, theme, phone }) => {
  await openApp(page, `r/${BOARD}`, theme);
  await page.getByRole("button", { name: "Automations" }).click();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}/automations`));

  const automations = pane(page);

  await expect(automations.getByRole("heading", { name: "Automations" })).toBeVisible();
  await expect(rules(page).getByRole("listitem")).toHaveCount(2);
  await expect(rules(page).getByText("Ember")).toBeVisible();
  await expect(automations.getByRole("spinbutton", { name: "Planned nudge minutes" })).toHaveValue(
    "1440",
  );
  await expect(automations.getByText("1 d", { exact: true })).toBeVisible();
  await expect(
    automations.getByRole("spinbutton", { name: "Blocked nudge minutes" }),
  ).toHaveValue("");
  await expect(automations.getByRole("button", { name: "Save SLA timers" })).toBeDisabled();
  await page.mouse.move(0, 0);
  await shot(page, phone ? "board-automations-phone" : "board-automations", theme);
});

test("adding and removing an auto-assign rule", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}/automations`);

  const form = pane(page).getByRole("form", { name: "Add an auto-assign rule" });

  await form.getByRole("button", { name: "Add rule" }).click();
  await expect(form.getByText(/can't be blank/)).toBeVisible();
  await expect(form.getByText("must exist")).toBeVisible();

  await form.getByLabel("Tag").fill("Ops");
  await form.getByLabel("Assign to").selectOption({ label: "Maya Okafor" });
  await form.getByRole("button", { name: "Add rule" }).click();
  await expect(page.getByText("Auto-assign rule added.")).toBeVisible();
  await expect(rules(page).getByRole("listitem")).toHaveCount(3);
  await expect(form.getByLabel("Tag")).toHaveValue("");

  await rules(page).getByRole("button", { name: "Remove auto-assign rule for ops" }).click();
  await expect(page.getByText("Auto-assign rule removed.")).toBeVisible();
  await expect(rules(page).getByRole("listitem")).toHaveCount(2);
});

test("SLA timers save together and show the classic alert", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}/automations`);

  const timers = pane(page).getByRole("form", { name: "SLA timers" });
  const escalate = timers.getByRole("spinbutton", { name: "Planned escalation minutes" });

  await escalate.fill("60");
  await timers.getByRole("button", { name: "Save SLA timers" }).click();
  await expect(timers.getByRole("alert")).toHaveText(
    "Planned: Escalate after minutes must be after the nudge threshold",
  );
  await expect(escalate).toHaveAttribute("aria-invalid", "true");

  await escalate.fill("2880");
  await timers.getByRole("spinbutton", { name: "Blocked nudge minutes" }).fill("90");
  await timers.getByRole("spinbutton", { name: "Blocked escalation minutes" }).fill("180");
  await timers.getByRole("button", { name: "Save SLA timers" }).click();
  await expect(page.getByText("SLA timers saved.")).toBeVisible();
  await expect(timers.getByRole("alert")).toHaveCount(0);
  await expect(timers.getByRole("button", { name: "Save SLA timers" })).toBeDisabled();

  await page.reload();
  await expect(
    pane(page).getByRole("spinbutton", { name: "Blocked nudge minutes" }),
  ).toHaveValue("90");
});

test("Automations closes from its toolbar button", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}/automations`);

  const button = page.getByRole("button", { name: "Automations" });

  await expect(button).toHaveAttribute("aria-pressed", "true");
  await button.click();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}$`));
  await expect(button).toHaveAttribute("aria-pressed", "false");
});
