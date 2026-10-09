import type { Page } from "@playwright/test";
import { DESKTOP, expect, expectTouchTargets, PHONE_TOUCH, ROOM_IDS, test } from "./support.ts";

async function openDrive(page: Page) {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM_IDS.general}`);
  await page.getByRole("log", { name: "Messages" }).waitFor();
  await page.getByRole("button", { name: "Attach and more" }).click();
  await page.getByRole("menuitem", { name: "From Google Drive" }).click();

  return page.getByRole("combobox", { name: "Search Drive files" });
}

test.describe("phone", () => {
  test.use(PHONE_TOUCH);

  test("search attaches a Drive file, and an edit can remove it", async ({ page }) => {
    const search = await openDrive(page);

    await search.fill("roadmap");
    await expect(page.getByRole("option", { name: /Q4 roadmap/ })).toBeVisible();
    await expectTouchTargets(page, ".drive-picker");
    await expect(page.getByRole("button", { name: "Close Drive search" })).toBeVisible();
    await search.press("ArrowDown");
    await search.press("Enter");

    await expect(page.getByRole("dialog", { name: "Share a Drive file" })).toBeVisible();
    await page.getByRole("button", { name: "Attach only" }).click();
    await expect(page.getByRole("button", { name: "Remove Q4 roadmap" })).toBeVisible();

    const input = page.getByRole("textbox", { name: "Message #general" });

    await input.click();
    await input.pressSequentially("see the roadmap");
    await expect(input).toHaveValue("see the roadmap");
    await page.getByRole("button", { name: "Send message" }).tap();

    const row = page.locator("[data-message-row]", { hasText: "see the roadmap" });

    await expect(row).toHaveAttribute("data-message-id", /^\d+$/);
    await expect(row.getByRole("link", { name: /Google Drive file/ })).toBeVisible();

    await row.locator(".message-body").click({ button: "right" });
    await page.getByRole("menuitem", { name: "Edit message" }).click();
    await page.getByRole("button", { name: "Remove Google Drive file" }).click();
    await page.getByRole("textbox", { name: "Edit message" }).press("Enter");
    await expect(row.getByRole("link", { name: /Google Drive file/ })).toHaveCount(0);
  });

  test("a missing Drive grant shows the disconnected state", async ({ page }) => {
    await page.route("**/api/v1/drive/files**", (route) =>
      route.fulfill({ status: 404, body: "" }),
    );
    const search = await openDrive(page);

    await search.fill("roadmap");
    await expect(page.getByText("Google Drive isn't connected.")).toBeVisible();
    await expect(page.getByRole("button", { name: "Enable Drive previews" })).toBeVisible();
    await expect(page.getByRole("option")).toHaveCount(0);
  });

  test("an older search cannot replace a newer one", async ({ page }) => {
    let releaseOlder = () => {};

    const olderHeld = new Promise<void>((resolve) => {
      releaseOlder = resolve;
    });

    let olderStarted = false;

    await page.route("**/api/v1/drive/files**", async (route) => {
      const query = new URL(route.request().url()).searchParams.get("q");

      if (query === "z") {
        olderStarted = true;
        await olderHeld;
      }

      await route.continue();
    });

    const search = await openDrive(page);

    await search.fill("z");
    await expect.poll(() => olderStarted).toBe(true);
    await search.fill("roadmap");
    await expect(page.getByRole("option", { name: /Q4 roadmap/ })).toBeVisible();

    const older = page.waitForResponse((response) => response.url().includes("q=z"));

    releaseOlder();
    await older;
    await expect(page.getByRole("option", { name: /Q4 roadmap/ })).toBeVisible();
    await expect(page.getByRole("option")).toHaveCount(1);
  });
});

test.describe("desktop", () => {
  test.use({ viewport: DESKTOP });

  test("search attaches a Drive file", async ({ page }) => {
    const search = await openDrive(page);

    await expect(page.getByRole("button", { name: "Close Drive search" })).toHaveCount(0);
    await search.fill("roadmap");
    await expect(page.getByRole("option", { name: /Q4 roadmap/ })).toBeVisible();
    await search.press("ArrowDown");
    await search.press("Enter");
    await page.getByRole("button", { name: "Attach only" }).click();
    await expect(page.getByRole("button", { name: "Remove Q4 roadmap" })).toBeVisible();
  });
});
