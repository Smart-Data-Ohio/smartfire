import { fileURLToPath } from "node:url";
import { expect, holdSync, openApp, ROOM_IDS, test } from "./support.ts";

test("an anchored /play awaits the latest page and plays the broadcast before the POST reply", async ({
  page,
}) => {
  await page.addInitScript(() => {
    HTMLMediaElement.prototype.play = () => {
      document.documentElement.dataset.chatSoundPlays = String(
        Number(document.documentElement.dataset.chatSoundPlays ?? "0") + 1,
      );

      return Promise.resolve();
    };
  });
  const releaseSync = await holdSync(page);

  await openApp(page, `r/${ROOM_IDS.general}/m/10005`);
  await expect(page.locator('[data-message-id="10005"]')).toBeVisible();
  await releaseSync();

  let releasePage: () => void = () => undefined;
  let releasePost: () => void = () => undefined;

  const pageGate = new Promise<void>((resolve) => {
    releasePage = resolve;
  });

  const postGate = new Promise<void>((resolve) => {
    releasePost = resolve;
  });

  const posts: unknown[] = [];
  let requestedLatest = false;

  await page.route(`**/api/v1/rooms/${ROOM_IDS.general}/slash_commands`, (route) => {
    if (route.request().method() === "POST") {
      throw new Error("/play must use ordinary message submission");
    }

    return route.continue();
  });
  await page.route(`**/api/v1/rooms/${ROOM_IDS.general}/messages`, async (route) => {
    if (route.request().method() === "GET") {
      requestedLatest = true;
      await pageGate;

      return route.continue();
    }

    posts.push(route.request().postDataJSON());
    // The real mock publishes the broadcast while the browser's POST response stays held.
    const response = await route.fetch();

    await postGate;

    return route.fulfill({ response });
  });

  try {
    const input = page.locator(".composer-input").first();

    await input.fill("/play bell");
    await input.press("Enter");
    await expect.poll(() => requestedLatest).toBe(true);
    expect(posts).toEqual([]);
    releasePage();
    await expect(page.getByRole("button", { name: "Play bell" })).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("data-chat-sound-plays", "1");
    expect(posts).toHaveLength(1);
    expect(posts[0]).toMatchObject({
      markdownSource: "/play bell",
      clientMessageId: expect.any(String),
    });
    releasePost();
    await expect(input).toHaveValue("");
    await expect(page.locator("html")).toHaveAttribute("data-chat-sound-plays", "1");
  } finally {
    releasePage();
    releasePost();
  }
});

test("the composer posts a classic sound with a working playback control, and reload stays silent", async ({
  page,
}) => {
  await page.route("**/assets/bell.mp3", (route) =>
    route.fulfill({
      path: fileURLToPath(new URL("../../../web/app/assets/sounds/bell.mp3", import.meta.url)),
      contentType: "audio/mpeg",
    }),
  );
  await page.addInitScript(() => {
    const play = HTMLMediaElement.prototype.play;

    HTMLMediaElement.prototype.play = function () {
      this.addEventListener(
        "playing",
        () => {
          document.documentElement.dataset.chatSoundUrl = this.src;
          document.documentElement.dataset.chatSoundPlays = String(
            Number(document.documentElement.dataset.chatSoundPlays ?? "0") + 1,
          );
        },
        { once: true },
      );

      return play.call(this);
    };
  });
  await openApp(page, `r/${ROOM_IDS.general}`);
  const input = page.locator(".composer-input").first();

  await input.fill("/play bell");
  await input.press("Enter");
  const button = page.getByRole("button", { name: "Play bell" });

  await expect(button).toBeVisible();
  await expect(button.locator("..")).toContainText("🔔");
  await expect(page.locator("html")).toHaveAttribute("data-chat-sound-url", /\/assets\/bell\.mp3$/);
  await expect(page.locator("html")).toHaveAttribute("data-chat-sound-plays", "1");
  await button.click();
  await expect(page.locator("html")).toHaveAttribute("data-chat-sound-plays", "2");

  await page.reload();
  await expect(button).toBeVisible();
  await expect(page.locator("html")).not.toHaveAttribute("data-chat-sound-plays");
  await button.click();
  await expect(page.locator("html")).toHaveAttribute("data-chat-sound-plays", "1");
});

test("the sound message renders the classic image and dimensions", async ({ page }) => {
  await page.route("**/assets/56k.mp3", (route) =>
    route.fulfill({
      path: fileURLToPath(new URL("../../../web/app/assets/sounds/56k.mp3", import.meta.url)),
      contentType: "audio/mpeg",
    }),
  );
  await page.route("**/assets/sounds/56k.webp", (route) =>
    route.fulfill({
      path: fileURLToPath(
        new URL("../../../web/app/assets/images/sounds/56k.webp", import.meta.url),
      ),
      contentType: "image/webp",
    }),
  );
  await openApp(page, `r/${ROOM_IDS.general}`);
  const input = page.locator(".composer-input").first();
  await input.fill("/play 56k");
  await input.press("Enter");
  const image = page.getByRole("img", { name: "56k", exact: true });
  await expect(page.getByRole("button", { name: "Play 56k" })).toBeVisible();
  await expect(image).toBeVisible();
  await expect(image).toHaveAttribute("width", "79");
  await expect(image).toHaveAttribute("height", "33");
  await expect
    .poll(() =>
      image.evaluate((element) => element instanceof HTMLImageElement && element.naturalWidth),
    )
    .toBe(79);
});
