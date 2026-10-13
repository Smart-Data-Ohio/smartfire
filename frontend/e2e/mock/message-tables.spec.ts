import type { Locator, Page } from "@playwright/test";
import { THREAD_IDS } from "../../mock/s2/seed.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { PinList } from "../../src/gen/PinList.ts";
import type { SavedItemList } from "../../src/gen/SavedItemList.ts";
import type { SearchResults } from "../../src/gen/SearchResults.ts";
import {
  expect,
  expectNoHorizontalOverflow,
  openApp,
  openHeaderTool,
  PHONE,
  ROOM_IDS,
  shot,
  THEMES,
  test,
  USER_IDS,
} from "./support.ts";

/**
 * A Markdown table in a message shows as a table wherever message bodies do: a header row, cell
 * padding and borders from the design tokens, each column's `:---:` alignment, and a wide table
 * scrolling sideways inside the message rather than widening the timeline, a pane or the page.
 * A task list shows its read-only checkboxes, and a numbered list its start.
 *
 * The bodies are what the server sends for a GFM table (crates/richtext): comrak's markup inside
 * the `trix-content` wrapper. The mock's own Markdown renderer has no tables, so the specs swap
 * them in, as message-markdown.spec.ts does.
 */

const DESKTOP = { width: 1280, height: 1000 } as const;

/** Comrak's markup for a table: one tag a line, `align` on the cells of an aligned column. */
function table(
  header: readonly string[],
  rows: readonly (readonly string[])[],
  align: readonly string[] = [],
) {
  const cell = (tag: string, text: string, column: number) => {
    const side = align[column];

    return `<${tag}${side === undefined || side === "" ? "" : ` align="${side}"`}>${text}</${tag}>`;
  };

  const row = (tag: string, cells: readonly string[]) =>
    `<tr>\n${cells.map((text, column) => cell(tag, text, column)).join("\n")}\n</tr>`;

  return (
    `<table>\n<thead>\n${row("th", header)}\n</thead>\n` +
    `<tbody>\n${rows.map((cells) => row("td", cells)).join("\n")}\n</tbody>\n</table>`
  );
}

/** A message body as the server sends it: the blocks in a `trix-content` wrapper. */
function wrapped(blocks: string): string {
  return `<div class="trix-content">\n  ${blocks}\n</div>\n`;
}

/** `| Left | Center | Right |` over `| :--- | :---: | ---: |`, with inline marks in the cells. */
const ALIGNED = wrapped(
  table(
    ["Left", "Center", "Right"],
    [
      ["<code>code</code>", '<a href="https://example.com">link</a>', "<strong>bold</strong> 😄"],
      ["one", "two", "three"],
    ],
    ["left", "center", "right"],
  ),
);

/** A table after a paragraph line, wider than a phone, with long text in its last column. */
const WIDE = wrapped(
  `<p>Quarterly revenue by region:</p>\n${table(
    ["Region", "Owner", "Q1 revenue", "Q2 revenue", "Q3 revenue", "Q4 revenue", "Growth", "Notes"],
    [
      [
        "North America",
        "Jordan Avery",
        "$1,204,331",
        "$1,310,882",
        "$1,402,117",
        "$1,550,004",
        "28.7%",
        "Enterprise renewals carried the second half; two large accounts expanded seats",
      ],
      [
        "Europe, Middle East and Africa",
        "Samira Haddad",
        "$884,210",
        "$901,447",
        "$955,002",
        "$1,020,394",
        "15.4%",
        "Currency headwinds in Q2",
      ],
    ],
    ["", "", "right", "right", "right", "right", "center", ""],
  )}`,
);

/** `messages` with message `id`'s body swapped for `bodyHtml` (every message's, with no id). */
function swapBody(messages: MessageDTO[], bodyHtml: string, id?: number): MessageDTO[] {
  return messages.map((message) =>
    id === undefined || message.id === id ? { ...message, bodyHtml } : message,
  );
}

/** Has Jonah post `markdown` in #general and returns the message id. */
async function post(page: Page, markdown: string): Promise<number> {
  const state = await (await page.request.get("/__mock/state")).json();

  const response = await page.request.post("/__mock/post", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { roomId: ROOM_IDS.general, userId: USER_IDS.jonah, markdown },
  });

  return (await response.json()).id;
}

