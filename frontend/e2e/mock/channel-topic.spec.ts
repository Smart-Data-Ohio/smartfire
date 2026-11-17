import type { APIRequestContext, Locator, Page } from "@playwright/test";
import {
  DESKTOP,
  expect,
  openApp,
  openHeaderTool,
  PHONE,
  ROOM_IDS,
  saveAccountAppearance,
  test,
  USER_IDS,
} from "./support.ts";

const ROOM = ROOM_IDS.launchPlanning;

const TOPIC = `<b>Planning</b> https://example.com/docs.\n${"A long channel topic. ".repeat(35)}`;

for (const viewport of [DESKTOP, PHONE]) {
  test(`a 1024-character topic keeps the room name readable at ${viewport.width}px`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize(viewport);
    await openApp(page, `r/${ROOM}/settings`);
    const dialog = page.getByRole("dialog", { name: "Channel settings" });
    const text = "W".repeat(1024);

    await dialog.getByRole("textbox", { name: "Topic" }).fill(text);
    await dialog.getByRole("button", { name: "Save changes" }).click();
    await expect(dialog).toBeHidden();
    const header = page.locator(".room-header");
    const name = header.getByRole("heading", { name: "launch-planning", level: 1 });

    await expect(name).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath(`topic-${viewport.width}.png`) });
    expect(
      await header
        .locator(".room-title-name")
        .evaluate((element) => element.scrollWidth <= element.clientWidth),
    ).toBe(true);
    expect(
      await header.evaluate((element) => {
        const title = element.querySelector(".room-title-button");
        const actions = element.querySelector(".page-header-actions");

        if (title === null || actions === null) throw new Error("Missing header controls");

        const titleBox = title.getBoundingClientRect();

        return (
          titleBox.right <= actions.getBoundingClientRect().left &&
          Array.from(title.children).every(
            (child) => child.getBoundingClientRect().right <= titleBox.right,
          ) &&
          element.scrollWidth <= element.clientWidth
        );
      }),
    ).toBe(true);
    const topic = header.getByRole("button", { name: "Channel topic, About" });

    if (viewport.width === DESKTOP.width) {
      await expect(topic).toHaveText(text);
      expect(
        await topic.evaluate((element) => {
          const title = element.parentElement?.querySelector(".room-title-button");

          if (title === null || title === undefined) throw new Error("Missing room name");

          return (
            element.getBoundingClientRect().left >= title.getBoundingClientRect().right &&
            element.scrollWidth > element.clientWidth &&
            getComputedStyle(element).textOverflow === "ellipsis"
          );
        }),
      ).toBe(true);
      await topic.click();
    } else {
      await expect(topic).toHaveCount(0);
      await header.getByRole("button", { name: "launch-planning, details" }).click();
    }

    await expect(page.getByRole("region", { name: "About" })).toContainText(text);
    await page.screenshot({ path: testInfo.outputPath(`topic-about-${viewport.width}.png`) });

    if (viewport.width === DESKTOP.width) {
      expect(await name.evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(
        true,
      );
      expect(await header.evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(
        true,
      );
    }
  });
}

// The room header at each text size's height token (--pane-header-height): 48 px, or a stacked
// pane title (1.25rem) over its subtitle (1rem) plus 6 px at Largest's 24 px root.
const HEADER_HEIGHT = { default: 48, largest: 60 } as const;

interface HeaderRoom {
  /** The room's name in test titles, when it isn't `name`. */
  readonly label?: string;
  readonly id: number;
  readonly name: string;
  readonly settings: { readonly field: string; readonly value: string } | null;
  readonly announcement?: boolean;
  /** Seven people join its call first: their faces widen the actions past room.css's steps. */
  readonly busy?: boolean;
}

const ANNOUNCEMENTS: HeaderRoom = {
  id: ROOM_IDS.announcements,
  name: "announcements",
  settings: null,
  announcement: true,
};

const BUSY_ANNOUNCEMENTS: HeaderRoom = {
  ...ANNOUNCEMENTS,
  label: "busy announcements",
  busy: true,
};

// #general has no topic and one scheduled message; launch-planning is given a topic;
// #announcements is made an announcement room, so it carries the "Announcement" tag, and the busy
// one has seven people in its call; the Lounge has people in its call and is given a voice limit,
// so its header shows their faces and "2/25".
const HEADER_ROOMS: readonly HeaderRoom[] = [
  { id: ROOM_IDS.general, name: "general", settings: null },
  {
    id: ROOM_IDS.launchPlanning,
    name: "launch-planning",
    settings: { field: "Topic", value: "Ship the launch checklist by Thursday" },
  },
  ANNOUNCEMENTS,
  BUSY_ANNOUNCEMENTS,
  { id: ROOM_IDS.lounge, name: "Lounge", settings: { field: "Voice user limit", value: "25" } },
];

