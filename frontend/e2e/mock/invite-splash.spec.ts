import { MOCK_JOIN_CODE } from "../../mock/s2/join.ts";
import { gradientPng } from "./png.ts";
import { expect, expectNoHorizontalOverflow, test } from "./support.ts";

test("an admin uploads the splash and signed-out join and About show it without image layout shift", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/app/admin/workspace");
  await page
    .getByRole("textbox", { name: "Description", exact: true })
    .fill("A place for our community.");
  await page.getByRole("button", { name: "Save description" }).click();
  await expect(page.getByText("Description saved", { exact: true })).toBeVisible();
  await page.getByLabel("Choose icon image").setInputFiles({
    name: "logo.png",
    mimeType: "image/png",
    buffer: gradientPng(64, 64, [39, 92, 196], [24, 180, 160]),
  });
  await expect(page.getByRole("button", { name: "Replace icon" })).toBeEnabled();
  await page.getByLabel("Choose invite splash image").setInputFiles({
    name: "splash.png",
    mimeType: "image/png",
    buffer: gradientPng(960, 540, [255, 122, 69], [39, 92, 196]),
  });
  await expect(page.getByRole("button", { name: "Replace invite splash" })).toBeVisible();
  const source = await page.getByAltText("Current invite splash").getAttribute("src");
  expect(source).toMatch(/\/blobs\//);
  await page.reload();
  await expect(page.getByAltText("Current invite splash")).toHaveAttribute("src", source ?? "");

  let releaseImage: () => void = () => undefined;

  const imageGate = new Promise<void>((resolve) => {
    releaseImage = resolve;
  });

  await page.route("**/rails/active_storage/blobs/**", async (route) => {
    await imageGate;
    await route.continue();
  });
  await page.goto(`/app/join/${MOCK_JOIN_CODE}`, { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Join Smart Data" })).toBeVisible();
  await expect(page.getByText("A place for our community.", { exact: true })).toBeVisible();
  await expect(page.locator(".auth-view-logo")).toHaveAttribute("src", /\/blobs\//);

  const background = page.locator(".invite-splash img");
  await expect(background).toHaveAttribute("src", source ?? "");
  await expect(background).toHaveAttribute("alt", "");
  const before = await page.getByRole("main").boundingBox();
  releaseImage();
  await expect
    .poll(() =>
      background.evaluate(
        (img) => img instanceof HTMLImageElement && img.complete && img.naturalWidth > 0,
      ),
    )
    .toBe(true);
  expect(await page.getByRole("main").boundingBox()).toEqual(before);
  await page.unroute("**/rails/active_storage/blobs/**");
  await page.screenshot({ path: "../.scratch/invite-splash-join-desktop.png" });

  for (const theme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.getByLabel("Name", { exact: true }).blur();
    await expect(page.getByRole("button", { name: "Create account" })).toBeVisible();
    await expectNoHorizontalOverflow(page);
    const card = page.locator(".auth-view-card");
    expect(await card.evaluate((element) => getComputedStyle(element).backgroundColor)).not.toBe(
      "rgba(0, 0, 0, 0)",
    );
    await page.screenshot({ path: `../.scratch/invite-splash-join-${theme}.png` });
  }

  await page.goto("/app/about");
  await expect(page.getByRole("heading", { level: 2, name: "Smart Data" })).toBeVisible();
  await expect(page.locator(".invite-splash img")).toHaveAttribute("src", source ?? "");
  await expectNoHorizontalOverflow(page);
  await expect(page.getByText("A place for our community.", { exact: true })).toBeVisible();
  await expect(page.locator(".public-view-workspace img")).toHaveAttribute("src", /\/blobs\//);
  await page.screenshot({ path: "../.scratch/invite-splash-about.png" });

  await page.goto("/app/admin/workspace");
  await page.getByRole("button", { name: "Remove invite splash" }).click();
  await expect(page.getByRole("button", { name: "Upload invite splash" })).toBeVisible();
  await page.goto(`/app/join/${MOCK_JOIN_CODE}`);
  await expect(page.getByRole("heading", { name: "Join Smart Data" })).toBeVisible();
  await expect(page.locator(".invite-splash img")).toHaveCount(0);
});
