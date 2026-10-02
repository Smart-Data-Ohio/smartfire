import { visible, expect } from "./runtime.mjs"

export async function searchUploads(page, query, filename) {
  await page.getByLabel("Search by filename", { exact: true }).fill(query)
  await page.getByRole("button", { name: "Search", exact: true }).click()
  // The upload was already visible before submission. Wait for the new
  // response's filter link so the next click cannot use the old empty query.
  const parameter = new URLSearchParams({ filename: query }).toString()
    .replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
  await expect(page.getByRole("link", { name: "Images", exact: true }))
    .toHaveAttribute("href", new RegExp(`[?&]${parameter}(?:&|$)`), { timeout: 10_000 })
  await visible(page.locator(".room-files__name").filter({ hasText: filename }), 10_000)
}
