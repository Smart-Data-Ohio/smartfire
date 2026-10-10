import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import { expect, PHONE_SMALL, ROOM_IDS, test } from "./support.ts";

test("creates and reads emoji and image poll options on a phone", async ({ page }, testInfo) => {
  await page.setViewportSize(PHONE_SMALL);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM_IDS.general}`);
  await page.getByRole("textbox", { name: /^Message #/ }).waitFor();
  await page.getByRole("button", { name: "Attach and more" }).click();
  await page.getByRole("menuitem", { name: "Create a poll" }).click();
  const dialog = page.getByRole("dialog", { name: "Create a poll" });
  await dialog.getByRole("textbox", { name: "Question" }).fill("Media lunch?");
  await dialog.getByRole("textbox", { name: "Option 1" }).fill("Taco");
  await dialog.getByRole("textbox", { name: "Option 2" }).fill("Pizza");
  await dialog.getByRole("button", { name: "Emoji for option 1" }).click();
  await page.getByRole("combobox", { name: /Search/ }).fill("taco");
  await page.getByRole("option", { name: /taco/i }).click();
  await dialog.getByLabel("Image for option 2").setInputFiles({
    name: "pizza.png",
    mimeType: "image/png",
    buffer: Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==",
      "base64",
    ),
  });
  await expect(dialog.getByText("pizza.png")).toBeVisible();

  const posted = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" &&
      response.url().endsWith(`/api/v1/rooms/${ROOM_IDS.general}/polls`),
  );

  await dialog.getByRole("button", { name: "Post poll" }).click();
  const response = await posted;
  expect(response.status()).toBe(201);
  const message: MessageDTO = await response.json();
  await expect(dialog).not.toBeVisible();
  await page.goto(`/app/r/${ROOM_IDS.general}/m/${message.id}`);
  const row = page.locator("[data-message-row]").filter({ hasText: "Media lunch?" }).first();
  const card = row.getByRole("region", { name: "Poll" });
  await expect(card.getByText("🌮")).toBeVisible();
  await expect(card.getByRole("img", { name: "Pizza", exact: true })).toBeVisible();
  await expect(card.getByRole("img", { name: "Pizza", exact: true })).toHaveJSProperty(
    "naturalWidth",
    1,
  );
  await card.getByRole("radio", { name: /Taco/ }).check();
  await card.getByRole("button", { name: "Vote", exact: true }).click();
  await expect(card.getByRole("img", { name: "Pizza", exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  await page.screenshot({ path: testInfo.outputPath("poll-media-phone.png") });
});
