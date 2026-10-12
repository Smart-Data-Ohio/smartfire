import type { APIRequestContext, Locator, Page } from "@playwright/test";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { PinList } from "../../src/gen/PinList.ts";
import type { SearchResults } from "../../src/gen/SearchResults.ts";
import {
  expect,
  openApp,
  openHeaderTool,
  PHONE_TOUCH,
  ROOM_IDS,
  shot,
  THEMES,
  type Theme,
  test,
  USER_IDS,
} from "./support.ts";

/**
 * A Markdown document pasted into a message (headings, a rule, lists) reads with a chat-sized
 * rhythm: more room above a heading than below it, a visible rule with space on both sides, and
 * list items a step apart, while ordinary paragraphs keep their 4 px step so a short message
 * doesn't change. Clamped previews (pins, search hits) still show whole lines, and on a phone
 * "(edited)" runs on after the last word whether or not the body has its production wrapper.
 */

const VIEWPORT = { width: 1280, height: 1200 } as const;

/** The blocks of the document the person pasted (abridged). */
const DOCUMENT_BLOCKS =
  "<h1>Smartfire: What It Can Do</h1>" +
  "<p>Smartfire is Smart Data's team workspace, where people and AI agents work side by side. " +
  "It brings chat, voice and video, project tracking, and the tools we already use into one place.</p>" +
  "<hr>" +
  "<h2>Conversations</h2>" +
  "<ul>" +
  "<li><strong>Channels</strong> for teams, projects, and topics.</li>" +
  "<li><strong>Direct messages and group DMs</strong> for one-on-one or small-group conversations.</li>" +
  "<li><strong>Threads</strong> keep side conversations organized without cluttering the main channel. " +
  "Private threads are visible only to the people invited into them.</li>" +
  "</ul>" +
  "<h2>Files and Media</h2>" +
  "<ul>" +
  "<li><strong>File sharing up to 100 MB</strong> with previews.</li>" +
  "<li><strong>Link previews</strong> for websites:<ul>" +
  "<li>X (Twitter) posts</li><li>GitHub pull requests</li></ul></li>" +
  "<li><strong>Google Drive:</strong> pick Drive files right from the message box.</li>" +
  "</ul>" +
  "<h3>Emoji and fun</h3>" +
  "<p>Custom emoji, stickers and GIFs.</p>";

/** A message body as production renders it: the blocks in a `trix-content` wrapper (content.rs). */
function wrapped(blocks: string): string {
  return `<div class="trix-content">\n  ${blocks}\n</div>\n`;
}

/** Ten one-line list items, as production renders them. */
const LIST_BODY = wrapped(
  `<ul>${Array.from({ length: 10 }, (_, index) => `<li>Item ${index + 1}</li>`).join("")}</ul>`,
);

/** `messages` with message `id`'s body swapped for `bodyHtml`. */
function swapBody(messages: MessageDTO[], id: number, bodyHtml: string): MessageDTO[] {
  return messages.map((message) => (message.id === id ? { ...message, bodyHtml } : message));
}

function row(page: Page, messageId: number): Locator {
  return page.locator(`[data-message-row][data-message-id="${messageId}"]`).first();
}

/** Has Jonah post `markdown` in #general and returns the message id. */
async function post(request: APIRequestContext, markdown: string): Promise<number> {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.post("/__mock/post", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { roomId: ROOM_IDS.general, userId: USER_IDS.jonah, markdown },
  });

  return (await response.json()).id;
}

