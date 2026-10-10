import type { Page } from "@playwright/test";
import { animatedIconGif } from "../../mock/s2/assets.ts";
import { MESSAGE_IDS } from "../../mock/s2/seed.ts";
import {
  DESKTOP,
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  matrix,
  openApp,
  PHONE_SMALL,
  PHONE_TOUCH,
  ROOM_IDS,
  SHOTS,
  shot,
  test,
} from "./support.ts";

/** Opens an admin section (`""` for the workspace) with motion reduced. */
async function openAdmin(page: Page, section: string, theme: "light" | "dark" = "light") {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(`/app/admin${section === "" ? "" : `/${section}`}`);
  await page.locator(".settings-page h1").waitFor();
}

function nav(page: Page) {
  return page.getByRole("navigation", { name: "Workspace sections" });
}

/** Lets entrance animations finish so a shot is settled. */
async function settle(page: Page): Promise<void> {
  await page.mouse.move(0, 0);
  await page.evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        .filter((animation) => animation.effect?.getComputedTiming().iterations !== Infinity)
        .map((animation) => animation.finished.catch(() => animation)),
    ),
  );
}

// Screenshots only: the tests below cover each section and the phone list's way back.
if (SHOTS) {
  matrix("the admin sections", async ({ page, theme, phone }) => {
    // On phones the root is the list of sections: the workspace is pushed from it, at its own address.
    await openAdmin(page, phone ? "workspace" : "", theme);

    await expect(page.getByRole("heading", { level: 1, name: "Workspace" })).toBeVisible();
    await settle(page);
    await shot(page, "admin-workspace", theme);

    for (const [link, heading, name] of [
      ["People", "People", "admin-people"],
      ["Workspace icons", "Workspace icons", "admin-icons"],
      ["Custom styles", "Custom CSS", "admin-styles"],
      ["Audit log", "Audit log", "admin-audit-log"],
      ["Integration health", "Integration health", "admin-integrations"],
    ] as const) {
      if (phone) {
        await page.getByRole("link", { name: "Back to Workspace" }).click();
      }

      await nav(page).getByRole("link", { name: link }).click();
      await expect(page.getByRole("heading", { level: 1, name: heading })).toBeVisible();
      await settle(page);
      await shot(page, name, theme);
    }
  });
}

test("the user menu opens the workspace in place", async ({ page }) => {
  await openApp(page, "");

  await page.getByRole("button", { name: "Your account" }).click();
  await page.getByRole("menuitem", { name: "Workspace and people" }).click();

  await expect(page).toHaveURL(/\/app\/admin$/);
  await expect(page.getByRole("heading", { level: 1, name: "Workspace" })).toBeVisible();
  await expect(page.getByRole("complementary", { name: "Conversations" })).toBeVisible();
  await expect(page).toHaveTitle(/Workspace · Smartfire/);
});

test("renaming the workspace saves and lands in the audit log", async ({ page }) => {
  await openAdmin(page, "");

  const name = page.getByRole("textbox", { name: "Name" });

  await name.fill("Smart Data Labs");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByText("Workspace saved")).toBeVisible();

  await nav(page).getByRole("link", { name: "Audit log" }).click();
  await expect(
    page.getByRole("cell", { name: "name: Smart Data → Smart Data Labs" }),
  ).toBeVisible();
});

