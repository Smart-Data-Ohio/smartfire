import type { APIRequestContext, Page } from "@playwright/test";
import { BOARD_ROOM_ID } from "../../mock/s6/seed.ts";
import {
  DESKTOP,
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  openApp,
  PHONE_TOUCH,
  shot,
  test,
} from "./support.ts";

const BOARD = BOARD_ROOM_ID;

/** Someone else replaces the board's catalog through the mock's `/__mock/board-tags` control. */
async function curate(
  request: APIRequestContext,
  body: {
    tags: { name: string; emoji?: string }[];
    tagsRequired?: boolean;
    defaultTag?: string;
  },
): Promise<void> {
  const reply = await request.post("/__mock/board-tags", { data: { roomId: BOARD, ...body } });

  expect(reply.ok()).toBe(true);
}

function editor(page: Page) {
  return page.getByRole("dialog", { name: "Board settings" });
}

function rows(page: Page) {
  return editor(page).getByRole("list", { name: "Board tags" }).getByRole("listitem");
}

async function openTags(page: Page): Promise<void> {
  await page.getByRole("button", { name: /^Filter/ }).click();
  await page.getByRole("menuitem", { name: /^Tag/ }).click();
  await page.getByRole("menuitem", { name: "Manage board tags…" }).click();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}/settings$`));
  await expect(editor(page).getByRole("tab", { name: "Tags" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
}

test("a board admin curates tags from the toolbar's tag filter", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}`);
  await openTags(page);

  const dialog = editor(page);

  await expect(dialog.getByText("No tags yet. Add the first below.")).toBeVisible();
  await expect(dialog.getByRole("switch", { name: /Require a tag/ })).toBeDisabled();

  // An emoji from the shared picker, then the name; Enter adds without saving the room form.
  await dialog.getByRole("button", { name: "Emoji for the new tag" }).click();

  const search = page.getByRole("combobox", { name: "Search emoji" });

  await expect(search).toBeFocused();
  await search.fill("bug");
  await page.keyboard.press("Enter");
  await expect(
    dialog.getByRole("button", { name: /^Emoji for the new tag \(.+\)$/ }),
  ).toBeVisible();

  const name = dialog.getByRole("textbox", { name: "New tag" });

  await name.fill("Bug");
  await name.press("Enter");
  await expect(rows(page)).toHaveCount(1);
  await expect(dialog.getByRole("button", { name: /^Emoji for Bug \(.+\)$/ })).toBeVisible();
  await expect(name).toHaveValue("");

  await name.fill("Idea");
  await dialog.getByRole("button", { name: "Add tag" }).click();
  await expect(rows(page)).toHaveCount(2);
  await name.fill("bug");
  await name.press("Enter");
  await expect(dialog.getByText("“Bug” is already a tag here.")).toBeVisible();
  await name.fill("");

  // Keyboard reorder: the handle keeps focus and the new place is announced.
  const handle = dialog.getByRole("button", { name: "Move Idea, position 2 of 2" });

  await handle.focus();
  await page.keyboard.press("ArrowUp");
  await expect(dialog.getByRole("button", { name: "Move Idea, position 1 of 2" })).toBeFocused();
  await expect(page.getByText("Idea moved to position 1 of 2.")).toBeAttached();
  await expect(rows(page).first()).toContainText("Idea");

  await dialog.getByRole("button", { name: "Idea options" }).click();
  await page.getByRole("menuitem", { name: "Rename" }).click();

  const rename = dialog.getByRole("textbox", { name: "Rename Idea" });

  await expect(rename).toBeFocused();
  await rename.fill("Proposal");
  await rename.press("Enter");
  await expect(rows(page).first()).toContainText("Proposal");

  await dialog.getByLabel("Start new posts with").selectOption({ label: "Proposal" });
  await expect(rows(page).first().getByText("Default")).toBeVisible();
  await dialog.getByRole("switch", { name: /Require a tag/ }).click();
  await expect(dialog.getByRole("switch", { name: /Require a tag/ })).toBeChecked();
  await expect(dialog.getByRole("status").filter({ hasText: "Saving…" })).toHaveCount(0);
  await shot(page, "board-tag-editor", "light");

  await dialog.getByRole("button", { name: "Bug options" }).click();
  await page.getByRole("menuitem", { name: "Delete…" }).click();

  const confirm = page.getByRole("alertdialog", { name: "Delete “Bug”?" });

  // The seeded posts tagged "bug" count: a curated tag matches them without regard to case.
  await expect(confirm.getByText("2 posts lose it.")).toBeVisible();
  await expect(confirm.getByText("This can't be undone.")).toBeVisible();
  await confirm.getByRole("button", { name: "Delete tag" }).click();
  await expect(confirm).toBeHidden();
  await expect(rows(page)).toHaveCount(1);
  await expect(
    dialog.getByRole("button", { name: "Move Proposal, position 1 of 1" }),
  ).toBeFocused();

  // The board's filter bar picks up the catalog as it stands.
  await page.keyboard.press("Escape");
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}$`));

  const strip = page.getByRole("group", { name: "Filter by board tag" });

  await expect(strip.getByRole("button", { name: /Proposal/ })).toBeVisible();
  await expect(strip.getByRole("button", { name: /Bug/ })).toHaveCount(0);
});

test("a post on a board that requires a tag starts with the default and needs one", async ({
  page,
  request,
}) => {
  await curate(request, {
    tags: [
      { name: "Bug", emoji: "🐞" },
      { name: "Idea", emoji: "💡" },
    ],
    tagsRequired: true,
    defaultTag: "Idea",
  });
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}`);
  await page.getByRole("button", { name: "New post" }).click();

  const dialog = page.getByRole("dialog", { name: "New post" });
  const picker = dialog.getByRole("group", { name: /Board tags/ });
  const bug = picker.getByRole("button", { name: /Bug/ });
  const idea = picker.getByRole("button", { name: /Idea/ });

  await expect(picker.getByText("Required")).toBeVisible();
  await expect(idea).toHaveAttribute("aria-pressed", "true");
  await expect(bug).toHaveAttribute("aria-pressed", "false");
  await expect(picker.getByText("Idea is the board's default.")).toBeVisible();

  await idea.click();
  await dialog.getByLabel("Title").fill("Crash when saving drafts");
  await dialog.getByRole("button", { name: "Create post" }).click();
  await expect(picker.getByText("Pick at least one of this board's tags.")).toBeVisible();
  await expect(bug).toBeFocused();
  await expect(picker).toHaveAttribute("aria-invalid", "true");

  await bug.click();
  await dialog.getByRole("textbox", { name: "More tags" }).fill("infra");
  await dialog.getByRole("button", { name: "Create post" }).click();
  await expect(dialog).toBeHidden();
  await expect(page).toHaveURL(new RegExp(`/r/${BOARD}/t/\\d+$`));
  await expect(
    page
      .getByRole("complementary", { name: "Crash when saving drafts" })
      .getByRole("listitem")
      .filter({ hasText: /^🐞\s*Bug$/ }),
  ).toBeVisible();

  const strip = page.getByRole("group", { name: "Filter by board tag" });

  await strip.getByRole("button", { name: /Bug/ }).click();
  await expect(strip.getByRole("button", { name: /Bug/ })).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByRole("link", { name: /Crash when saving drafts/ })).toBeVisible();
});

