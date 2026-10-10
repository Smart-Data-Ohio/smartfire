import type { Page } from "@playwright/test";
import {
  expect,
  expectTouchTargets,
  openApp,
  PHONE_TOUCH,
  postMessage,
  ROOM_IDS,
  shot,
  type Theme,
  test,
  USER_IDS,
} from "./support.ts";

const ENGINEERING = `r/${ROOM_IDS.engineering}`;

const SOURCE = 'fn main() {\n    // <script>alert("x")</script>\n    let answer = 42;\n}';

const MARKDOWN = `The fix:\n\n\`\`\`rust\n${SOURCE}\n\`\`\``;

/** The newest message's code block, once its colours are in. */
async function colouredBlock(page: Page) {
  const block = page.getByRole("log", { name: "Messages" }).locator(".code-block").last();

  await expect(block.locator(".code-token--keyword").first()).toBeVisible();

  return block;
}

/** A token's painted colour. */
function colourOf(page: Page, selector: string): Promise<string> {
  return page
    .locator(selector)
    .last()
    .evaluate((element) => getComputedStyle(element).color);
}

for (const theme of ["light", "dark"] as const satisfies readonly Theme[]) {
  test(`colours a fenced block from the design tokens and copies it (${theme})`, async ({
    page,
    context,
    request,
  }) => {
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);
    await postMessage(request, {
      roomId: ROOM_IDS.engineering,
      userId: USER_IDS.maya,
      markdown: MARKDOWN,
    });
    await openApp(page, ENGINEERING, theme);

    const block = await colouredBlock(page);
    const code = block.locator("pre code");

    await expect(code).toHaveAttribute("data-code-language", "rust");
    await expect(code).toHaveText(SOURCE);
    await expect(block.locator(".code-token--keyword").first()).toHaveText("fn");
    await expect(block.locator(".code-token--number")).toHaveText("42");
    // The <script> in the comment is text: no element, and the page never ran it.
    await expect(block.locator("script")).toHaveCount(0);
    await expect(block.locator(".code-token--comment")).toContainText("<script>");

    // Each kind takes its own token colour, none of them the block's plain text colour.
    const keyword = await colourOf(page, ".code-block .code-token--keyword");
    const number = await colourOf(page, ".code-block .code-token--number");
    const plain = await block.locator("pre").evaluate((element) => getComputedStyle(element).color);

    const expected = await page.evaluate(() => {
      const probe = document.createElement("span");

      probe.style.color = "var(--code-keyword)";
      document.body.append(probe);

      const color = getComputedStyle(probe).color;

      probe.remove();

      return color;
    });

    expect(keyword).toBe(expected);
    expect(new Set([keyword, number, plain]).size).toBe(3);

    await block.hover();
    await shot(page, "code-block", theme);

    const copy = block.getByRole("button", { name: "Copy code" });

    await copy.click();
    await expect(block.getByRole("button", { name: "Copied" })).toBeVisible();
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(SOURCE);
    await expect(block.getByRole("button", { name: "Copy code" })).toBeVisible();
  });
}

test("the Copy button works from the keyboard", async ({ page, context, request }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await postMessage(request, {
    roomId: ROOM_IDS.engineering,
    userId: USER_IDS.maya,
    markdown: MARKDOWN,
  });
  await openApp(page, ENGINEERING);

  const block = await colouredBlock(page);
  const copy = block.getByRole("button", { name: "Copy code" });

  await copy.focus();
  await expect(copy).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(block.getByRole("button", { name: "Copied" })).toBeFocused();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(SOURCE);
});

test("the highlighter loads only when a message holds a code block", async ({ page, request }) => {
  const fetched: string[] = [];

  page.on("request", (sent) => {
    // The worker and what it loads; the small client in the entry chunk doesn't count.
    if (
      /code-highlight\/(worker|tokenize|detect)\.ts|\/assets\/(worker|detect)-|shiki|highlight\.js/.test(
        sent.url(),
      )
    ) {
      fetched.push(sent.url());
    }
  });

  // Settings render no message body, so nothing of the highlighter loads.
  await openApp(page, "settings");
  await expect(page.getByRole("main")).toBeVisible();
  expect(fetched).toEqual([]);

  await postMessage(request, {
    roomId: ROOM_IDS.engineering,
    userId: USER_IDS.maya,
    markdown: MARKDOWN,
  });
  await page.goto(`/app/${ENGINEERING}`);
  await colouredBlock(page);
  // The dev server's worker module, or the build's worker chunk.
  expect(fetched.some((url) => /code-highlight\/worker\.ts|\/assets\/worker-/.test(url))).toBe(
    true,
  );
});

test("colours a block in the composer preview", async ({ page }) => {
  await openApp(page, ENGINEERING);

  const composer = page.getByRole("textbox", { name: /^Message #/ });

  await composer.fill('```ts\nconst value: string = "hello";\n```');
  await page.getByRole("button", { name: "Attach and more" }).click();
  await page.getByRole("menuitem", { name: "Preview message" }).click();

  const preview = page.locator(".composer-preview-body");

  await expect(preview.locator(".code-token--keyword")).toHaveText("const");
  await expect(preview.locator(".code-token--string")).toHaveText('"hello"');
  await expect(preview.getByRole("button", { name: "Copy code" })).toBeAttached();
});

test.describe("on a touch phone", () => {
  test.use(PHONE_TOUCH);

  test("the Copy button always shows, at a finger's size", async ({ page, context, request }) => {
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);
    await postMessage(request, {
      roomId: ROOM_IDS.engineering,
      userId: USER_IDS.maya,
      markdown: MARKDOWN,
    });
    await openApp(page, ENGINEERING);

    const block = await colouredBlock(page);
    const copy = block.getByRole("button", { name: "Copy code" });

    await expect(copy).toHaveCSS("opacity", "1");
    // Only this message's block: the seeded ones are checked the same way wherever they show.
    await expectTouchTargets(page, ".code-block:has(.code-token--number)");
    await shot(page, "code-block", "light");
    await copy.tap();
    await expect(block.getByRole("button", { name: "Copied" })).toBeVisible();
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(SOURCE);
  });
});
