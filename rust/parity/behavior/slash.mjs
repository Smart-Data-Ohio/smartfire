// All 26 test/system/slash_commands_test.rb cases; agent setup uses validated Rails fixtures.
import assert from "node:assert/strict"
import { registerLiveCommand } from "./fixture-client.mjs"
import { runSuite, visible, expect } from "./runtime.mjs"

const editor = page => page.getByLabel("Write a message", { exact: true })
const option = (page, text) => page.locator("suggestion-option").filter({ hasText: text })
const feedback = page => page.locator("#composer .composer__feedback")
const emptyEditor = page => expect(editor(page)).toHaveValue("")
async function selectByEnter(page, name) {
  await editor(page).fill(name)
  await visible(option(page, name))
  await editor(page).press("Enter")
}
async function submit(page, text) {
  await editor(page).fill(text)
  await page.getByRole("button", { name: "Send Message", exact: true }).click()
}
async function withNewMessage(page, send, text, selector = ".message[data-message-id]") {
  // Unlike Rails' per-test transaction, a live server retains earlier cases' posts.
  // Exclude existing persisted IDs, so a repeated assertion cannot pass on an old post.
  const oldIds = await page.locator(".message[data-message-id]").evaluateAll(nodes => nodes.map(n => n.id))
  const excluding = oldIds.map(id => `:not([id=${JSON.stringify(id)}])`).join("")
  await send()
  await visible(page.locator(`${selector}${excluding} .message__body`).filter({ hasText: text }).first(), 10_000)
  await emptyEditor(page)
}
async function recordFetches(page) {
  await page.evaluate(() => {
    window.__slashCommandFetchUrls = []
    const originalFetch = window.fetch
    window.fetch = (input, init) => {
      const url = typeof input === "string" ? input : input.url
      if (url.includes("/autocompletable/slash_commands")) window.__slashCommandFetchUrls.push(url)
      return originalFetch(input, init)
    }
  })
}

