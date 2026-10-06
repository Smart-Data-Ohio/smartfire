// All four test/system/search_files_test.rb cases, on the actual app/browser.
import { readFile } from "node:fs/promises"
import { runSuite, visible, expect } from "./runtime.mjs"
import { createMessage } from "./messages.mjs"
import { searchUploads } from "./file-search.mjs"
import { DEFAULT_ORIGIN } from "../capture/proxy.ts"

const cases = ({ room, labels }) => [
  ["filter chips show parsed operators and remove them", async page => {
    await createMessage(page, room, { markdown_source: "chiptune launch notes" })
    await page.goto(new URL("/searches?q=from%3A%40jz%20has%3Afile%20chiptune", DEFAULT_ORIGIN).href)
    await expect(page.locator(".search-filter-chip")).toHaveCount(2, { timeout: 10_000 })
    await visible(page.locator(".search-filter-chip").filter({ hasText: "from: jz" }))
    await expect(page.locator(".message-area--empty")).not.toBeVisible()
    await page.getByRole("link", { name: "Remove has: file filter", exact: true }).click()
    await expect(page.locator(".search-filter-chip")).toHaveCount(1, { timeout: 10_000 })
    await visible(page.locator(".search-filter-chip").filter({ hasText: "from: jz" }))
    await visible(page.locator(".message__body").filter({ hasText: "chiptune launch notes" }).first(), 10_000)
  }],
  ["a pasted permalink renders a quote card with a working jump link", async (page, { withUser }) => {
    const source = await withUser("david", room, other => createMessage(other, room, { markdown_source: "quoted system words here" }))
    const quoting = await createMessage(page, room, { markdown_source: `see this /rooms/${room}/@${source}` })
    await page.reload()
    const card = page.locator(`.message[data-message-id="${quoting}"] blockquote.message-quote`)
    await visible(card.filter({ hasText: "quoted system words here" }), 10_000)
    await expect(card.locator(".message-quote__author")).toContainText("David")
    await card.getByRole("link", { name: "Jump to message", exact: true }).click()
    await expect(page).toHaveURL(new RegExp(`/rooms/${room}/@${source}$`), { timeout: 10_000 })
  }],
  ["a cross-room permalink loads its quote frame for members and outsiders", async (page, { withUser }) => {
    const sourceRoom = labels["rooms.watercooler"]
    const source = await withUser("jason", sourceRoom, other => createMessage(other, sourceRoom,
      { markdown_source: "cross-room quoted system words" }))
    const quoting = await createMessage(page, room, { markdown_source: `see this /rooms/${sourceRoom}/@${source}` })
    await page.reload()
    const message = page.locator(`.message[data-message-id="${quoting}"]`)
    await visible(message.locator("turbo-frame.message-link-frame"), 10_000)
    await visible(message.locator(".message-quote-private").filter({ hasText: "Message in a private room" }), 10_000)
    await withUser("david", room, async other => {
      const card = other.locator(`.message[data-message-id="${quoting}"] blockquote.message-quote`)
      await visible(card.filter({ hasText: "cross-room quoted system words" }), 10_000)
      await expect(card.locator(".message-quote__author")).toContainText("Jason")
    })
  }],
  ["the Files tab lists uploads and Drive rows with working filters", async page => {
    const base64 = (await readFile(new URL("../../fixtures/files/earth.png", import.meta.url))).toString("base64")
    await createMessage(page, room, { body: "system file rows" }, { base64, name: "system-cover.png", type: "image/png" })
    await createMessage(page, room, { body: "system drive rows", drive_file_ids: ["1a2b3c4d5e6f7g8h9i0j"] })
    await page.reload()
    await page.getByRole("link", { name: "Show files", exact: true }).click()
    await visible(page.locator(".room-files__name").filter({ hasText: "system-cover.png" }), 10_000)
    await visible(page.locator(".room-files__drive-link").filter({ hasText: "Drive file" }).first())
    await searchUploads(page, "cover", "system-cover.png")
    await page.getByRole("link", { name: "Images", exact: true }).click()
    await expect(page.locator(".room-files__name")).toHaveCount(1, { timeout: 10_000 })
    await page.getByRole("link", { name: "Videos", exact: true }).click()
    await visible(page.getByText("No uploads match.", { exact: true }), 10_000)
  }],
]

await runSuite("search-files", cases)