const CALLERS = [
  USER_IDS.maya,
  USER_IDS.jonah,
  USER_IDS.priya,
  USER_IDS.sam,
  USER_IDS.lucia,
  USER_IDS.theo,
  USER_IDS.grace,
];

async function control(
  request: APIRequestContext,
  action: string,
  data: Record<string, number | string | boolean>,
): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.post(`/__mock/${action}`, {
    headers: { "X-CSRF-Token": state.csrfToken },
    data,
  });

  expect(response.ok(), await response.text()).toBe(true);
}

/** Opens `room` at `textSize`, after saving its topic or voice limit or making it announce. */
async function openHeaderRoom(
  page: Page,
  room: HeaderRoom,
  textSize: "default" | "largest",
): Promise<Locator> {
  if (room.busy === true) {
    for (const userId of CALLERS) {
      await control(page.request, "huddle-join", { roomId: room.id, userId });
    }
  }

  await openApp(page, "");
  await saveAccountAppearance(page, { textSize });

  if (room.announcement === true) {
    await openApp(page, `r/${room.id}/settings`);
    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    await dialog.getByRole("tab", { name: "Announcements" }).click();
    const flag = dialog.getByRole("switch", { name: /Announcement channel/ });

    await flag.click();
    await expect(flag).toHaveAttribute("aria-checked", "true");
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(page.locator(".room-header .room-announcement-tag")).toBeAttached();
  } else if (room.settings === null) {
    await openApp(page, `r/${room.id}`);
  } else {
    await openApp(page, `r/${room.id}/settings`);
    const dialog = page.getByRole("dialog");
    const field = dialog.getByLabel(room.settings.field);

    await (room.settings.field === "Topic"
      ? field.fill(room.settings.value)
      : field.selectOption(room.settings.value));
    await dialog.getByRole("button", { name: "Save changes" }).click();
    await expect(dialog).toBeHidden();
  }

  const header = page.locator(".room-header");
  await expect(header.getByRole("heading", { name: room.name, level: 1 })).toBeAttached();

  if (room.busy === true) {
    await expect(header.locator(".huddle-stack .avatar")).not.toHaveCount(0);
  }

  return header;
}

/** The narrowest the name gets (or its full width, if less) before the header folds for it. */
const NAME_FLOOR_REM = 6;

/**
 * What's wrong with the room header's layout, or nothing: a control past its edge, overflow, a
 * name narrower than `NAME_FLOOR_REM` (or its full width), overlapping the actions or truncated
 * beside the announcement tag, or a search narrower than its usable width.
 */
function headerProblems(header: Locator): Promise<string[]> {
  return header.evaluate((element, floorRem) => {
    const problems: string[] = [];
    const right = element.getBoundingClientRect().right;
    const name = element.querySelector(".room-title-name");
    const actions = element.querySelector(".page-header-actions");
    const search = element.querySelector<HTMLElement>(".header-search");
    const tag = element.querySelector<HTMLElement>(".room-announcement-tag");

    if (name === null || actions === null) throw new Error("Missing header parts");

    const rem = Number.parseFloat(getComputedStyle(document.documentElement).fontSize);
    const nameWidth = name.getBoundingClientRect().width;

    if (nameWidth + 0.5 < Math.min(floorRem * rem, name.scrollWidth)) {
      problems.push(`name ${Math.round(nameWidth)}px of ${name.scrollWidth}px`);
    }

    if (tag?.checkVisibility() && name.scrollWidth > name.clientWidth) {
      problems.push("tag beside a truncated name");
    }

    if (element.scrollWidth > element.clientWidth) {
      problems.push(`overflow ${element.scrollWidth} > ${element.clientWidth}`);
    }

    for (const control of actions.querySelectorAll(":is(button, a, input)")) {
      const box = control.getBoundingClientRect();

      if (box.width > 0 && box.right > right + 0.5) {
        problems.push(
          `${control.getAttribute("aria-label")} ${Math.round(box.right - right)}px out`,
        );
      }
    }

    // The name area (the name, its tag or topic) ends before the actions start.
    const actionsLeft = actions.getBoundingClientRect().left;

    for (const part of element.querySelectorAll(".room-title > *")) {
      const box = part.getBoundingClientRect();

      if (box.width > 0 && box.right > actionsLeft + 0.5) {
        problems.push(
          `${part.className} ${Math.round(box.right - actionsLeft)}px over the actions`,
        );
      }
    }

    if (search?.checkVisibility()) {
      const usable = Number.parseFloat(
        getComputedStyle(element).getPropertyValue("--header-search-min"),
      );

      if (search.getBoundingClientRect().width < usable) {
        problems.push(`search ${Math.round(search.getBoundingClientRect().width)}px`);
      }
    }

    return problems;
  }, NAME_FLOOR_REM);
}

