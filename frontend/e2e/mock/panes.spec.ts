import type { Page } from "@playwright/test";
import { onboardingMockupPng } from "../../mock/s2/assets.ts";
import { md5Base64 } from "../../mock/s2/digest.ts";
import { THREAD_IDS } from "../../mock/s2/seed.ts";
import type { RoomFile } from "../../src/gen/RoomFile.ts";
import {
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  matrix,
  openHeaderTool,
  PHONE_TOUCH,
  ROOM_IDS,
  shot,
  type Theme,
  test,
} from "./support.ts";

/**
 * Opens the app at `path` (under /app/) with motion reduced; unlike `openApp` it waits for the
 * conversation, since a phone in a room or thread hides the sidebar.
 */
async function open(page: Page, path: string, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/${path}`);
  await page.getByRole("main").waitFor();
}

const GENERAL = ROOM_IDS.general;

function pane(page: Page) {
  return page.locator("aside.right-pane");
}

matrix("the members pane", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}`, theme);
  await openHeaderTool(page, /^Members/);

  await expect(pane(page).getByRole("heading", { name: "Members" })).toBeVisible();
  await expect(pane(page).getByRole("heading", { name: /^Starred/ })).toBeVisible();
  await expect(pane(page).getByRole("heading", { name: /^Online/ })).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "members", theme);

  await pane(page).getByRole("searchbox", { name: "Find a member" }).fill("maya");
  await expect(pane(page).getByRole("button", { name: /^Message Maya/ })).toBeVisible();
  await shot(page, "members-search", theme);
});

test("starring a member moves them to Starred", async ({ page }) => {
  await open(page, `r/${GENERAL}`);
  await openHeaderTool(page, /^Members/);

  const starred = pane(page).getByRole("region", { name: /^Starred/ });

  await pane(page)
    .getByRole("button", { name: /^Star Jonah/ })
    .click();
  await expect(starred.getByRole("button", { name: /^Message Jonah/ })).toBeVisible();
  await pane(page)
    .getByRole("button", { name: /^Unstar Jonah/ })
    .click();
  await expect(starred.getByRole("button", { name: /^Message Jonah/ })).toHaveCount(0);
});

test("choosing a member opens the direct message", async ({ page }) => {
  await open(page, `r/${GENERAL}`);
  await openHeaderTool(page, /^Members/);
  await pane(page)
    .getByRole("button", { name: /^Message Jonah/ })
    .click();

  await expect(page).not.toHaveURL(new RegExp(`/r/${GENERAL}$`));
  await expect(page.getByRole("textbox", { name: /Jonah/ })).toBeVisible();
});

matrix("the pins pane", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}`, theme);
  await openHeaderTool(page, /^Pinned messages/);

  await expect(pane(page).getByRole("heading", { name: "Pinned messages" })).toBeVisible();

  const cards = pane(page).locator(".pin-card");

  await expect(cards).toHaveCount(3);
  await page.mouse.move(0, 0);
  await shot(page, "pins", theme);

  await cards.first().hover();
  await cards.first().getByRole("button", { name: "Unpin" }).click();
  await expect(cards).toHaveCount(2);
});

matrix("the files pane", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}`, theme);
  await openHeaderTool(page, /^Files/);

  await expect(pane(page).getByRole("heading", { name: "Files" })).toBeVisible();
  await expect(
    pane(page).getByRole("list", { name: "Files" }).getByRole("listitem").first(),
  ).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "files", theme);

  await pane(page).getByRole("tab", { name: "Images" }).click();
  await expect(pane(page).locator(".file-grid .file-tile").first()).toBeVisible();
  await shot(page, "files-images", theme);

  await pane(page).getByRole("tab", { name: "All" }).click();
  await pane(page).getByRole("searchbox", { name: "Search files by name" }).fill("zzz-nothing");
  await expect(pane(page).getByText('No files match "zzz-nothing".')).toBeVisible();
});

