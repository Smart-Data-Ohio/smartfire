// Six test/system/pins_saved_test.rb cases; the seventh runs in pins-dst.mjs.
import assert from "node:assert/strict"
import { runSuite, visible, expect, waitForRoom, waitForComposer } from "./runtime.mjs"
import { createMessage } from "./messages.mjs"
import { DEFAULT_ORIGIN } from "../capture/proxy.ts"

export async function showMessage(page, room, id) {
  await page.goto(new URL(`/rooms/${room}/@${id}`, DEFAULT_ORIGIN).href)
  await waitForRoom(page)
  await waitForComposer(page)
  await visible(page.locator(`.message[data-message-id="${id}"]`))
}
export async function openMenu(page, id) {
  await page.locator(`.message[data-message-id="${id}"] [data-reply-target='body']`).first().click({ button: "right" })
  await visible(page.locator("[data-message-actions-target='menu']"), 10_000)
}
export async function openSave(page, room, id) {
  await showMessage(page, room, id)
  await openMenu(page, id)
  await page.getByRole("menuitem", { name: "Save for later", exact: true }).click()
  await visible(page.locator(".message-save-dialog[open]"))
}
async function pinFixture(page, id, method = "POST") {
  const status = await page.evaluate(async ({ id, method }) => {
    const response = await fetch(`/messages/${id}/pin`, { method, headers: { Accept: "application/json",
      "X-CSRF-Token": document.querySelector("meta[name='csrf-token']").content } })
    return response.status
  }, { id, method })
  assert.equal(status, method === "POST" ? 201 : 200, "pin fixture HTTP status")
}

