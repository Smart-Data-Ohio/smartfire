import assert from 'node:assert/strict'

// SystemTestHelper#fill_in_thread_name in the pinned Rails test helper: its
// beginCreate animation frame focuses First message before Thread name is typed.
export async function fillThreadName(page, name) {
  await page.waitForFunction(() => document.activeElement?.matches("[data-thread-panel-target='createMessage']"), null, {timeout:10000})
  const title = page.locator("#thread-panel [data-thread-panel-target='createName']")
  await title.fill(name)
  assert.equal(await title.inputValue(), name, 'system_test_helper.rb:115: exact thread name')
}
