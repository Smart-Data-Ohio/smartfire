import {
  DESKTOP,
  expect,
  expectNoHorizontalOverflow,
  openApp,
  PHONE,
  ROOM_IDS,
  test,
} from "./support.ts";

const ROOM = ROOM_IDS.launchPlanning;
// Six settings tabs are wider than the dialog: their strip scrolls sideways on purpose.
const TAB_STRIP = ".room-form .t-tabs";

for (const viewport of [DESKTOP, PHONE]) {
  test(`a channel webhook is created, shown once, tested and deleted at ${viewport.width}px`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize(viewport);
    await openApp(page, `r/${ROOM}/settings`);

    const dialog = page.getByRole("dialog", { name: "Channel settings" });

    await dialog.getByRole("tab", { name: "Webhooks" }).click();
    await expect(dialog.getByText("No webhooks yet.")).toBeVisible();

    await dialog.getByLabel("Name").fill("Deployments");
    await dialog.getByRole("button", { name: "Create webhook" }).click();

    const notice = dialog.getByRole("status", { name: "Deployments's webhook URL" });

    await expect(notice).toContainText("You won't see this URL again");
    await expect(notice.locator("code")).toHaveText(
      /^http:\/\/127\.0\.0\.1\/api\/webhooks\/\d+\/[0-9a-f]{48}$/,
    );

    const url = (await notice.locator("code").textContent()) ?? "";

    await expectNoHorizontalOverflow(page, { allowScroll: TAB_STRIP });
    await page.screenshot({ path: testInfo.outputPath(`webhook-url-${viewport.width}.png`) });

    await notice.getByRole("button", { name: "Done" }).click();
    await expect(notice).toBeHidden();
    await expect(dialog.getByRole("listitem", { name: "Deployments" })).toContainText("Created by");
    expect(await page.content()).not.toContain(url);

    await dialog.getByRole("button", { name: "Send a test message from Deployments" }).click();
    await expect(page.getByText("Deployments posted a test message")).toBeVisible();
    await expectNoHorizontalOverflow(page, { allowScroll: TAB_STRIP });
    await page.screenshot({ path: testInfo.outputPath(`webhook-list-${viewport.width}.png`) });

    await dialog.getByRole("button", { name: "Delete Deployments" }).click();
    await page
      .getByRole("alertdialog", { name: "Delete Deployments?" })
      .getByRole("button", { name: "Delete webhook" })
      .click();
    await expect(dialog.getByText("No webhooks yet.")).toBeVisible();

    await dialog.getByRole("button", { name: "Cancel" }).click();
    await expect(dialog).toBeHidden();

    // The test post sits in the room under the webhook's name with its APP tag.
    const posted = page
      .locator("article.message")
      .filter({ hasText: "This is a test webhook message." });

    await expect(posted.locator(".message-author")).toHaveText("Deployments");
    await expect(posted.locator(".agent-badge")).toHaveText("App");
    await page.screenshot({ path: testInfo.outputPath(`webhook-message-${viewport.width}.png`) });
  });
}