async function openOn(page: Page, messageId: number, theme: Theme): Promise<void> {
  await page.setViewportSize(VIEWPORT);
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM_IDS.general}/m/${messageId}`);
  await expect(row(page, messageId)).toBeVisible();
}

/** Computed box metrics for `locator`, in px. */
function metrics(locator: Locator) {
  return locator.evaluate((element) => {
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();

    return {
      top: rect.top,
      bottom: rect.bottom,
      height: rect.height,
      marginTop: Number.parseFloat(style.marginTop),
      marginBottom: Number.parseFloat(style.marginBottom),
      fontSize: Number.parseFloat(style.fontSize),
      fontWeight: style.fontWeight,
      borderTopWidth: Number.parseFloat(style.borderTopWidth),
      borderTopStyle: style.borderTopStyle,
      borderTopColor: style.borderTopColor,
    };
  });
}

/** The px value of a length token, or the colour of a colour token, resolved on the page. */
function resolveToken(page: Page, property: "font-size" | "color", token: string): Promise<string> {
  return page.evaluate(
    ([prop, name]) => {
      const probe = document.createElement("span");

      probe.style.setProperty(prop, `var(${name})`);
      document.body.append(probe);

      const value = getComputedStyle(probe).getPropertyValue(prop);

      probe.remove();

      return value;
    },
    [property, token] as const,
  );
}

/** Production's wrapped body, and the bare blocks the mock and some other producers emit. */
const RENDERINGS = [
  { name: "wrapped", html: wrapped(DOCUMENT_BLOCKS), container: ":scope > .trix-content" },
  { name: "bare", html: DOCUMENT_BLOCKS, container: ":scope" },
] as const;

for (const rendering of RENDERINGS) {
  for (const theme of rendering.name === "wrapped" ? THEMES : (["light"] as const)) {
    test(`a pasted Markdown document keeps a compact rhythm (${rendering.name}, ${theme})`, async ({
      page,
      request,
    }) => {
      const id = await post(request, "The document");

      await page.route(`**/api/v1/rooms/${ROOM_IDS.general}/messages*`, async (route) => {
        const response = await route.fetch();
        const reply: MessagePage = await response.json();

        await route.fulfill({
          response,
          json: { ...reply, messages: swapBody(reply.messages, id, rendering.html) },
        });
      });

      await openOn(page, id, theme);

      const body = row(page, id).locator(".message-body");
      const blocks = body.locator(rendering.container);

      await expect(body.locator("h1")).toBeVisible();
      await row(page, id).scrollIntoViewIfNeeded();

      if (rendering.name === "wrapped") {
        await shot(page, "message-markdown", theme);
      }

      const h1 = await metrics(blocks.locator(":scope > h1"));
      const intro = await metrics(blocks.locator(":scope > p").first());
      const rule = await metrics(blocks.locator(":scope > hr"));
      const h2 = await metrics(blocks.locator(":scope > h2").first());
      const list = await metrics(blocks.locator(":scope > ul").first());
      const secondH2 = await metrics(blocks.locator(":scope > h2").nth(1));
      const h3 = await metrics(blocks.locator(":scope > h3"));
      const last = await metrics(blocks.locator(":scope > :last-child"));
      const outer = await metrics(body);

      // The first block sits on the body's top edge; the last adds nothing below.
      expect(h1.marginTop).toBe(0);
      expect(h1.top).toBeCloseTo(outer.top, 0);
      expect(last.marginBottom).toBe(0);
      expect(last.bottom).toBeCloseTo(outer.bottom, 0);

      // Chat-sized headings from the type scale, semibold.
      expect(`${h1.fontSize}px`).toBe(await resolveToken(page, "font-size", "--text-page"));
      expect(`${h2.fontSize}px`).toBe(await resolveToken(page, "font-size", "--text-title"));
      expect(`${h3.fontSize}px`).toBe(await resolveToken(page, "font-size", "--text-body"));
      expect(h1.fontWeight).toBe("600");

      // The rule is visible, in the strong border colour, with room on both sides.
      expect(rule.borderTopWidth).toBe(1);
      expect(rule.borderTopStyle).toBe("solid");
      expect(rule.borderTopColor).toBe(await resolveToken(page, "color", "--border-strong"));
      expect(rule.top - intro.bottom).toBeGreaterThanOrEqual(12);
      expect(h2.top - rule.bottom).toBeGreaterThanOrEqual(12);

      // A heading keeps more space above it than below: it belongs to what follows.
      const aboveH2 = secondH2.top - list.bottom;
      const belowH2 = list.top - h2.bottom;

      expect(aboveH2).toBeGreaterThanOrEqual(16);
      expect(belowH2).toBeGreaterThanOrEqual(3.5);
      expect(belowH2).toBeLessThanOrEqual(4.5);

      // List items and nested lists sit a small step apart.
      const items = blocks.locator(":scope > ul").first().locator(":scope > li");

      expect((await metrics(items.nth(1))).marginTop).toBe(2);
      expect((await metrics(blocks.locator("li > ul"))).marginTop).toBe(2);
    });
  }
}

/** Every one of the first `count` list items in `body` shows whole: none is cut by its bottom. */
async function expectWholeLines(body: Locator, count: number): Promise<void> {
  const box = await metrics(body);
  const items = body.locator("li");

  for (let index = 0; index < count; index += 1) {
    const item = await metrics(items.nth(index));

    expect(item.bottom, `item ${index + 1} ends inside the preview`).toBeLessThanOrEqual(
      box.bottom + 0.5,
    );
  }
}

test("a pinned list shows eight whole lines", async ({ page }) => {
  await page.route(`**/api/v1/rooms/${ROOM_IDS.general}/pins*`, async (route) => {
    const response = await route.fetch();
    const reply: PinList = await response.json();

    await route.fulfill({
      response,
      json: {
        ...reply,
        messages: reply.messages.map((message) => ({ ...message, bodyHtml: LIST_BODY })),
      },
    });
  });

  await page.setViewportSize(VIEWPORT);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM_IDS.general}`);
  await openHeaderTool(page, /^Pinned messages/);

  const body = page.locator("aside.right-pane .pin-card-body").first();

  await expect(body.locator("li").first()).toHaveText("Item 1");

  // Eight 22 px lines, no step between them, and the eighth whole under the cap.
  expect((await metrics(body.locator("li").nth(1))).marginTop).toBe(0);
  expect((await metrics(body)).height).toBeCloseTo(8 * 22, 0);
  await expectWholeLines(body, 8);
});