test("the files pane keeps loaded pages and scroll position after a text-only edit", async ({
  page,
}) => {
  const entries = Array.from(
    { length: 60 },
    (_, index): RoomFile => ({
      messageId: 100_000 + index,
      threadId: null,
      creatorId: 1,
      createdAt: "2026-10-06T00:00:00.000Z",
      attachment: {
        filename: `file-${index}.txt`,
        contentType: "text/plain",
        byteSize: 100,
        width: null,
        height: null,
        preview: "file",
        spoiler: false,
        url: `/files/file-${index}.txt`,
        downloadUrl: `/files/file-${index}.txt?disposition=attachment`,
        thumbnailUrl: null,
      },
    }),
  );

  const requests: number[] = [];

  await page.route(`**/api/v1/rooms/${GENERAL}/files?*`, async (route) => {
    const current = Number(new URL(route.request().url()).searchParams.get("page") ?? 1);

    requests.push(current);
    await route.fulfill({
      json: {
        files: entries.slice((current - 1) * 30, current * 30),
        users: [],
        nextPage: current === 1 ? 2 : null,
      },
    });
  });
  await open(page, `r/${GENERAL}`);
  const composer = page.getByRole("textbox", { name: "Message #general" });

  await composer.fill("Text edit while reading files");
  await composer.press("Enter");

  const message = page.locator(".message[data-message-id]", {
    has: page.getByText("Text edit while reading files", { exact: true }),
  });

  await expect(message).toHaveCount(1);
  const id = await message.getAttribute("data-message-id");

  await openHeaderTool(page, /^Files/);
  const rows = pane(page).getByRole("list", { name: "Files" }).getByRole("listitem");

  await expect(rows).toHaveCount(30);
  await pane(page).getByRole("button", { name: "Load more" }).click();
  await expect(rows).toHaveCount(60);
  const loadedRequests = [...requests];
  const body = pane(page).locator(".pane-body");

  const scrollTop = await body.evaluate((element) => {
    element.scrollTop = 800;

    return element.scrollTop;
  });

  expect(scrollTop).toBeGreaterThan(0);
  const state = await (await page.request.get("/__mock/state")).json();

  const response = await page.request.patch(`/api/v1/messages/${id}`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { markdownSource: "Edited while reading files" },
  });

  expect(response.ok()).toBe(true);
  await expect(page.locator(`.message[data-message-id="${id}"]`)).toContainText(
    "Edited while reading files",
  );
  await expect(rows).toHaveCount(60);
  expect(await rows.locator(".file-row-name").allTextContents()).toEqual(
    entries.map((file) => file.attachment.filename),
  );
  expect(await body.evaluate((element) => element.scrollTop)).toBe(scrollTop);
  expect(requests).toEqual(loadedRequests);
});

test("the files pane shows an upload and drops it again when its message is deleted, live", async ({
  page,
}) => {
  const fileRequests: string[] = [];

  page.on("request", (request) => {
    if (request.url().includes(`/api/v1/rooms/${GENERAL}/files`)) {
      fileRequests.push(request.url());
    }
  });
  await open(page, `r/${GENERAL}`);
  await openHeaderTool(page, /^Files/);
  const rows = pane(page).getByRole("list", { name: "Files" }).getByRole("listitem");

  await expect(rows.first()).toBeVisible();
  const loaded = await rows.count();
  const loadedRequests = [...fileRequests];

  const composer = page.getByRole("textbox", { name: "Message #general" });

  await page.locator('.composer input[type="file"]').setInputFiles({
    name: "live-pane.png",
    mimeType: "image/png",
    buffer: Buffer.from(onboardingMockupPng()),
  });
  await expect(page.locator('.tray-chip[data-phase="done"]')).toHaveCount(1);
  await composer.fill("Live pane upload");
  await composer.press("Enter");

  const message = page.locator(".message[data-message-id]", {
    has: page.getByText("Live pane upload", { exact: true }),
  });

  await expect(message).toHaveCount(1);
  const id = await message.getAttribute("data-message-id");

  await expect(pane(page).getByText("live-pane.png")).toBeVisible();
  await expect(rows).toHaveCount(loaded + 1);
  expect(fileRequests).toEqual(loadedRequests);

  const state = await (await page.request.get("/__mock/state")).json();
  const response = await page.request.delete(`/api/v1/messages/${id}`, {
    headers: { "X-CSRF-Token": state.csrfToken },
  });

  expect(response.ok()).toBe(true);
  await expect(message).toHaveCount(0);
  await expect(pane(page).getByText("live-pane.png")).toHaveCount(0);
  await expect(rows).toHaveCount(loaded);
  expect(fileRequests).toEqual(loadedRequests);
});