test("an open new-post dialog picks up tags someone else just curated", async ({
  page,
  request,
}) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${BOARD}`);
  await page.getByRole("button", { name: "New post" }).click();

  const dialog = page.getByRole("dialog", { name: "New post" });

  await expect(dialog.getByRole("textbox", { name: "Tags" })).toBeVisible();
  await expect(dialog.getByRole("group", { name: /Board tags/ })).toHaveCount(0);

  await curate(request, { tags: [{ name: "Bug", emoji: "🐞" }], defaultTag: "Bug" });

  const bug = dialog
    .getByRole("group", { name: /Board tags/ })
    .getByRole("button", { name: /Bug/ });

  await expect(bug).toHaveAttribute("aria-pressed", "true");
  await expect(dialog.getByRole("textbox", { name: "More tags" })).toBeVisible();
});

test.describe("on a phone", () => {
  test.use(PHONE_TOUCH);

  test("the tag editor, the chips and the filter bar fit and take a thumb", async ({
    page,
    request,
  }) => {
    await curate(request, {
      tags: [
        { name: "Bug", emoji: "🐞" },
        { name: "Idea", emoji: "💡" },
        { name: "Question", emoji: "❓" },
        { name: "Shipped", emoji: "🚀" },
        { name: "Needs design", emoji: "🎨" },
      ],
      tagsRequired: true,
      defaultTag: "Idea",
    });
    await openApp(page, `r/${BOARD}`);

    const strip = page.getByRole("group", { name: "Filter by board tag" });

    await expect(strip.getByRole("button", { name: /Needs design/ })).toBeAttached();
    await expectTouchTargets(page, ".board-tag-strip");
    await expectNoHorizontalOverflow(page, { allowScroll: ".board-tag-strip" });

    await page.getByRole("button", { name: "New post" }).click();

    const dialog = page.getByRole("dialog", { name: "New post" });

    await expect(dialog.getByRole("group", { name: /Board tags/ })).toBeVisible();
    await expectTouchTargets(page, ".board-tag-options");
    await expectNoHorizontalOverflow(page, { allowScroll: ".board-tag-strip" });
    await dialog.getByRole("button", { name: "Cancel" }).click();

    await openApp(page, `r/${BOARD}/settings`);
    await editor(page).getByRole("tab", { name: "Tags" }).click();
    await expect(rows(page)).toHaveCount(5);
    // The Require switch's whole row is its label, so the track itself may stay small.
    await expectTouchTargets(page, ".tag-editor", { ignore: ".toggle" });
    await expectNoHorizontalOverflow(page, { allowScroll: ".board-tag-strip" });
    await shot(page, "board-tag-editor-phone", "light");

    // The keyboard path works from a phone's hardware keyboard too; the menu is an action sheet.
    await editor(page).getByRole("button", { name: "Shipped options" }).click();
    await page.getByRole("menuitem", { name: "Move up" }).click();
    await expect(rows(page).nth(2)).toContainText("Shipped");
  });
});
