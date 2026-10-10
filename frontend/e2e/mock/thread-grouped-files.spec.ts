import { onboardingMockupPng } from "../../mock/s2/assets.ts";
import { MESSAGE_IDS } from "../../mock/s2/seed.ts";
import { expect, ROOM_IDS, test } from "./support.ts";

test("a thread starts with a three-file gallery and every file appears in room Files", async ({
  page,
}) => {
  const roomId = ROOM_IDS.general;
  const names = ["thread-one.png", "thread-two.png", "thread-three.png"];

  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${roomId}/t/new?parent=${MESSAGE_IDS.generalChart}`);
  const pane = page.locator("aside.right-pane");
  const composer = pane.getByRole("textbox", { name: "Reply…" });

  await composer.waitFor();
  await pane.locator('.composer input[type="file"]').setInputFiles(
    names.map((name) => ({
      name,
      mimeType: "image/png",
      buffer: Buffer.from(onboardingMockupPng()),
    })),
  );
  await expect(pane.locator('.tray-chip[data-phase="done"]')).toHaveCount(3);
  await composer.fill("Grouped thread files");
  await composer.press("Enter");
  await expect(page).toHaveURL(new RegExp(`/r/${roomId}/t/\\d+$`));

  const gallery = pane
    .getByRole("log", { name: "Replies" })
    .getByRole("list", { name: "3 images and videos" });

  await expect(gallery.getByRole("listitem")).toHaveCount(3);
  await page.goto(`/app/r/${roomId}/files`);

  for (const name of names) {
    await expect(page.locator("aside.right-pane").getByText(name, { exact: true })).toBeVisible();
  }
});
