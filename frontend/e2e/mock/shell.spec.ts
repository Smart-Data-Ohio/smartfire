import { expect, openApp, ROOM_IDS, test } from "./support.ts";

test("opens #general from the mock workspace", async ({ page }) => {
  await openApp(page, `r/${ROOM_IDS.general}`);

  await expect(page.getByRole("heading", { name: "general" })).toBeVisible();
  await expect(page.getByRole("log", { name: "Messages" })).toBeVisible();
});
