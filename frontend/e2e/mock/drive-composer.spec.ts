import type { Locator, Page } from "@playwright/test";
import {
  DESKTOP,
  expect,
  expectTouchTargets,
  PHONE_TOUCH,
  ROOM_IDS,
  test,
  USER_IDS,
} from "./support.ts";

async function openDrive(page: Page) {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM_IDS.general}`);
  await page.getByRole("log", { name: "Messages" }).waitFor();
  await page.getByRole("button", { name: "Attach and more" }).click();
  await page.getByRole("menuitem", { name: "From Google Drive" }).click();

  return page.getByRole("combobox", { name: "Search Drive files" });
}

const MAYA = { id: USER_IDS.maya, name: "Maya Okafor", email: "maya@37signals.com" };

const JONAH = { id: USER_IDS.jonah, name: "Jonah Lindqvist", email: "jonah@37signals.com" };

function shareSurface(page: Page): Locator {
  return page
    .getByRole("dialog", { name: "Share a Drive file" })
    .or(page.getByRole("alertdialog", { name: "Share a Drive file" }));
}

function shareBody(
  outcome: string,
  extra: {
    readonly blocked?: string | null;
    readonly changedIds?: readonly number[];
    readonly recipients?: readonly {
      readonly id: number;
      readonly name: string;
      readonly email: string;
    }[];
    readonly results?: readonly {
      readonly recipient: { readonly id: number; readonly name: string; readonly email: string };
      readonly status: string;
      readonly reason: string | null;
    }[];
  } = {},
) {
  return {
    outcome,
    fileId: "1RoadmapQ4draft",
    blocked: extra.blocked ?? null,
    changedIds: extra.changedIds ?? [],
    recipients: extra.recipients ?? [],
    results: extra.results ?? [],
  };
}

async function reviewRoadmap(page: Page): Promise<Locator> {
  const search = await openDrive(page);

  await search.fill("roadmap");
  await page.getByRole("option", { name: /Q4 roadmap/ }).click();

  const dialog = shareSurface(page);

  await expect(dialog).toBeVisible();

  return dialog;
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

  test("Picker reaches an existing file outside the stored app grant", async ({ page }) => {
    const googleRequests: string[] = [];
    page.on("request", (request) => {
      if (/^https:\/\/(apis|accounts)\.google\.com\//.test(request.url()))
        googleRequests.push(request.url());
    });
    const search = await openDrive(page);
    await search.fill("private");
    await expect(page.getByText("No files found")).toBeVisible();
    await page.getByRole("button", { name: "Choose from Google Drive" }).click();
    await page
      .getByRole("dialog", { name: "Choose a Drive file" })
      .getByRole("button", { name: "Existing private plan" })
      .click();
    await page.getByRole("button", { name: "Attach only" }).click();
    await expect(page.getByRole("button", { name: "Remove Existing private plan" })).toBeVisible();
    await page.getByRole("textbox", { name: "Message #general" }).fill("Chosen in Picker");
    await page.getByRole("button", { name: "Send message" }).click();
    const row = page.locator("[data-message-row]", { hasText: "Chosen in Picker" });
    await expect(row.getByRole("link", { name: /Google Drive file/ })).toHaveAttribute(
      "href",
      "https://drive.google.com/open?id=3ExistingDriveFile",
    );
    await expect(page.getByRole("button", { name: "Remove Existing private plan" })).toHaveCount(0);
    expect(googleRequests).toEqual([]);
  });

  test("recipient failures show an error and retry restores sharing", async ({ page }) => {
    let attempts = 0;
    await page.route("**/api/v1/rooms/*/drive/recipients", async (route) => {
      attempts += 1;

      if (attempts === 1) await route.fulfill({ status: 503, body: "Unavailable" });
      else await route.continue();
    });
    const dialog = await reviewRoadmap(page);
    await expect(dialog.getByRole("alert")).toContainText("Couldn't load recipients");
    await expect(
      dialog.getByRole("button", { name: "Grant view access and attach" }),
    ).toBeDisabled();
    await dialog.getByRole("button", { name: "Retry" }).click();
    await dialog.getByRole("checkbox", { name: /Maya Okafor/ }).check();
    await expect(
      dialog.getByRole("button", { name: "Grant view access and attach" }),
    ).toBeEnabled();
    await expect(dialog.getByRole("alert")).toHaveCount(0);
    expect(attempts).toBe(2);
  });

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

  test("a grant in flight cannot be dismissed", async ({ page }) => {
    let release = () => {};

    const held = new Promise<void>((resolve) => {
      release = resolve;
    });

    await page.route("**/api/v1/rooms/*/drive/shares", async (route) => {
      await held;
      await route.fulfill({
        json: shareBody("shared", {
          results: [{ recipient: MAYA, status: "granted", reason: null }],
        }),
      });
    });

    const dialog = await reviewRoadmap(page);

    await dialog.getByRole("checkbox", { name: /Maya Okafor/ }).check();
    await dialog.getByRole("button", { name: "Grant view access and attach" }).click();

    const granting = page.getByRole("alertdialog", { name: "Share a Drive file" });

    await expect(granting.getByRole("button", { name: "Cancel" })).toBeDisabled();
    await expect(granting.getByRole("button", { name: "Attach only" })).toBeDisabled();
    await expect(granting.getByRole("button", { name: "Close" })).toHaveCount(0);
    await expect(
      granting.getByRole("button", { name: "Grant view access and attach" }),
    ).toHaveAttribute("aria-busy", "true");

    await page.keyboard.press("Escape");
    await page.mouse.click(8, 8);
    await expect(granting).toBeVisible();

    release();
    await expect(page.getByRole("button", { name: "Remove Q4 roadmap" })).toBeVisible();
    await expect(shareSurface(page)).toHaveCount(0);
  });

  test("an address change asks for the selection again", async ({ page }) => {
    const emails: string[] = [];

    await page.route("**/api/v1/rooms/*/drive/shares", async (route) => {
      // SAFETY: this route's body is the share JSON the composer posts (`recipients[].email`).
      const body = route.request().postDataJSON() as {
        recipients?: { email?: string }[];
      };

      emails.push(body.recipients?.[0]?.email ?? "");

      if (emails.length === 1) {
        await route.fulfill({
          json: shareBody("confirmation_required", {
            changedIds: [USER_IDS.maya],
            recipients: [{ ...MAYA, email: "maya.changed@37signals.com" }, JONAH],
          }),
        });

        return;
      }

      await route.fulfill({
        json: shareBody("shared", {
          results: [
            {
              recipient: { ...MAYA, email: "maya.changed@37signals.com" },
              status: "granted",
              reason: null,
            },
          ],
        }),
      });
    });

    const dialog = await reviewRoadmap(page);
    const maya = dialog.getByRole("checkbox", { name: /Maya Okafor/ });

    await maya.check();
    await dialog.getByRole("button", { name: "Grant view access and attach" }).click();
    await expect(dialog.getByText(/Recipient details changed since this review/)).toBeVisible();
    await expect(dialog.getByText("maya.changed@37signals.com")).toBeVisible();
    await expect(maya).not.toBeChecked();
    await expect(
      dialog.getByRole("button", { name: "Grant view access and attach" }),
    ).toBeDisabled();

    await maya.check();
    await dialog.getByRole("button", { name: "Grant view access and attach" }).click();
    await expect(page.getByRole("button", { name: "Remove Q4 roadmap" })).toBeVisible();
    expect(emails[1]).toBe("maya.changed@37signals.com");
  });

  test("a partial grant stays on screen until it is dismissed", async ({ page }) => {
    await page.route("**/api/v1/rooms/*/drive/shares", (route) =>
      route.fulfill({
        json: shareBody("shared", {
          results: [
            { recipient: MAYA, status: "granted", reason: null },
            { recipient: JONAH, status: "failed", reason: "denied" },
          ],
        }),
      }),
    );

    const dialog = await reviewRoadmap(page);

    await dialog.getByRole("checkbox", { name: /Maya Okafor/ }).check();
    await dialog.getByRole("checkbox", { name: /Jonah Lindqvist/ }).check();
    await dialog.getByRole("button", { name: "Grant view access and attach" }).click();
    await expect(page.getByRole("button", { name: "Remove Q4 roadmap" })).toBeVisible();
    await expect(dialog.getByText("1 of 2 recipients have access.")).toBeVisible();
    await expect(dialog.getByText("granted view access")).toBeVisible();
    await expect(dialog.getByText("not granted (refused by Google)")).toBeVisible();
    await dialog.getByRole("button", { name: "Done" }).click();
    await expect(dialog.getByText("not granted (refused by Google)")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Remove Q4 roadmap" })).toBeVisible();
  });

  test("a folder and a shortcut cannot be granted", async ({ page }) => {
    await page.route("**/api/v1/drive/files**", (route) =>
      route.fulfill({
        json: {
          files: [
            {
              id: "folderPlans0001",
              name: "Plans folder",
              kind: "folder",
              modifiedAt: null,
              owner: "Maya Okafor",
              url: "https://drive.google.com/drive/folders/folderPlans0001",
            },
            {
              id: "shortcutPlans01",
              name: "Plans shortcut",
              kind: "shortcut",
              modifiedAt: null,
              owner: "Maya Okafor",
              url: "https://drive.google.com/open?id=shortcutPlans01",
            },
          ],
        },
      }),
    );

    const search = await openDrive(page);

    await search.fill("plans");

    for (const name of [/Plans folder/, /Plans shortcut/]) {
      await page.getByRole("option", { name }).click();

      const dialog = shareSurface(page);

      await dialog.getByRole("checkbox", { name: /Maya Okafor/ }).check();
      await expect(dialog.getByText(/Folders and shortcuts cannot be shared/)).toBeVisible();
      await expect(
        dialog.getByRole("button", { name: "Grant view access and attach" }),
      ).toBeDisabled();
      await dialog.getByRole("button", { name: "Cancel" }).click();
    }
  });

  test("an existing-access recipient is reported", async ({ page }) => {
    await page.route("**/api/v1/rooms/*/drive/shares", (route) =>
      route.fulfill({
        json: shareBody("shared", {
          results: [{ recipient: MAYA, status: "already", reason: null }],
        }),
      }),
    );

    const dialog = await reviewRoadmap(page);

    await dialog.getByRole("checkbox", { name: /Maya Okafor/ }).check();
    await dialog.getByRole("button", { name: "Grant view access and attach" }).click();
    await expect(dialog.getByText("Everyone selected already has access.")).toBeVisible();
    await expect(dialog.getByText("maya@37signals.com · already had access")).toBeVisible();
    await dialog.getByRole("button", { name: "Done" }).click();
    await expect(shareSurface(page)).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Remove Q4 roadmap" })).toBeVisible();
  });
});