/** Answers a room's message pages with message `id`'s body swapped for `bodyHtml`. */
async function swapRoomBody(page: Page, id: number, bodyHtml: string): Promise<void> {
  await page.route(`**/api/v1/rooms/${ROOM_IDS.general}/messages*`, async (route) => {
    const response = await route.fetch();
    const reply: MessagePage = await response.json();

    await route.fulfill({
      response,
      json: { ...reply, messages: swapBody(reply.messages, bodyHtml, id) },
    });
  });
}

function row(page: Page, messageId: number): Locator {
  return page.locator(`[data-message-row][data-message-id="${messageId}"]`).first();
}

async function openMessage(page: Page, id: number): Promise<Locator> {
  await openApp(page, `r/${ROOM_IDS.general}/m/${id}`);

  const body = row(page, id).locator(".message-body");

  await expect(body.locator("table")).toBeVisible();
  await row(page, id).scrollIntoViewIfNeeded();

  return body;
}

/** The computed styles of the first matching cell, and its box. */
function cellStyle(cell: Locator) {
  return cell.evaluate((element) => {
    const style = getComputedStyle(element);

    return {
      textAlign: style.textAlign,
      fontWeight: style.fontWeight,
      paddingTop: Number.parseFloat(style.paddingTop),
      paddingLeft: Number.parseFloat(style.paddingLeft),
      borderTopWidth: Number.parseFloat(style.borderTopWidth),
      borderTopStyle: style.borderTopStyle,
      borderTopColor: style.borderTopColor,
      backgroundColor: style.backgroundColor,
    };
  });
}

/** The colour a token resolves to on the page. */
function resolveColor(page: Page, property: "color" | "background-color", token: string) {
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

/** The table's box, its scroll extent, and the boxes it must stay inside. */
function containment(tableLocator: Locator, container: string) {
  return tableLocator.evaluate((element, selector) => {
    const box = element.getBoundingClientRect();
    const outer = element.closest(selector)?.getBoundingClientRect();

    if (outer === undefined) {
      throw new Error(`no ${selector} around the table`);
    }

    return {
      right: box.right,
      containerRight: outer.right,
      clientWidth: element.clientWidth,
      scrollWidth: element.scrollWidth,
      viewport: window.innerWidth,
      pageWidth: document.documentElement.scrollWidth,
    };
  }, container);
}

/** The wide table stays inside `container` and the viewport, scrolling sideways when it must. */
async function expectContained(tableLocator: Locator, container: string): Promise<void> {
  const box = await containment(tableLocator, container);

  expect(box.right, "the table ends inside its container").toBeLessThanOrEqual(
    box.containerRight + 0.5,
  );
  expect(box.right, "the table ends inside the viewport").toBeLessThanOrEqual(box.viewport + 0.5);
  expect(box.pageWidth, "the page never scrolls sideways").toBeLessThanOrEqual(box.viewport);
}

for (const theme of THEMES) {
  test(`a Markdown table has a header row, padded bordered cells and its alignment (${theme})`, async ({
    page,
  }) => {
    const id = await post(page, "The aligned table");

    await swapRoomBody(page, id, ALIGNED);
    await page.setViewportSize(DESKTOP);
    await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });

    const body = await openMessage(page, id);
    const tableLocator = body.locator(":scope > .trix-content > table");

    await shot(page, "message-table", theme);

    // The structure the server sent: a header row of three, two body rows.
    await expect(tableLocator.locator("thead th")).toHaveText(["Left", "Center", "Right"]);
    await expect(tableLocator.locator("tbody tr")).toHaveCount(2);
    await expect(tableLocator.locator("tbody td code")).toHaveText("code");
    await expect(tableLocator.locator("tbody td a")).toHaveText("link");
    await expect(tableLocator.locator("tbody td strong")).toHaveText("bold");

    // Each column keeps its `:---`, `:---:` and `---:` alignment, header and cells alike.
    for (const cells of [
      tableLocator.locator("th"),
      tableLocator.locator("tbody tr").first().locator("td"),
    ]) {
      expect((await cellStyle(cells.nth(0))).textAlign).toBe("left");
      expect((await cellStyle(cells.nth(1))).textAlign).toBe("center");
      expect((await cellStyle(cells.nth(2))).textAlign).toBe("right");
    }

    // Padded cells with a visible border in the strong border colour; a semibold, shaded header.
    const header = await cellStyle(tableLocator.locator("th").first());
    const cell = await cellStyle(tableLocator.locator("td").first());
    const border = await resolveColor(page, "color", "--border-strong");

    for (const style of [header, cell]) {
      expect(style.paddingTop).toBe(4);
      expect(style.paddingLeft).toBe(12);
      expect(style.borderTopWidth).toBe(1);
      expect(style.borderTopStyle).toBe("solid");
      expect(style.borderTopColor).toBe(border);
    }

    expect(header.fontWeight).toBe("600");
    expect(header.backgroundColor).toBe(
      await resolveColor(page, "background-color", "--bg-sunken"),
    );
    expect(cell.fontWeight).toBe("400");
  });
}

