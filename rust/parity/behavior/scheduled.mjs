// test/system/scheduled_messages_test.rb, exercised against either real HTTP server.
import assert from "node:assert/strict"
import { runSuite, visible, notice, scheduled } from "./runtime.mjs"
import { DEFAULT_ORIGIN } from "../capture/proxy.ts"

const cases = ({ room }) => [
  ["schedules from the composer and lists in the Scheduled view", async page => {
    await page.evaluate(() => {
      window.__scheduledCreates = []
      const originalFetch = window.fetch
      window.fetch = async (input, init) => {
        const response = await originalFetch(input, init)
        const url = typeof input === "string" ? input : input.url
        if (init?.method === "POST" && url.endsWith("/scheduled_messages")) {
          response.clone().json().then(body => window.__scheduledCreates.push({ status: response.status, body }))
        }
        return response
      }
    })
    await page.getByLabel("Write a message", { exact: true }).fill("Morning, team!")
    await page.getByRole("button", { name: "Schedule send", exact: true }).click()
    await visible(page.locator("#schedule-send[open]"))
    await page.locator("#schedule-send").getByRole("button", { name: "In 1 hour", exact: true }).click()
    await visible(page.locator("#composer .composer__feedback").filter({ hasText: "Scheduled for" }))
    await page.waitForFunction(() => window.__scheduledCreates.length === 1)
    const created = await page.evaluate(() => window.__scheduledCreates[0])
    assert.equal(created.status, 201)
    const { id } = created.body
    assert.equal(await page.getByLabel("Write a message", { exact: true }).inputValue(), "")
    await scheduled(page)
    await visible(page.locator("h1").filter({ hasText: "Scheduled" }))
    assert.equal(await page.locator(`#scheduled_message_${id} textarea`).inputValue(), "Morning, team!")
  }],
  ["schedule send requires a draft", async page => {
    await page.getByRole("button", { name: "Schedule send", exact: true }).click()
    await page.locator("#schedule-send").getByRole("button", { name: "Tomorrow at 9 AM", exact: true }).click()
    await visible(page.locator(".schedule-send__status").filter({ hasText: "Write a message first." }))
  }],
  ["edits, sends now, and cancels from the Scheduled view", async page => {
    // The source case creates these fixtures with ScheduledMessage.create!; use the public
    // create endpoint as setup so both backends receive identical valid persisted inputs.
    const ids = await page.evaluate(async room => {
      const ids = []
      for (const markdown_source of ["Draft one", "Draft two", "Draft three"]) {
        const response = await fetch(`/rooms/${room}/scheduled_messages`, {
          method: "POST", headers: { "Content-Type": "application/json", Accept: "application/json",
            "X-CSRF-Token": document.querySelector("meta[name='csrf-token']").content },
          body: JSON.stringify({ scheduled_message: { markdown_source, send_at: "2026-03-02T18:00:00Z" } }),
        })
        if (response.status !== 201) throw new Error(`fixture create: HTTP ${response.status}`)
        ids.push((await response.json()).id)
      }
      return ids
    }, room)
    await scheduled(page)
    const row = id => page.locator(`#scheduled_message_${id}`)
    // Rails repeats field ids across rows; Capybara scopes its label lookup to the row.
    await row(ids[0]).locator("textarea[name='scheduled_message[markdown_source]']").fill("Draft one, edited")
    await row(ids[0]).getByRole("button", { name: "Save", exact: true }).click()
    await notice(page, "Scheduled message updated.")
    await scheduled(page)
    assert.equal(await row(ids[0]).locator("textarea[name='scheduled_message[markdown_source]']").inputValue(), "Draft one, edited")
    await row(ids[1]).getByRole("button", { name: "Send now", exact: true }).click()
    await notice(page, "Message sent.")
    await page.goto(new URL(`/rooms/${room}`, DEFAULT_ORIGIN).href)
    await visible(page.locator(".message").filter({ hasText: "Draft two" }).first(), 10_000)
    await scheduled(page)
    await row(ids[2]).getByRole("button", { name: "Cancel", exact: true }).click()
    await notice(page, "Scheduled message cancelled.")
    await scheduled(page)
    assert.equal(await row(ids[2]).count(), 0)
  }],
  ["the sidebar links the Scheduled view", async page => {
    await scheduled(page)
    await page.locator(".sidebar").getByRole("link", { name: "Scheduled", exact: true }).click()
    await visible(page.locator("h1").filter({ hasText: "Scheduled" }))
  }],
]

await runSuite("scheduled", cases)