/**
 * The header's measured state (`data-search`, `data-compact`) in each of four frames, after two
 * to settle: it mustn't change from one frame to the next at a fixed width.
 */
function fitFrames(header: Locator): Promise<string[]> {
  return header.evaluate(async (element) => {
    const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));
    const states: string[] = [];

    await frame();
    await frame();

    for (let index = 0; index < 4; index++) {
      states.push(`${element.dataset.search ?? "on"}/${element.dataset.compact ?? "full"}`);
      await frame();
    }

    return states;
  });
}

for (const textSize of ["default", "largest"] as const) {
  for (const room of HEADER_ROOMS) {
    for (const width of [1440, 1280, 760]) {
      test(`at the ${textSize} text size with a pane open, ${room.label ?? room.name}'s header fits at ${width}px`, async ({
        page,
      }, testInfo) => {
        await page.setViewportSize({ ...DESKTOP, width });
        const header = await openHeaderRoom(page, room, textSize);

        // The topic shows in a wide enough header; the narrowest leaves it to About.
        if (room.id === ROOM_IDS.launchPlanning) {
          await expect(header.getByRole("button", { name: "Channel topic, About" })).toBeVisible({
            visible: width > 760,
          });
        }

        if (room.id === ROOM_IDS.lounge) {
          await expect(header.locator(".huddle-stack")).toBeAttached();
          await expect(
            header.getByRole("status", { name: /of 25 voice participants$/ }),
          ).toBeVisible();
        }

        await openHeaderTool(page, /^Pinned messages/);
        const pane = page.locator("aside.right-pane");
        await expect(pane.getByRole("heading", { name: "Pinned messages" })).toBeVisible();
        await page.mouse.move(0, 0);
        await page.screenshot({
          path: testInfo.outputPath(`${textSize}-${room.label ?? room.name}-${width}.png`),
        });

        // The actions give way (the call button keeps its glyph, the search narrows or goes, the
        // pane buttons fold into ⋯), so the name shows beside them and no control ends past the
        // header.
        expect(await headerProblems(header)).toEqual([]);

        expect((await header.boundingBox())?.height).toBe(HEADER_HEIGHT[textSize]);

        // Send stays in the composer's card, beside "1 scheduled" in #general.
        const card = page.locator(".room .composer-card");
        const send = card.locator(".composer-send-group");
        const [cardBox, sendBox] = await Promise.all([card.boundingBox(), send.boundingBox()]);

        expect(sendBox).not.toBeNull();
        expect(cardBox).not.toBeNull();
        expect((sendBox?.x ?? 0) + (sendBox?.width ?? 0)).toBeLessThanOrEqual(
          (cardBox?.x ?? 0) + (cardBox?.width ?? 0),
        );
      });
    }
  }
}

// A stage with seven people in its call: its button and their faces widen the actions past what
// room.css's steps count, so the search must go before it would squeeze to a sliver.
const BUSY_STAGE: HeaderRoom = {
  id: ROOM_IDS.townHall,
  name: "Town Hall",
  settings: null,
  busy: true,
};

const SWEEP_ROOMS: readonly HeaderRoom[] = [
  BUSY_STAGE,
  ...HEADER_ROOMS.filter(
    (room) =>
      room.id === ROOM_IDS.general ||
      room.id === ROOM_IDS.lounge ||
      room.id === ROOM_IDS.announcements,
  ),
];

/** Whether the header shows its search and the call button's label. */
function shownParts(header: Locator): Promise<{ search: boolean; label: boolean }> {
  return header.evaluate((element) => ({
    search: element.querySelector(".header-search")?.checkVisibility() === true,
    label: element.querySelector(".huddle-launcher-label")?.checkVisibility() === true,
  }));
}

// Every 10 px, and the widths where the search's visibility once flipped every frame (Town Hall,
// the Lounge, at both sizes).
const SWEEP_WIDTHS = [
  ...new Set([
    ...Array.from({ length: 73 }, (_, step) => 1440 - step * 10),
    ...[1234, 1235, 1236, 1154, 1155, 1156, 1070, 1071, 1072, 1073, 1062, 1063, 1064],
    ...[1000, 1001, 1002, 930, 931, 932, 933, 934],
  ]),
].sort((a, b) => b - a);