test("an unaligned column's cells start at the start, header included", async ({ page }) => {
  const id = await post(page, "The unaligned table");

  await swapRoomBody(page, id, wrapped(table(["Name", "Status"], [["Alpha", "done"]])));
  await page.setViewportSize(DESKTOP);

  const body = await openMessage(page, id);

  // A browser centres `th` by default; a Markdown header reads from the left like its column.
  expect((await cellStyle(body.locator("th").first())).textAlign).toBe("start");
  expect((await cellStyle(body.locator("td").first())).textAlign).toBe("start");
});

for (const viewport of [
  { name: "desktop", size: DESKTOP },
  { name: "phone", size: PHONE },
] as const) {
  test(`a wide table scrolls inside its message (${viewport.name})`, async ({ page }) => {
    const id = await post(page, "The wide table");

    await swapRoomBody(page, id, WIDE);
    await page.setViewportSize(viewport.size);

    const body = await openMessage(page, id);
    const tableLocator = body.locator("table");

    await shot(page, `message-table-wide-${viewport.name}`, "light");

    await expect(body.locator("p")).toHaveText("Quarterly revenue by region:");
    await expect(tableLocator.locator("th")).toHaveCount(8);
    await expectContained(tableLocator, "[data-message-row]");
    await expectNoHorizontalOverflow(page);

    const box = await containment(tableLocator, ".message-body");

    if (viewport.name === "phone") {
      // Wider than the phone: the table scrolls within the message, and a swipe reaches the end.
      expect(box.scrollWidth).toBeGreaterThan(box.clientWidth);

      const scrolled = await tableLocator.evaluate((element) => {
        element.scrollLeft = element.scrollWidth;

        return element.scrollLeft;
      });

      expect(scrolled).toBeGreaterThan(0);
      await shot(page, "message-table-wide-scrolled", "light");
    }

    // Every cell is at least 12 characters wide: a column of prose stays readable, and the
    // table scrolls instead of squeezing it into a tall strip.
    const narrowest = await tableLocator.locator("th, td").evaluateAll((cells) =>
      Math.min(
        ...cells.map((cell) => {
          const probe = document.createElement("span");

          probe.style.cssText = "display: inline-block; width: 12ch";
          cell.append(probe);

          const twelve = probe.getBoundingClientRect().width;

          probe.remove();

          const style = getComputedStyle(cell);

          const content =
            cell.getBoundingClientRect().width -
            Number.parseFloat(style.paddingLeft) -
            Number.parseFloat(style.paddingRight) -
            Number.parseFloat(style.borderLeftWidth) -
            Number.parseFloat(style.borderRightWidth);

          return content - twelve;
        }),
      ),
    );

    expect(narrowest, "no cell narrower than 12ch").toBeGreaterThanOrEqual(-1);

    // Cells wrap at words, never mid-word: no header is broken inside a word.
    const headers = await tableLocator.locator("th").evaluateAll((cells) =>
      cells.map((cell) => {
        const range = document.createRange();

        range.selectNodeContents(cell);

        const lines = new Set([...range.getClientRects()].map((rect) => Math.round(rect.top)));

        return { text: cell.textContent ?? "", lines: lines.size };
      }),
    );

    for (const header of headers) {
      if (!header.text.includes(" ")) {
        expect(header.lines, `"${header.text}" on one line`).toBe(1);
      }
    }
  });
}

/** `- [ ] Draft` / `- [x] Review` and `3. three` as the server renders them (crates/richtext). */
const TASKS = wrapped(
  '<ul>\n<li><input type="checkbox" disabled="disabled"> Draft the plan</li>\n' +
    '<li><input type="checkbox" checked="" disabled="disabled"> Review it</li>\n</ul>\n' +
    '<ol start="3">\n<li>three</li>\n<li>four</li>\n</ol>',
);

