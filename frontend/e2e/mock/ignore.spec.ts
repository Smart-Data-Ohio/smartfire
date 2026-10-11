import {
  expect,
  expectNoHorizontalOverflow,
  openApp,
  PHONE_SMALL,
  postMessage,
  ROOM_IDS,
  test,
  USER_IDS,
} from "./support.ts";

test("ignoring someone collapses their messages, and settings lists them until you stop", async ({
  page,
  request,
}) => {
  for (const markdown of ["First from Maya", "Second from Maya"]) {
    await postMessage(request, { roomId: ROOM_IDS.quiet, userId: USER_IDS.maya, markdown });
  }

  await openApp(page, `r/${ROOM_IDS.quiet}`);

  const timeline = page.getByRole("log", { name: "Messages" });

  await expect(timeline.getByText("Second from Maya")).toBeVisible();

  // In the app, so the room's messages stay held: ignoring has to collapse them in place.
  await page.getByRole("button", { name: "Your account" }).click();
  await page.getByRole("menuitem", { name: "People", exact: true }).click();
  await page.getByRole("link", { name: "Maya Okafor", exact: true }).click();
  await page.getByRole("button", { name: "Ignore", exact: true }).click();

  const confirm = page.getByRole("alertdialog", { name: "Ignore Maya Okafor?" });

  await expect(confirm).toContainText("won't be able to start new DMs with you");
  await confirm.getByRole("button", { name: "Ignore Maya Okafor" }).click();
  await expect(page.getByRole("button", { name: "Stop ignoring" })).toBeVisible();

  await page.goBack();
  await page.goBack();
  await expect(timeline.getByText("2 messages from someone you ignore")).toBeVisible();
  await expect(timeline.getByText("Second from Maya")).toBeHidden();

  await timeline.getByRole("button", { name: "Show" }).click();
  await expect(timeline.getByText("First from Maya")).toBeVisible();
  await expect(timeline.getByText("Second from Maya")).toBeVisible();
  await timeline.getByRole("button", { name: "Hide" }).click();
  await expect(timeline.getByText("Second from Maya")).toBeHidden();

  // A fresh load collapses them too: the server marks them, and nothing of the reveal was kept.
  await page.setViewportSize(PHONE_SMALL);
  await openApp(page, `r/${ROOM_IDS.quiet}`);
  await expect(timeline.getByText("2 messages from someone you ignore")).toBeVisible();
  await expectNoHorizontalOverflow(page);

  await openApp(page, "settings/ignored");

  const stop = page.getByRole("button", { name: "Stop ignoring Maya Okafor" });

  await expect(page.getByRole("link", { name: "Maya Okafor" })).toBeVisible();
  await expectNoHorizontalOverflow(page);

  // The write's `blocks.updated` echo refetches the list; she must stay gone after it lands.
  const echo = page.waitForResponse(
    (response) =>
      response.request().method() === "GET" && response.url().endsWith("/api/v1/blocks"),
  );

  await stop.click();
  await expect(page.getByText("You're not ignoring anyone")).toBeVisible();
  await echo;
  await expect(page.getByRole("link", { name: "Maya Okafor" })).toHaveCount(0);
  await expect(page.getByText("You're not ignoring anyone")).toBeVisible();
});
