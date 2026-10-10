import { THREAD_IDS } from "../../mock/s2/seed.ts";
import { CARD_IDS } from "../../mock/s3/cards.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import { expect, PHONE_SMALL, ROOM_IDS, syncWelcomed, test } from "./support.ts";

test("ending a poll updates an observer's final results live", async ({ page, context }) => {
  const observer = await context.newPage();
  const path = `/app/r/${CARD_IDS.room}/m/${CARD_IDS.messages.pollOpen}`;
  const welcomed = syncWelcomed(observer);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await observer.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(path);
  await observer.goto(path);
  await welcomed;
  const selector = `[data-message-row][data-message-id="${CARD_IDS.messages.pollOpen}"]`;
  const card = page.locator(selector).first().getByRole("region", { name: "Poll" });
  const observed = observer.locator(selector).first().getByRole("region", { name: "Poll" });
  await expect(observed.getByRole("button", { name: "Vote", exact: true })).toBeVisible();
  await card.getByRole("button", { name: "End poll now" }).click();
  await expect(card.getByText("Closed", { exact: true })).toBeVisible();
  await expect(observed.getByText("Closed", { exact: true })).toBeVisible();
  await expect(observed.locator(".poll-total")).toContainText("Final results");
  await expect(observed.getByRole("radio")).toHaveCount(0);
  await expect(observed.getByRole("button", { name: "End poll now" })).toHaveCount(0);
  await observer.close();
});

for (const viewport of [{ width: 1440, height: 900 }, PHONE_SMALL]) {
  test(`creates and ends a poll inside a thread at ${viewport.width}px`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize(viewport);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(`/app/r/${ROOM_IDS.general}/t/${THREAD_IDS.generalActive}`);
    const pane = page.locator("aside.right-pane");
    await pane.getByRole("textbox", { name: "Reply…" }).waitFor();
    await pane.getByRole("button", { name: "Attach and more" }).click();
    await page.getByRole("menuitem", { name: "Create a poll" }).click();
    const dialog = page.getByRole("dialog", { name: "Create a poll" });
    await dialog.getByRole("textbox", { name: "Question" }).fill("Thread lunch?");
    await dialog.getByRole("textbox", { name: "Option 1" }).fill("Pizza");
    await dialog.getByRole("textbox", { name: "Option 2" }).fill("Tacos");
    await dialog.getByRole("button", { name: "Post poll" }).click();
    await expect(dialog).not.toBeVisible();

    const card = pane.getByRole("region", { name: "Poll" });

    await expect(card.getByRole("radio", { name: "Pizza" })).toBeVisible();
    await card.getByRole("button", { name: "End poll now" }).click();
    await expect(card.locator(".poll-total")).toContainText("Final results");
    await expect(card.getByRole("radio")).toHaveCount(0);

    const root: MessagePage = await (
      await page.request.get(`/api/v1/rooms/${ROOM_IDS.general}/messages`)
    ).json();

    expect(root.messages.some((message) => message.markdownSource === "Thread lunch?")).toBe(false);
    await page.screenshot({ path: testInfo.outputPath("thread-poll.png") });
  });
}