test("a task list shows read-only checkboxes in place of bullets, and a list keeps its start", async ({
  page,
}) => {
  const id = await post(page, "The task list");

  await swapRoomBody(page, id, TASKS);
  await openApp(page, `r/${ROOM_IDS.general}/m/${id}`);

  const body = row(page, id).locator(".message-body");
  const boxes = body.locator('li > input[type="checkbox"]');

  await expect(boxes).toHaveCount(2);
  await row(page, id).scrollIntoViewIfNeeded();
  await shot(page, "message-task-list", "light");
  await expect(boxes.nth(0)).not.toBeChecked();
  await expect(boxes.nth(1)).toBeChecked();

  for (const box of [boxes.nth(0), boxes.nth(1)]) {
    await expect(box).toBeDisabled();
  }

  await expect(body.locator("ul > li").first()).toHaveCSS("list-style-type", "none");
  await expect(body.locator("ol")).toHaveAttribute("start", "3");
  await expect(body.locator("ol > li").first()).toHaveCSS("list-style-type", "decimal");

  // The checkbox sits inside the item's box, where the bullet was, and before its text.
  const placement = await body
    .locator("ul > li")
    .first()
    .evaluate((item) => {
      const input = item.querySelector("input");

      if (input === null || item.parentElement === null) {
        throw new Error("no checkbox in the task item");
      }

      const range = document.createRange();

      range.selectNodeContents(item);
      range.setStartAfter(input);

      return {
        box: input.getBoundingClientRect().toJSON(),
        list: item.parentElement.getBoundingClientRect().toJSON(),
        text: range.getBoundingClientRect().toJSON(),
      };
    });

  expect(placement.box.left).toBeGreaterThanOrEqual(placement.list.left - 0.5);
  expect(placement.box.right).toBeLessThanOrEqual(placement.text.left + 0.5);

  await boxes.nth(0).click({ force: true });
  await expect(boxes.nth(0)).not.toBeChecked();
});

/** `- [ ] Draft\n\n- [x] Review`: a loose list, so each label is a paragraph after its checkbox. */
const LOOSE_TASKS = wrapped(
  '<ul>\n<li><input type="checkbox" disabled="disabled"> \n<p>Draft</p>\n</li>\n' +
    '<li><input type="checkbox" checked="" disabled="disabled"> \n<p>Review</p>\n<p>More notes</p>\n</li>\n</ul>',
);

test("a loose task list keeps each checkbox on its label's line", async ({ page }) => {
  const id = await post(page, "The loose task list");

  await swapRoomBody(page, id, LOOSE_TASKS);
  await openApp(page, `r/${ROOM_IDS.general}/m/${id}`);

  const body = row(page, id).locator(".message-body");

  await expect(body.locator('li > input[type="checkbox"]')).toHaveCount(2);
  await row(page, id).scrollIntoViewIfNeeded();
  await shot(page, "message-task-list-loose", "light");

  const items = await body.locator("ul > li").evaluateAll((elements) =>
    elements.map((item) => {
      const input = item.querySelector("input");
      const [label, ...rest] = Array.from(item.querySelectorAll("p"));

      if (input === null || label === undefined) {
        throw new Error("no checkbox or label in the task item");
      }

      const range = document.createRange();

      range.selectNodeContents(label);

      return {
        box: input.getBoundingClientRect().toJSON(),
        label: range.getBoundingClientRect().toJSON(),
        rest: rest.map((paragraph) => paragraph.getBoundingClientRect().toJSON()),
      };
    }),
  );

  for (const item of items) {
    // The checkbox and its label share a line, the checkbox first.
    expect(item.box.top).toBeLessThan(item.label.bottom);
    expect(item.label.top).toBeLessThan(item.box.bottom);
    expect(item.box.right).toBeLessThanOrEqual(item.label.left + 0.5);

    // A later paragraph in the item still starts its own line.
    for (const paragraph of item.rest) {
      expect(paragraph.top).toBeGreaterThanOrEqual(item.label.bottom - 0.5);
    }
  }
});