for (const textSize of ["default", "largest"] as const) {
  for (const room of SWEEP_ROOMS) {
    test(`at the ${textSize} text size ${room.label ?? room.name}'s header fits from 1440 to 720px`, async ({
      page,
    }) => {
      await page.setViewportSize(DESKTOP);
      const header = await openHeaderRoom(page, room, textSize);

      if (room.id === ROOM_IDS.lounge || room.busy === true) {
        await expect(header.locator(".huddle-stack")).toBeAttached();
      }

      const problems: string[] = [];
      // The actions give way in order as the header narrows, the call button's label before the
      // search, and nothing that went comes back narrower.
      let gone = { search: false, label: false };

      for (const width of SWEEP_WIDTHS) {
        await page.setViewportSize({ ...DESKTOP, width });
        const states = await fitFrames(header);

        if (new Set(states).size > 1) {
          problems.push(`${width}px: ${states.join(" → ")}`);
        }

        const shown = await shownParts(header);

        if (shown.label && !shown.search) {
          problems.push(`${width}px: the search went before the call button's label`);
        }

        for (const part of ["search", "label"] as const) {
          if (gone[part] && shown[part]) {
            problems.push(`${width}px: the ${part} came back`);
          }
        }

        gone = { search: gone.search || !shown.search, label: gone.label || !shown.label };

        for (const problem of await headerProblems(header)) {
          problems.push(`${width}px: ${problem}`);
        }
      }

      expect(problems).toEqual([]);
    });
  }
}

test("an announcement room's name shows in full beside its tag at 1100px and wider", async ({
  page,
}) => {
  await page.setViewportSize(DESKTOP);
  const header = await openHeaderRoom(page, ANNOUNCEMENTS, "default");
  const name = header.locator(".room-title-name");

  for (const width of [1440, 1280, 1180, 1100]) {
    await page.setViewportSize({ ...DESKTOP, width });
    await expect(header.getByText("Announcement", { exact: true })).toBeVisible();
    await expect
      .poll(() => name.evaluate((element) => element.scrollWidth <= element.clientWidth))
      .toBe(true);
  }
});

test("the ⋯ menu closes when the header widens past it, focus on a pane button", async ({
  page,
}) => {
  await page.setViewportSize({ ...DESKTOP, width: 760 });
  await openApp(page, `r/${ROOM_IDS.general}`);
  const header = page.locator(".room-header");
  const more = header.getByRole("button", { name: "More" });

  await more.click();
  const menu = page.getByRole("menu", { name: "More" });
  await expect(menu.getByRole("menuitem", { name: "Threads" })).toBeVisible();
  await page.keyboard.press("ArrowDown");

  await page.setViewportSize(DESKTOP);
  await expect(menu).toHaveCount(0);
  await expect(more).toBeHidden();
  const threads = header.getByRole("button", { name: "Threads" });

  await expect(threads).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(threads).toBeFocused();
});

test("typing in the header's search doesn't re-measure the header", async ({ page }) => {
  await page.setViewportSize({ ...DESKTOP, width: 1280 });
  await openApp(page, `r/${ROOM_IDS.general}`);
  const header = page.locator(".room-header");
  const field = header.getByRole("combobox", { name: /search/i });

  await expect(field).toBeVisible();
  // The dropdown's first opening loads a font, and a font loading re-measures: open it once first.
  await field.click();
  await page.keyboard.type("l");
  await expect(header.locator(".search-dropdown")).toBeVisible();
  await page.keyboard.press("Escape");
  await page.mouse.click(0, 0);
  await page.evaluate(() => document.fonts.ready);

  // Every measurement sets the header's data-search (and data-label) while it measures.
  const records = await header.evaluateHandle((element) => {
    const seen: MutationRecord[] = [];
    const observer = new MutationObserver((batch) => seen.push(...batch));

    observer.observe(element, { attributes: true, attributeFilter: ["data-search", "data-label"] });

    return seen;
  });

  await field.click();
  await page.keyboard.type("launch", { delay: 30 });
  await expect(header.locator(".search-dropdown")).toBeVisible();
  await page.mouse.move(0, 0);

  expect(
    await records.evaluate(async (seen) => {
      await new Promise((resolve) => requestAnimationFrame(resolve));

      return seen.length;
    }),
  ).toBe(0);
});

test("focus returns to the wide header after a trip through the phone layout", async ({ page }) => {
  await page.setViewportSize({ ...DESKTOP, width: 760 });
  await openApp(page, `r/${ROOM_IDS.general}`);
  const header = page.locator(".room-header");

  await header.getByRole("button", { name: "More" }).click();
  await expect(page.getByRole("menu", { name: "More" })).toBeVisible();

  await page.setViewportSize(PHONE);
  await expect(page.getByRole("menu")).toHaveCount(0);
  await page.setViewportSize(DESKTOP);
  await expect(header.getByRole("button", { name: "Threads" })).toBeFocused();
});

