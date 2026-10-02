// The server and browser must both run at 2025-11-01T16:00:00Z for this source case.
import assert from "node:assert/strict"
import { runSuite, expect } from "./runtime.mjs"
import { openSave } from "./pins.mjs"

await runSuite("pins/saves DST", ({ room, labels }) => [["tomorrow at 9am resolves across the DST fall-back", async (page, { db }) => {
  assert(db, "--database required to observe the persisted reminder")
  assert.equal(await page.evaluate(() => new Date().toISOString()), "2025-11-01T16:00:00.000Z")
  await openSave(page, room, labels["messages.third"])
  const dialog = page.locator(".message-save-dialog[open]")
  await dialog.getByLabel("Tomorrow at 9am (your time)", { exact: true }).check()
  await dialog.getByRole("button", { name: "Save", exact: true }).click()
  await expect(page.locator(".message-save-dialog[open]")).toHaveCount(0)
  const row = db.prepare("SELECT remind_at FROM saved_items WHERE user_id=? AND message_id=?")
    .get(labels["users.david"], labels["messages.third"])
  assert(row, "saved item must persist before its dialog closes")
  assert.equal(new Date(`${row.remind_at.replace(" ", "T")}Z`).toISOString(), "2025-11-02T14:00:00.000Z")
}]], { user: "david", timezone: "America/New_York" })