test("the files pane shows a thread reply's upload live and drops it when the reply is deleted", async ({
  page,
}) => {
  await open(page, `r/${GENERAL}`);
  await openHeaderTool(page, /^Files/);
  const rows = pane(page).getByRole("list", { name: "Files" }).getByRole("listitem");

  await expect(rows.first()).toBeVisible();
  const loaded = await rows.count();

  const state = await (await page.request.get("/__mock/state")).json();
  const csrf = { "X-CSRF-Token": state.csrfToken };
  const bytes = onboardingMockupPng();

  const upload = await page.request.post("/api/v1/uploads", {
    headers: csrf,
    data: {
      filename: "thread-live.png",
      byteSize: bytes.length,
      checksum: md5Base64(bytes),
      contentType: "image/png",
    },
  });

  expect(upload.ok()).toBe(true);
  const { signedId, uploadUrl } = await upload.json();
  const stored = await page.request.put(uploadUrl, {
    headers: { "Content-Type": "image/png" },
    data: Buffer.from(bytes),
  });

  expect(stored.ok()).toBe(true);
  const posted = await page.request.post(`/api/v1/threads/${THREAD_IDS.generalActive}/messages`, {
    headers: csrf,
    data: {
      clientMessageId: "e2e-thread-live-file",
      markdownSource: "Thread live upload",
      replyToMessageId: null,
      replyNotifyAuthor: null,
      attachmentSignedId: signedId,
    },
  });

  expect(posted.ok()).toBe(true);
  const reply = await posted.json();

  await expect(pane(page).getByText("thread-live.png")).toBeVisible();
  await expect(rows).toHaveCount(loaded + 1);

  const removed = await page.request.delete(`/api/v1/messages/${reply.id}`, { headers: csrf });

  expect(removed.ok()).toBe(true);
  await expect(pane(page).getByText("thread-live.png")).toHaveCount(0);
  await expect(rows).toHaveCount(loaded);
});

matrix("the threads pane", async ({ page, theme }) => {
  await open(page, `r/${GENERAL}`, theme);
  await openHeaderTool(page, /^Threads/);

  await expect(pane(page).getByRole("heading", { name: "Threads" })).toBeVisible();
  await expect(pane(page).locator(".thread-row").first()).toBeVisible();
  await page.mouse.move(0, 0);
  await shot(page, "threads", theme);

  await pane(page).getByRole("tab", { name: "Following" }).click();
  await expect(pane(page).locator(".thread-row")).toHaveCount(1);

  await pane(page).getByRole("tab", { name: "Closed" }).click();
  await expect(pane(page).locator(".thread-row").first()).toBeVisible();
  await shot(page, "threads-closed", theme);
});

test("the header buttons show which pane is open", async ({ page }) => {
  await open(page, `r/${GENERAL}`);

  const files = page.locator(".room-header").getByRole("button", { name: "Files" });

  await files.click();
  await expect(files).toHaveAttribute("aria-pressed", "true");
  await files.click();
  await expect(files).toHaveAttribute("aria-pressed", "false");
  await expect(pane(page)).toHaveCount(0);
});

test.describe("pane headers on a 360 px touch phone", () => {
  test.use(PHONE_TOUCH);

  for (const [name, heading] of [
    [/^Members/, "Members"],
    [/^Pinned messages/, "Pinned messages"],
    [/^Files/, "Files"],
    [/^Threads/, "Threads"],
  ] as const) {
    test(`the ${heading} pane is a pushed page with a touch-size back button`, async ({ page }) => {
      await open(page, `r/${GENERAL}`);
      await openHeaderTool(page, name);

      const header = pane(page).locator(".pane-header");
      const back = header.getByRole("button", { name: "Back to #general" });

      await expect(header.getByRole("heading", { name: heading, exact: true })).toBeVisible();
      await expect(back).toBeVisible();
      // No close button: Back is the way out of a pushed page.
      await expect(header.getByRole("button", { name: "Close" })).toHaveCount(0);
      await expectTouchTargets(page, "aside.right-pane .pane-header");
      await expectNoHorizontalOverflow(page);

      // The back chevron leads at the screen's edge, the title right after it.
      const backBox = await back.boundingBox();
      const titleBox = await header.locator(".pane-title").boundingBox();

      expect(backBox?.x).toBeLessThanOrEqual(8);
      expect((titleBox?.x ?? 0) - ((backBox?.x ?? 0) + (backBox?.width ?? 0))).toBeLessThanOrEqual(
        12,
      );

      await back.click();
      await expect(pane(page)).toHaveCount(0);
      await expect(page.locator(".room-header .room-title-name")).toBeInViewport();
    });
  }

  for (const [name, heading, rows] of [
    [/^Threads/, "Threads", ".thread-list > li"],
    [/^Files/, "Files", ".file-row"],
  ] as const) {
    test(`the ${heading} pane's rows fit its body without scrolling sideways`, async ({ page }) => {
      await open(page, `r/${GENERAL}`);
      await openHeaderTool(page, name);
      await expect(pane(page).locator(rows).first()).toBeVisible();

      const body = pane(page).locator(".pane-body");

      const { scrollWidth, clientWidth } = await body.evaluate((element) => ({
        scrollWidth: element.scrollWidth,
        clientWidth: element.clientWidth,
      }));

      expect(scrollWidth, "the pane body's content width").toBeLessThanOrEqual(clientWidth);
      await expectNoHorizontalOverflow(page);
    });
  }
});