test("a role change and a removal update the people list", async ({ page }) => {
  await openAdmin(page, "people");

  const maya = page.locator(".admin-person", { hasText: "Maya" });

  await maya.getByRole("switch").click();
  await expect(
    page.getByRole("region", { name: "Administrators" }).getByText("maya@smartdata.example"),
  ).toBeVisible();
  // The row moved lists and remounted; its switch keeps the focus.
  await expect(maya.getByRole("switch")).toBeFocused();

  await maya.getByRole("button", { name: /Remove Maya/ }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Remove" }).click();
  await expect(page.getByText("was removed")).toBeVisible();
  await expect(page.locator(".admin-person", { hasText: "Maya" })).toHaveCount(0);
  // The removed row was the dialog's return target; a neighbouring row takes the focus.
  await expect(page.locator(".admin-person:focus, .admin-person :focus")).toHaveCount(1);
});

test("deleting an icon keeps the focus in the list", async ({ page }) => {
  await openAdmin(page, "icons");

  await page.getByRole("button", { name: "Delete :smartdata:" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Delete" }).click();

  await expect(page.getByText("No workspace icons yet.")).toBeVisible();
  await expect(page.locator(".admin-focus-root")).toBeFocused();
});

test("an animated icon uploaded here plays in a reaction, and rests on its still under reduced motion", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto("/app/admin/icons");
  await expect(page.getByText("Animated icons: 0 of 250 used.")).toBeVisible();

  await page.getByRole("textbox", { name: "Name" }).fill("dance");
  await page.getByRole("textbox", { name: "Title" }).fill("Dance");
  await page.getByLabel("Icon file").setInputFiles({
    name: "dance.gif",
    mimeType: "image/gif",
    buffer: Buffer.from(animatedIconGif()),
  });
  await page.getByRole("button", { name: "Upload icon" }).click();

  const listed = page.locator(".settings-list-row", { hasText: ":dance:" });

  await expect(listed.getByText("Animated")).toBeVisible();
  await expect(page.getByText("Animated icons: 1 of 250 used.")).toBeVisible();

  await page.goto(`/app/r/${ROOM_IDS.general}/m/${MESSAGE_IDS.generalReactions}`);

  const target = page.locator(`[data-message-id="${MESSAGE_IDS.generalReactions}"]`);
  const bar = target.getByRole("toolbar", { name: "Message actions" });

  await expect(async () => {
    await page.mouse.move(0, 0);
    await target.hover();
    await expect(bar).toBeVisible({ timeout: 1000 });
  }).toPass();
  await bar.getByRole("button", { name: "Add reaction" }).click();
  await page.getByRole("combobox", { name: "Search emoji" }).fill("dance");
  await page.getByRole("option", { name: "Dance", exact: true }).click();

  const image = target.getByRole("button", { name: /^Dance:/ }).locator("img");

  // The original plays: the GIF itself, all its frames, not the still.
  await expect(image).toHaveAttribute("src", "/icons/dance");
  await expect
    .poll(() =>
      image.evaluate((element: HTMLImageElement) => element.complete && element.naturalWidth),
    )
    .toBe(64);

  const served = await page.request.get("/icons/dance");

  expect(served.headers()["content-type"]).toBe("image/gif");

  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(image).toHaveAttribute("src", "/icons/dance?still=1");
});

test("custom CSS waits out the password confirmation", async ({ page, request }) => {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/lapse-sudo", { headers: { "X-CSRF-Token": state.csrfToken } });
  await page.route("**/sudo/new", (route) =>
    route.fulfill({
      status: 200,
      contentType: "text/html",
      body: "<h1>Confirm your password</h1>",
    }),
  );
  await openAdmin(page, "styles");

  await page.getByRole("textbox", { name: "Custom CSS" }).fill("body { color: red; }");
  await page.getByRole("button", { name: "Save changes" }).click();
  await expect(page).toHaveURL(/\/sudo\/new$/);

  await openAdmin(page, "styles");

  await expect(page.getByRole("textbox", { name: "Custom CSS" })).toHaveValue(
    "body { color: red; }",
  );
  await expect(page.getByText("Your unsaved CSS is back")).toBeVisible();
});

test("the audit log filters by action", async ({ page }) => {
  await openAdmin(page, "audit-log");

  await page.getByLabel("Action").selectOption("user.role.change");
  await page.getByRole("button", { name: "Filter" }).click();

  await expect(page.getByText("No audit entries match these filters.")).toBeVisible();
  await page.getByRole("button", { name: "Clear" }).click();
  await expect(page.getByRole("cell", { name: "account.settings.change" })).toBeVisible();
});

test.describe("on a phone", () => {
  test.use(PHONE_TOUCH);

  test("the workspace is a list of sections, each pushed over it with a way back", async ({
    page,
  }) => {
    await openApp(page, "admin");

    const list = nav(page);

    await expect(list.getByRole("link")).toHaveCount(8);
    await expectTouchTargets(page, ".settings-nav");
    await expectNoHorizontalOverflow(page);
    await shot(page, "admin-list", "light");

    await list.getByRole("link", { name: "People" }).click();
    await expect(page).toHaveURL(/\/app\/admin\/people$/);
    await expect(list).toBeHidden();

    await page.getByRole("link", { name: "Back to Workspace" }).click();
    await expect(page).toHaveURL(/\/app\/admin$/);
    await expect(list).toBeVisible();
  });

  test("people rows stack, their changes in a ⋯ menu, nothing overlapping", async ({ page }) => {
    await openAdmin(page, "people");
    await page.locator(".admin-person").first().waitFor();

    await expectNoHorizontalOverflow(page);
    await expectTouchTargets(page, ".settings");
    await shot(page, "admin-people-rows", "light");

    const collisions = await page.locator(".admin-person").evaluateAll((rows) =>
      rows.flatMap((row) => {
        const parts = [
          ...row.querySelectorAll<HTMLElement>(
            ".admin-person-name-text, .settings-badge, .admin-person-email, .admin-person-actions > *",
          ),
        ];

        return parts.flatMap((part, index) =>
          parts.slice(index + 1).flatMap((other) => {
            const a = part.getBoundingClientRect();
            const b = other.getBoundingClientRect();

            const hit =
              a.left < b.right - 0.5 &&
              b.left < a.right - 0.5 &&
              a.top < b.bottom - 0.5 &&
              b.top < a.bottom - 0.5;

            return hit ? [`${part.className} × ${other.className}`] : [];
          }),
        );
      }),
    );

    expect(collisions, "overlapping parts of a person row").toEqual([]);

    // Your own row says "My settings" in words, not as a floating button.
    const you = page.locator(".admin-person", { hasText: "Riel" });

    await expect(you.getByRole("link", { name: "My settings" })).toBeVisible();
    // The desktop row's buttons are gone: the role and the rest are in the menu.
    await expect(page.getByRole("switch")).toHaveCount(0);

    const maya = page.locator(".admin-person", { hasText: "Maya" });

    await maya.getByRole("button", { name: "Role and access for Maya Okafor" }).click();

    const menu = page.getByRole("menu", { name: "Maya Okafor: role and access" });

    await expect(menu.getByRole("menuitem", { name: "Remove from workspace" })).toBeVisible();
    await menu.getByRole("menuitem", { name: "Make an administrator" }).click();
    await expect(
      page.getByRole("region", { name: "Administrators" }).getByText("maya@smartdata.example"),
    ).toBeVisible();
  });

  test("the audit log is a stack of cards", async ({ page }) => {
    await openAdmin(page, "audit-log");

    const entry = page.getByRole("cell", { name: "account.settings.change" });

    await expect(entry).toBeVisible();
    await expectNoHorizontalOverflow(page);

    // A card: the action heads it, the other cells are labelled lines under it, all on screen.
    const card = page.locator(".admin-audit-table tbody tr").first();

    const cells = await card.locator("td").evaluateAll((tds) =>
      tds.map((td) => {
        const box = td.getBoundingClientRect();

        return { right: box.right, top: box.top, title: td.hasAttribute("data-title") };
      }),
    );

    expect(Math.max(...cells.map((cell) => cell.right))).toBeLessThanOrEqual(
      PHONE_TOUCH.viewport.width,
    );
    expect(cells.find((cell) => cell.title)?.top).toBe(Math.min(...cells.map((cell) => cell.top)));
    await shot(page, "admin-audit-cards", "light");

    // Still a table to a screen reader: its headers hidden only visually, each value under one.
    const table = page.getByRole("table");

    await expect(table.getByRole("columnheader")).toHaveText([
      "Time",
      "Actor",
      "Action",
      "Target",
      "Changes",
      "IP",
    ]);
    await expect(table.locator("thead")).toHaveCSS("clip-path", "inset(50%)");
    await expect(table.getByRole("row").nth(1).getByRole("cell")).toHaveCount(6);
    await expect(table).toMatchAriaSnapshot(`
      - table:
        - rowgroup:
          - row:
            - columnheader "Time"
            - columnheader "Actor"
            - columnheader "Action"
            - columnheader "Target"
            - columnheader "Changes"
            - columnheader "IP"
        - rowgroup:
          - row:
            - cell
            - cell
            - cell "account.settings.change"
    `);
  });

  test("the import runs are cards, and the run page goes back to Slack import", async ({
    page,
  }) => {
    await openAdmin(page, "slack/runs");

    await expect(page.getByRole("link", { name: "#1" })).toBeVisible();
    await expectNoHorizontalOverflow(page);
    await expectTouchTargets(page, ".settings-header");

    await page.getByRole("link", { name: "Back to Slack import" }).click();
    await expect(page).toHaveURL(/\/app\/admin\/slack$/);
  });

  test("the workspace profile preview fits the screen", async ({ page }) => {
    await openAdmin(page, "workspace");

    const preview = page.getByRole("figure", { name: "Preview" });

    await expect(preview).toBeVisible();
    await expect(preview.locator(".profile-preview-lines")).toBeHidden();
    expect((await preview.boundingBox())?.height).toBeLessThan(140);
    await expectNoHorizontalOverflow(page);
    await shot(page, "admin-workspace-preview", "light");
  });
});

test.describe("across the phone breakpoint", () => {
  test.use({ viewport: DESKTOP });

  test("workspace drafts survive the window narrowing and widening again", async ({ page }) => {
    // The root's own section, which the phone list covers.
    await openAdmin(page, "");

    const name = page.getByRole("textbox", { name: "Name" });

    await name.fill("Smart Data, unsaved");
    await page.setViewportSize(PHONE_SMALL);
    await expect(nav(page).getByRole("link", { name: "Audit log" })).toBeVisible();
    await page.setViewportSize(DESKTOP);
    await expect(name).toHaveValue("Smart Data, unsaved");

    // A section page, pushed on phones.
    await nav(page).getByRole("link", { name: "Custom styles" }).click();

    const css = page.getByRole("textbox", { name: "Custom CSS" });

    await css.fill("a { color: teal; }");
    await page.setViewportSize(PHONE_SMALL);
    await expect(page.getByRole("link", { name: "Back to Workspace" })).toBeVisible();
    await expect(css).toHaveValue("a { color: teal; }");
    await page.setViewportSize(DESKTOP);
    await expect(css).toHaveValue("a { color: teal; }");
  });

  test("the browser's Back and a reload keep the phone pages in place", async ({ page }) => {
    await page.setViewportSize(PHONE_SMALL);
    await openApp(page, "admin");

    await nav(page).getByRole("link", { name: "Workspace", exact: true }).click();
    await expect(page).toHaveURL(/\/app\/admin\/workspace$/);

    await page.reload();
    await expect(page.getByRole("heading", { level: 2, name: "Workspace profile" })).toBeVisible();
    await expect(
      page.locator(".settings-header").getByText("Workspace", { exact: true }),
    ).toBeVisible();

    await page.goBack();
    await expect(page).toHaveURL(/\/app\/admin$/);
    await expect(nav(page).getByRole("link", { name: "People" })).toBeVisible();
  });
});