test("a search hit's list shows five whole lines", async ({ page, request }) => {
  const id = await post(request, "pasteddocument list");

  await page.route("**/api/v1/search*", async (route) => {
    const response = await route.fetch();
    const reply: SearchResults = await response.json();

    await route.fulfill({
      response,
      json: { ...reply, messages: swapBody(reply.messages, id, LIST_BODY) },
    });
  });

  await page.setViewportSize(VIEWPORT);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/app/search?q=pasteddocument");

  const body = page.locator("[data-search-hit] .search-hit-body").first();

  await expect(body.locator("li").first()).toHaveText("Item 1");

  // The clamp ends the box on the fifth line, whatever the step between items.
  await expectWholeLines(body, 5);
  expect((await metrics(body.locator("li").nth(5))).top).toBeGreaterThanOrEqual(
    (await metrics(body)).bottom - 0.5,
  );
});

test("ordinary short messages keep their height", async ({ page, request }) => {
  await post(request, "Are we still on for the demo at three?");

  const id = await post(request, "Yes, room B.\n\nI'll bring the projector adapter.");

  await openOn(page, id, "light");

  const body = row(page, id).locator(".message-body");
  const paragraphs = body.locator(":scope > p");

  await expect(paragraphs).toHaveCount(2);
  await shot(page, "message-markdown-short", "light");

  // Two 22 px lines and the 4 px paragraph step, exactly as before.
  const first = await metrics(paragraphs.first());
  const second = await metrics(paragraphs.nth(1));

  expect(first.marginTop).toBe(0);
  expect(second.marginTop).toBe(4);
  expect((await metrics(body)).height).toBeCloseTo(first.height + 4 + second.height, 0);
  expect(first.height).toBeCloseTo(22, 0);
});

test.describe("on a touch phone", () => {
  test.use(PHONE_TOUCH);

  const FIRST = "Notes from the call";

  const LAST =
    "A last paragraph long enough to wrap onto a second line on a phone screen, now edited";

  const BLOCKS = `<p>${FIRST}</p><p>${LAST}</p>`;

  for (const rendering of [
    { name: "wrapped", html: wrapped(BLOCKS) },
    { name: "bare", html: BLOCKS },
  ] as const) {
    test(`(edited) runs on after an edited body's last word (${rendering.name})`, async ({
      page,
      request,
    }) => {
      // A continuation of the group: a group's first row shows "(edited)" in its header instead.
      await post(request, "First of two, so the next one continues the group");

      const id = await post(request, "To be edited");

      await page.route(`**/api/v1/rooms/${ROOM_IDS.general}/messages*`, async (route) => {
        const response = await route.fetch();
        const reply: MessagePage = await response.json();

        await route.fulfill({
          response,
          json: {
            ...reply,
            // Edited, so the row carries "(edited)".
            messages: swapBody(reply.messages, id, rendering.html).map((message) =>
              message.id === id ? { ...message, editedAt: message.createdAt } : message,
            ),
          },
        });
      });

      await openApp(page, `r/${ROOM_IDS.general}/m/${id}`);

      const bodyRow = row(page, id).locator(".message-body-row");

      await expect(bodyRow.locator(":scope > .message-edited")).toBeVisible();
      await expect(bodyRow.locator(".message-body p").nth(1)).toHaveText(LAST);

      const placement = await bodyRow.evaluate((element) => {
        const paragraphs = element.querySelectorAll(".message-body p");
        const first = paragraphs[0]?.getBoundingClientRect();
        const last = paragraphs[1];
        const mark = element.querySelector(".message-edited")?.getBoundingClientRect();

        if (first === undefined || last === undefined || mark === undefined) {
          throw new Error("no paragraphs or no (edited)");
        }

        const range = document.createRange();

        range.selectNodeContents(last);

        const lines = [...range.getClientRects()].filter((rect) => rect.width > 0);
        const end = lines.at(-1);
        const start = lines.at(0);

        if (end === undefined || start === undefined) {
          throw new Error("the last paragraph has no text");
        }

        return {
          lines: new Set(lines.map((rect) => Math.round(rect.top))).size,
          sameLine: mark.top < end.bottom && mark.bottom > end.top,
          gap: mark.left - end.right,
          step: start.top - first.bottom,
        };
      });

      expect(placement.lines, "the last paragraph wraps").toBeGreaterThan(1);
      expect(placement.sameLine, "(edited) sits on the body's last line").toBe(true);
      expect(placement.gap, "right after the last word").toBeGreaterThanOrEqual(0);
      expect(placement.gap, "right after the last word").toBeLessThan(12);
      // The paragraph before the inline last one keeps the 4 px step.
      expect(placement.step).toBeGreaterThanOrEqual(3.5);
    });
  }
});
