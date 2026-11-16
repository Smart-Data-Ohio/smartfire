import {
  DESKTOP,
  expect,
  openApp,
  openHeaderTool,
  PHONE,
  ROOM_IDS,
  saveAccountAppearance,
  test,
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

// #general has no topic and one scheduled message; launch-planning is given a topic; the Lounge
// has people in its call and is given a voice limit, so its header shows their faces and "2/25".
const HEADER_ROOMS = [
  { id: ROOM_IDS.general, name: "general", settings: null },
  {
    id: ROOM_IDS.launchPlanning,
    name: "launch-planning",
    settings: { field: "Topic", value: "Ship the launch checklist by Thursday" },
  },
  { id: ROOM_IDS.lounge, name: "Lounge", settings: { field: "Voice user limit", value: "25" } },
] as const;

for (const textSize of ["default", "largest"] as const) {
  for (const room of HEADER_ROOMS) {
    for (const width of [1280, 760]) {
      test(`at the ${textSize} text size with a pane open, ${room.name}'s header fits at ${width}px`, async ({
        page,
      }, testInfo) => {
        await page.setViewportSize({ ...DESKTOP, width });
        await openApp(page, "");
        await saveAccountAppearance(page, { textSize });

        if (room.settings === null) {
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
          path: testInfo.outputPath(`${textSize}-${room.name}-${width}.png`),
        });

        // The actions give way (the call button keeps its glyph, the search narrows or goes, the
        // pane buttons fold into ⋯), so the name shows and no control ends past the header.
        expect(
          await header.evaluate((element) => {
            const right = element.getBoundingClientRect().right;
            const name = element.querySelector(".room-title-name");

            if (name === null) throw new Error("Missing room name");

            return {
              named: name.getBoundingClientRect().width > 0,
              spilled: Array.from(
                element.querySelectorAll(".page-header-actions :is(button, a, input)"),
                (control) => control.getBoundingClientRect(),
              )
                .filter((box) => box.width > 0 && box.right > right + 0.5)
                .map((box) => Math.round(box.right - right)),
              contained: element.scrollWidth <= element.clientWidth,
            };
          }),
        ).toEqual({ named: true, spilled: [], contained: true });

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