const cases = ({ labels }) => [
  ["typing slash opens the command picker with combobox semantics", async page => {
    await editor(page).fill("/")
    await visible(option(page, "/poll"))
    await visible(option(page, "/huddle"))
    await expect(editor(page)).toHaveAttribute("aria-autocomplete", "list")
    await editor(page).fill("/shr")
    await visible(option(page, "/shrug"))
    await expect(option(page, "/huddle")).toHaveCount(0)
    await editor(page).press("Enter")
    await expect(editor(page)).toHaveValue("/shrug ")
  }],
  ["picking poll by Enter runs it immediately", async page => {
    await selectByEnter(page, "/poll")
    await visible(page.locator("#poll-builder[open]"))
    await emptyEditor(page)
  }],
  ["picking poll by click runs it immediately", async page => {
    await editor(page).fill("/poll")
    await option(page, "/poll").click()
    await visible(page.locator("#poll-builder[open]"))
    await emptyEditor(page)
  }],
  ["picking event by Enter opens the form immediately", async page => {
    await selectByEnter(page, "/event")
    await visible(page.locator("h1").filter({ hasText: "Schedule an event" }))
  }],
  ["picking huddle by Enter runs it immediately", async page => {
    await selectByEnter(page, "/huddle")
    await visible(feedback(page).filter({ hasText: "not configured" }))
  }],
  ["suggestion rows hint argument placeholders only", async page => {
    await editor(page).fill("/poll")
    await visible(option(page, "/poll"))
    await expect(page.locator(".slash-command__hint").filter({ hasText: "runs now" })).toHaveCount(0)
    await editor(page).fill("/remind")
    await visible(option(page, "<when> <text>"))
  }],
  ["the close button dismisses the picker without sending", async page => {
    await editor(page).fill("/po")
    await visible(option(page, "/poll"))
    await page.getByLabel("Close suggestions", { exact: true }).click()
    await expect(page.locator("suggestion-option")).toHaveCount(0)
    await expect(editor(page)).toHaveValue("/po")
    await expect(page.locator("#poll-builder[open]")).toHaveCount(0)
  }],
  ["the close button does not overlap the first row's text", async page => {
    await editor(page).fill("/")
    await visible(page.locator("suggestion-option").first())
    const overlap = await page.evaluate(() => {
      const close = document.querySelector(".suggestion__close").getBoundingClientRect()
      const row = document.querySelector("suggestion-option .autocomplete__btn")
      const style = getComputedStyle(row)
      return row.getBoundingClientRect().right - parseFloat(style.paddingRight) > close.left
    })
    assert.equal(overlap, false, "row text runs under the close button")
  }],
  ["Escape closes the picker without sending", async page => {
    await editor(page).fill("/po")
    await visible(option(page, "/poll"))
    await editor(page).press("Escape")
    await expect(page.locator("suggestion-option")).toHaveCount(0)
    await expect(editor(page)).toHaveValue("/po")
    await expect(page.locator("#poll-builder[open]")).toHaveCount(0)
  }],
  ["mentions and emoji pickers have no close button and still commit", async page => {
    await editor(page).fill(":open")
    await visible(option(page, "OpenAI"))
    await expect(page.locator(".suggestion__close")).toHaveCount(0)
    await editor(page).press("Enter")
    await expect(editor(page)).toHaveValue(":openai: ")
    await editor(page).fill("@kev")
    await visible(option(page, "Kevin"))
    await expect(page.locator(".suggestion__close")).toHaveCount(0)
    await editor(page).press("Enter")
    await expect(editor(page)).toHaveValue("@[Kevin] ")
  }],
  ["the picker lists registered agent commands", async page => {
    await editor(page).fill("/dep")
    await visible(option(page, "/deploy"))
    await expect(option(page, "/deploy")).toContainText("Bender Bot")
  }],
  ["agent commands that take arguments insert and wait", async (page, { db }) => {
    assert(db, "--database required to observe agent events")
    const count = () => db.prepare("SELECT COUNT(*) AS n FROM agent_events WHERE agent_id=?").get(labels["agents.bender"]).n
    const before = count()
    await selectByEnter(page, "/dep")
    await expect(editor(page)).toHaveValue("/deploy ")
    assert.equal(count(), before, "picking an argument command must create no agent event")
  }],
  ["agent commands without arguments run immediately when picked", async page => {
    await selectByEnter(page, "/ship")
    await visible(feedback(page).filter({ hasText: "Sent to Bender Bot" }))
    await emptyEditor(page)
  }],
  ["shrug posts through the picker", async page => {
    await selectByEnter(page, "/shrug")
    await expect(editor(page)).toHaveValue("/shrug ")
    await withNewMessage(page, async () => {
      await editor(page).pressSequentially("ship it")
      await editor(page).press("Enter")
    }, "ship it")
  }],
  ["arguments close the slash picker and Enter posts", async page => {
    await recordFetches(page)
    await editor(page).fill("/shrug")
    await visible(option(page, "/shrug"))
    await withNewMessage(page, async () => {
      await editor(page).fill("/shrug ship it")
      await page.waitForFunction(() => window.__slashCommandFetchUrls.some(url => url.includes("ship"))
        || document.querySelectorAll("suggestion-select").length === 0)
      await expect(page.locator("suggestion-option")).toHaveCount(0)
      await editor(page).press("Enter")
    }, "ship it")
  }],
  ["Enter submits while the picker's deactivating update is still pending", async page => {
    await editor(page).fill("/shrug")
    await visible(option(page, "/shrug"))
    await withNewMessage(page, async () => {
      await editor(page).fill("/shrug ship it")
      await editor(page).press("Enter")
    }, "ship it")
  }],
  ["a submit queued during the live command check still runs the command", async page => {
    // The source intentionally delays this check by 2 seconds. This is its interleaving,
    // not a wait threshold or retry; completion must submit the new draft.
    await page.evaluate(() => {
      window.__slashCommandFetchUrls = []
      const originalFetch = window.fetch
      window.fetch = (input, init) => {
        const url = typeof input === "string" ? input : input.url
        if (url.includes("/autocompletable/slash_commands")) {
          window.__slashCommandFetchUrls.push(url)
          return new Promise(resolve => setTimeout(() => resolve(originalFetch(input, init)), 2000))
        }
        return originalFetch(input, init)
      }
    })
    await withNewMessage(page, async () => {
      await submit(page, "/etc/hosts is not a command")
      await page.waitForFunction(() => window.__slashCommandFetchUrls.some(url => !url.includes("query=")))
      await editor(page).fill("/shrug late switch")
      await editor(page).press("Enter")
    }, "(ツ)")
    await expect(page.locator(".message__body").filter({ hasText: "/shrug late switch" })).toHaveCount(0)
  }],
  ["unknown slash words post as normal messages", async page => {
    await withNewMessage(page, () => submit(page, "/etc/hosts is not a command"), "/etc/hosts is not a command")
  }],
  ["double slash escapes a known command", async page => {
    await withNewMessage(page, () => submit(page, "//poll takes no vote"), "/poll takes no vote")
  }],
  ["a command registered after page load still runs", async (page, { db }) => {
    assert(db, "--database required to observe registration and persisted invocation")
    assert.equal(db.prepare("SELECT COUNT(*) AS n FROM agent_slash_commands WHERE room_id=? AND name='later'").get(labels["rooms.designers"]).n, 0)
    await editor(page).fill("/later staging")
    await registerLiveCommand()
    await page.getByRole("button", { name: "Send Message", exact: true }).click()
    await visible(feedback(page).filter({ hasText: "Sent to Bender Bot" }))
    await emptyEditor(page)
    await expect(page.locator(".message__body").filter({ hasText: "later staging" })).toHaveCount(0)
    const event = db.prepare("SELECT metadata FROM agent_events WHERE agent_id=? AND event_type='slash_command' ORDER BY id DESC LIMIT 1").get(labels["agents.bender"])
    assert.equal(JSON.parse(event.metadata).arguments, "staging")
  }],
  ["me renders as an action line", async page => {
    await withNewMessage(page, () => submit(page, "/me is reviewing the deploy"),
      "is reviewing the deploy", ".message--action[data-message-id]")
  }],
  ["poll opens the poll builder", async page => {
    await submit(page, "/poll")
    await visible(page.locator("#poll-builder[open]"))
    await emptyEditor(page)
  }],
  ["event navigates to the prefilled form", async page => {
    await submit(page, "/event Launch party")
    await visible(page.locator("h1").filter({ hasText: "Schedule an event" }))
    await expect(page.getByLabel("Title", { exact: true })).toHaveValue("Launch party")
  }],
  ["agent commands respond ephemerally until the agent replies", async (page, { db }) => {
    assert(db, "--database required to observe persisted command arguments")
    const last = () => db.prepare("SELECT id, metadata FROM agent_events WHERE agent_id=? AND event_type='slash_command' ORDER BY id DESC LIMIT 1").get(labels["agents.bender"])
    const before = last()?.id ?? 0
    await submit(page, "/deploy staging")
    await visible(feedback(page).filter({ hasText: "Sent to Bender Bot" }))
    await emptyEditor(page)
    await expect(page.locator(".message__body").filter({ hasText: "deploy staging" })).toHaveCount(0)
    const event = last()
    assert(event && event.id > before, "submission must persist a new agent event")
    assert.equal(JSON.parse(event.metadata).arguments, "staging")
  }],
  ["status sets the custom status", async (page, { db }) => {
    assert(db, "--database required to observe persisted custom status")
    await submit(page, "/status 🚂 On a train")
    await visible(feedback(page).filter({ hasText: "Status set" }))
    assert.equal(db.prepare("SELECT custom_status_emoji FROM users WHERE id=?").get(labels["users.jz"]).custom_status_emoji, "🚂")
  }],
  ["huddle reports when unconfigured", async page => {
    await submit(page, "/huddle")
    await visible(feedback(page).filter({ hasText: "not configured" }))
  }],
]

await runSuite("slash", cases)