test("the ⋯ menu stays closed after a trip through the phone layout", async ({ page }) => {
  await page.setViewportSize({ ...DESKTOP, width: 760 });
  await openApp(page, `r/${ROOM_IDS.general}`);
  const header = page.locator(".room-header");

  await header.getByRole("button", { name: "More" }).click();
  await expect(page.getByRole("menu", { name: "More" })).toBeVisible();

  await page.setViewportSize(PHONE);
  await expect(page.getByRole("menu")).toHaveCount(0);
  await page.setViewportSize({ ...DESKTOP, width: 760 });
  await expect(header.getByRole("button", { name: "More" })).toBeVisible();
  await expect(page.getByRole("menu")).toHaveCount(0);
});

test("at the largest text size a pane's header lines up with the room's", async ({ page }) => {
  await page.setViewportSize({ ...DESKTOP, width: 1280 });
  await openApp(page, "");
  await saveAccountAppearance(page, { textSize: "largest" });
  await openApp(page, `r/${ROOM_IDS.general}`);
  await openHeaderTool(page, /^Pinned messages/);
  const pane = page.locator("aside.right-pane");
  await expect(pane.locator(".pin-card")).toHaveCount(3);

  const heights = await Promise.all(
    [page.locator(".room-header"), pane.locator(".pane-header")].map(
      async (locator) => (await locator.boundingBox())?.height,
    ),
  );

  expect(heights[0]).toBe(HEADER_HEIGHT.largest);
  expect(heights[1]).toBe(heights[0]);
});

test("a topic saves, truncates in the header, and updates another open client", async ({
  page,
  context,
}) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `r/${ROOM}`);
  const other = await context.newPage();
  await other.setViewportSize(DESKTOP);
  await other.goto(`/app/r/${ROOM}`);
  await expect(other.getByRole("heading", { name: "launch-planning", level: 1 })).toBeVisible();
  await page.getByRole("link", { name: /room settings$/ }).click();
  const dialog = page.getByRole("dialog", { name: "Channel settings" });
  await dialog.getByRole("textbox", { name: "Topic" }).fill(TOPIC);
  await expect(dialog.getByText(`${Array.from(TOPIC).length} / 1024`)).toBeVisible();
  await dialog.getByRole("button", { name: "Save changes" }).click();
  await expect(dialog).toBeHidden();
  const topic = other.getByRole("button", { name: "Channel topic, About" });
  await expect(topic).toHaveText(TOPIC);
  expect(
    await topic.evaluate((element) => {
      const css = getComputedStyle(element);

      return {
        overflow: css.overflow,
        ellipsis: css.textOverflow,
        wrap: css.whiteSpace,
        truncated: element.scrollWidth > element.clientWidth,
      };
    }),
  ).toEqual({ overflow: "hidden", ellipsis: "ellipsis", wrap: "nowrap", truncated: true });
  await topic.click();
  const about = other.getByRole("region", { name: "About" });
  await expect(about).toContainText(TOPIC);
  await expect(about.locator("b")).toHaveCount(0);
  await expect(about.getByRole("link", { name: "https://example.com/docs" })).toHaveAttribute(
    "href",
    "https://example.com/docs",
  );
  await page.reload();
  await expect(page.getByRole("button", { name: "Channel topic, About" })).toHaveText(TOPIC);
});

test("phones show the topic only in About and clearing removes it", async ({ page }) => {
  await page.setViewportSize(PHONE);
  await openApp(page, `r/${ROOM}/settings`);
  let dialog = page.getByRole("dialog", { name: "Channel settings" });
  await dialog.getByRole("textbox", { name: "Topic" }).fill("Phone topic");
  await dialog.getByRole("button", { name: "Save changes" }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByRole("button", { name: "Channel topic, About" })).toHaveCount(0);
  await page.getByRole("button", { name: "launch-planning, details" }).click();
  await expect(page.getByRole("region", { name: "About" })).toContainText("Phone topic");
  await page.getByRole("link", { name: "Channel settings", exact: true }).click();
  dialog = page.getByRole("dialog", { name: "Channel settings" });
  await expect(dialog.getByRole("textbox", { name: "Topic" })).toHaveValue("Phone topic");
  await dialog.getByRole("textbox", { name: "Topic" }).fill("  ");
  await dialog.getByRole("button", { name: "Save changes" }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByRole("region", { name: "About" })).toHaveCount(0);
});