export const cases = ({ room, labels }) => {
  let third = labels["messages.third"]
  const second = labels["messages.second"]
  const created = []
  const thirdText = "Third time's a charm."
  const thirdMessage = page => page.locator(`.message[data-message-id="${third}"]`)
  const note = page => page.locator(".message--system-note").filter({ has: page.locator(`a[href$='/@${third}']`) }).last()
  return [
    ["pinning from the menu badges the message and fills the panel, live for others", async (page, { withUser }) => {
      await pinFixture(page, third, "DELETE")
      await page.reload()
      await waitForRoom(page)
      await withUser("kevin", room, async other => {
        await openMenu(page, third)
        await page.getByRole("menuitem", { name: "Pin message", exact: true }).click()
        await visible(thirdMessage(page).locator(".message__pin-badge").filter({ hasText: "Pinned" }))
        await expect(page.locator(".room-header__pins-count")).toHaveText("1")
        await visible(note(page).filter({ hasText: "pinned a message" }))
        await visible(thirdMessage(other).locator(".message__pin-badge").filter({ hasText: "Pinned" }), 15_000)
      })
      await page.getByLabel("Show pinned messages", { exact: true }).click()
      const panel = page.locator(".pins-panel")
      await expect(panel).toContainText(thirdText)
      await expect(panel).toContainText("Pinned by")
      await panel.getByRole("link", { name: "Jump to message", exact: true }).click()
      await expect(page).toHaveURL(new RegExp(`/rooms/${room}/@${third}$`), { timeout: 10_000 })
      await expect(thirdMessage(page)).toContainText(thirdText)
    }],
    ["the pin note's jump link stays in this tab", async page => {
      await pinFixture(page, third)
      await page.reload()
      await waitForRoom(page)
      const link = note(page).getByRole("link", { name: "jump to message", exact: true })
      await expect(link).not.toHaveAttribute("target", "_blank", { timeout: 10_000 })
      await link.click()
      assert.equal(page.context().pages().length, 1, "jump must stay in this tab")
      await expect(page).toHaveURL(new RegExp(`/rooms/${room}/@${third}$`), { timeout: 10_000 })
    }],
    ["pin notes render as compact notes with no message menu", async page => {
      await pinFixture(page, third)
      await page.reload()
      await waitForRoom(page)
      const item = note(page)
      await visible(item.filter({ hasText: "David" }), 10_000)
      await expect(item).toHaveAttribute("role", "note")
      await expect(item).toContainText("pinned a message")
      await visible(item.locator(".message__system-note-icon"))
      await expect(item.locator(".message__avatar")).toHaveCount(0)
      await expect(item.locator(".message__toolbar")).toHaveCount(0)
      await item.locator(".message__system-note-author").click({ button: "right" })
      await expect(page.locator("[data-message-actions-target='menu']")).not.toBeVisible()
    }],
    ["unpinning from the panel clears the badge everywhere", async page => {
      await pinFixture(page, third)
      await page.reload()
      await waitForRoom(page)
      await visible(thirdMessage(page).locator(".message__pin-badge"))
      await page.getByLabel("Show pinned messages", { exact: true }).click()
      const panel = page.locator(".pins-panel")
      await expect(panel).toContainText(thirdText)
      await panel.getByRole("button", { name: "Unpin", exact: true }).click()
      await expect(panel).toContainText("No pinned messages yet")
      await panel.locator(".pins-panel__close").click()
      await expect(thirdMessage(page).locator(".message__pin-badge")).not.toBeVisible()
      await expect(page.locator(".room-header__pins-count")).toHaveText("0")
    }],
    ["saving with a reminder lists the message in Saved until done or removed", async page => {
      await openSave(page, room, third)
      const dialog = page.locator(".message-save-dialog[open]")
      await expect(dialog).toContainText(thirdText)
      await dialog.getByLabel("In 1 hour", { exact: true }).check()
      await dialog.getByRole("button", { name: "Save", exact: true }).click()
      await expect(page.locator(".message-save-dialog[open]")).toHaveCount(0)
      await page.goto(new URL("/saved", DEFAULT_ORIGIN).href)
      await expect(page.locator("#saved-items-title")).toHaveText("Saved")
      await expect(page.locator("main")).toContainText(thirdText)
      await expect(page.locator("main")).toContainText("In progress")
      await expect(page.locator("main")).toContainText("Reminds")
      await page.getByRole("button", { name: "Mark done", exact: true }).click()
      await expect(page.locator(".saved-item__kind")).toHaveText("Done")
      await page.getByRole("navigation", { name: "Saved filters" }).getByRole("link", { name: "In progress", exact: true }).click()
      await expect(page.locator("main")).toContainText("Nothing saved here yet.")
      await page.getByRole("navigation", { name: "Saved filters" }).getByRole("link", { name: "Done", exact: true }).click()
      await expect(page.locator("main")).toContainText(thirdText)
      await page.getByRole("button", { name: "Remove", exact: true }).click()
      await expect(page.locator("main")).toContainText("Nothing saved here yet.")
    }],
    ["saving with a custom reminder time", async page => {
      await openSave(page, room, second)
      const dialog = page.locator(".message-save-dialog[open]")
      await dialog.getByLabel("Custom time", { exact: true }).check()
      await page.locator("#reminder_custom_at").evaluate(element => { element.value = "2030-06-01T09:00" })
      await dialog.getByRole("button", { name: "Save", exact: true }).click()
      await expect(page.locator(".message-save-dialog[open]")).toHaveCount(0)
      await page.goto(new URL("/saved", DEFAULT_ORIGIN).href)
      await expect(page.locator("main")).toContainText("Reminds")
    }],
  ].map(([name, run], index) => [name, async (page, helpers) => {
    if (index < 4) {
      // The source transaction has only a few messages. Give each pin case a
      // fresh JZ-authored fixture in the latest window of this populated seed,
      // so its live note is appended to that window rather than an old anchor.
      for (const id of created) await pinFixture(page, id, "DELETE")
      third = await helpers.withUser("jz", room, other => createMessage(other, room,
        { markdown_source: thirdText }))
      created.push(third)
      await page.reload()
      await waitForRoom(page)
    }
    await run(page, helpers)
  }])
}
// Also imported by the DST case for its shared menu helpers.
if (process.argv[1]?.endsWith("/pins.mjs")) await runSuite("pins/saves", cases, { user: "david" })