test("a wide table stays inside the thread pane", async ({ page }) => {
  await page.route(`**/api/v1/threads/${THREAD_IDS.generalActive}/messages*`, async (route) => {
    const response = await route.fetch();
    const reply: MessagePage = await response.json();

    await route.fulfill({
      response,
      json: { ...reply, messages: swapBody(reply.messages, WIDE) },
    });
  });

  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${ROOM_IDS.general}/t/${THREAD_IDS.generalActive}`);

  const tableLocator = page.locator("aside.right-pane .message-body table").first();

  await expect(tableLocator).toBeVisible();
  await expectContained(tableLocator, "aside.right-pane");
  await expectNoHorizontalOverflow(page);
});

/**
 * A table in a preview (a search hit, a saved item): cut at the card's edge rather than scrolled,
 * since the card opens the message, and faded toward that edge only when it is cut.
 */
function previewTable(tableLocator: Locator) {
  return tableLocator.evaluate((element) => {
    const style = getComputedStyle(element);

    return {
      overflowX: style.overflowX,
      cut: element.scrollWidth > element.clientWidth + 1,
      faded: style.maskImage !== "none",
    };
  });
}

/** Clicks the middle of the part of `tableLocator` its `card` shows, as a pointer would. */
async function clickShownPart(page: Page, tableLocator: Locator, card: string): Promise<void> {
  const point = await tableLocator.evaluate((element, selector) => {
    const box = element.getBoundingClientRect();
    const outer = element.closest(selector)?.getBoundingClientRect() ?? box;
    const top = Math.max(box.top, outer.top);
    const bottom = Math.min(box.bottom, outer.bottom);

    return { x: (box.left + Math.min(box.right, outer.right)) / 2, y: (top + bottom) / 2 };
  }, card);

  await page.mouse.click(point.x, point.y);
}

test("a wide table in a search hit is cut with a fade, and clicking it opens the message", async ({
  page,
}) => {
  const id = await post(page, "quarterlytable revenue");
  const narrow = await post(page, "quarterlytable narrow");

  await page.route("**/api/v1/search*", async (route) => {
    const response = await route.fetch();
    const reply: SearchResults = await response.json();
    const messages = swapBody(swapBody(reply.messages, WIDE, id), ALIGNED, narrow);

    await route.fulfill({ response, json: { ...reply, messages } });
  });

  for (const size of [DESKTOP, PHONE]) {
    await page.setViewportSize(size);
    await openApp(page, "search?q=quarterlytable");

    const hits = page.locator("[data-search-hit]");
    const wide = hits.filter({ hasText: "Region" }).locator(".search-hit-body table");
    const fits = hits.filter({ hasText: "Center" }).locator(".search-hit-body table");

    await expect(wide).toBeVisible();
    await shot(page, "message-table-search-hit", "light");
    await expectContained(wide, "[data-search-hit]");
    await expectNoHorizontalOverflow(page);
    expect(await previewTable(wide)).toEqual({ overflowX: "hidden", cut: true, faded: true });

    if (size === DESKTOP) {
      // A table the card shows whole has no fade.
      await expect(fits).toBeVisible();
      expect(await previewTable(fits)).toEqual({ overflowX: "hidden", cut: false, faded: false });
    }
  }

  await clickShownPart(
    page,
    page.locator("[data-search-hit]").filter({ hasText: "Region" }).locator("table"),
    "[data-search-hit]",
  );
  await expect(page).toHaveURL(new RegExp(`/r/${ROOM_IDS.general}/m/${id}`));
});

test("a wide table in a saved item is cut with a fade, and clicking it opens the message", async ({
  page,
}) => {
  await page.route("**/api/v1/saved*", async (route) => {
    const response = await route.fetch();
    const reply: SavedItemList = await response.json();

    await route.fulfill({ response, json: { ...reply, messages: swapBody(reply.messages, WIDE) } });
  });

  for (const size of [DESKTOP, PHONE]) {
    await page.setViewportSize(size);
    await openApp(page, "saved");

    const tableLocator = page.locator(".saved-body.message-body table").first();

    await expect(tableLocator).toBeAttached();
    await expectContained(tableLocator, ".saved-body");
    await expectNoHorizontalOverflow(page);
    expect(await previewTable(tableLocator)).toEqual({
      overflowX: "hidden",
      cut: true,
      faded: true,
    });
  }

  const tableLocator = page.locator(".saved-body.message-body table").first();

  const messageId = await tableLocator.evaluate(
    (element) => element.closest("[data-message-id]")?.getAttribute("data-message-id") ?? "",
  );

  await clickShownPart(page, tableLocator, ".saved-body");
  // The message itself, or its thread with the message picked out.
  await expect(page).toHaveURL(new RegExp(`/m/${messageId}$|/t/\\d+\\?m=${messageId}$`));
});

test("a wide table stays inside its pinned card", async ({ page }) => {
  await page.route(`**/api/v1/rooms/${ROOM_IDS.general}/pins*`, async (route) => {
    const response = await route.fetch();
    const reply: PinList = await response.json();

    await route.fulfill({ response, json: { ...reply, messages: swapBody(reply.messages, WIDE) } });
  });

  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${ROOM_IDS.general}`);
  await openHeaderTool(page, /^Pinned messages/);

  const tableLocator = page.locator("aside.right-pane .pin-card-body table").first();

  await expect(tableLocator).toBeVisible();
  await expectContained(tableLocator, "aside.right-pane");
  await expectNoHorizontalOverflow(page);
});
