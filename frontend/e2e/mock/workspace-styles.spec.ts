import { expect, saveAccountAppearance, test } from "./support.ts";

const selector = 'style[data-turbo-track="reload"]';

const css =
  ":root { --accent: rgb(200, 40, 60); } body { border-top: 7px solid rgb(200, 40, 60); }";

test("saved CSS reaches this SPA, another open tab and the next load", async ({
  page,
  context,
}) => {
  await page.goto("/app/admin/styles");
  const observer = await context.newPage();
  await observer.goto("/app/");
  await expect(observer.getByRole("button", { name: "Your account" })).toBeVisible();
  await page.getByRole("textbox", { name: "Custom CSS" }).fill(css);
  await page.getByRole("button", { name: "Save changes" }).click();
  await expect(page.getByText("Custom styles saved", { exact: true })).toBeVisible();

  for (const tab of [page, observer]) {
    await expect(tab.locator("body")).toHaveCSS("border-top-width", "7px");
    await expect(tab.locator(selector)).toHaveJSProperty("textContent", css);
  }

  await observer.reload();
  await expect(observer.locator("body")).toHaveCSS("border-top-width", "7px");
  await page.getByRole("textbox", { name: "Custom CSS" }).fill("");
  await page.getByRole("button", { name: "Save changes" }).click();
  await expect(observer.locator(selector)).toHaveCount(0);
  await expect(observer.locator("body")).toHaveCSS("border-top-width", "0px");
});

test("preview renders the real page, safely, and leaving restores the saved CSS", async ({
  page,
}) => {
  await page.goto("/app/admin/styles");
  const draft = `${css} body::after { content: "</style><script>window.injected = true</script>&"; }`;
  await page.getByRole("textbox", { name: "Custom CSS" }).fill(draft);
  await page.getByRole("checkbox", { name: "Preview on this page" }).check();
  await expect(page.locator("body")).toHaveCSS("border-top-width", "7px");
  expect(
    await page.locator("body").evaluate((body) => getComputedStyle(body, "::after").content),
  ).toBe('"</style><script>window.injected = true</script>&"');
  await page.getByRole("button", { name: "Save changes" }).click();
  await expect(page.getByText("Custom styles saved", { exact: true })).toBeVisible();
  await page.getByRole("checkbox", { name: "Preview on this page" }).uncheck();
  await expect(page.locator("body")).toHaveCSS("border-top-width", "7px");
  await page
    .getByRole("textbox", { name: "Custom CSS" })
    .fill("body { border-top: 19px solid blue; }");
  await page.getByRole("checkbox", { name: "Preview on this page" }).check();
  await expect(page.locator("body")).toHaveCSS("border-top-width", "19px");
  await page
    .getByRole("navigation", { name: "Workspace sections" })
    .getByRole("link", { name: "Workspace", exact: true })
    .click();
  await expect(page.locator("body")).toHaveCSS("border-top-width", "7px");
  await page.reload();
  await expect(page.locator("body")).toHaveCSS("border-top-width", "7px");
  expect(await page.locator(selector).textContent()).toContain("\\3c /style>");
  expect(await page.evaluate(() => Object.hasOwn(window, "injected"))).toBe(false);
});

test("workspace tokens beat base styles while device appearance remains effective", async ({
  page,
}) => {
  await page.addInitScript(() =>
    localStorage.setItem(
      "smartfire.appearance",
      JSON.stringify({
        themeOverride: "dark",
        font: "serif",
        palette: "graphite",
        density: "compact",
        motion: "reduce",
      }),
    ),
  );
  await page.goto("/app/admin/styles");

  await saveAccountAppearance(page, { textSize: "large" });
  await page.reload();

  const before = await page
    .locator("html")
    .evaluate((root) => getComputedStyle(root).getPropertyValue("--accent"));

  await page
    .getByRole("textbox", { name: "Custom CSS" })
    .fill(
      `${css} :root { color-scheme: light; --font-sans: monospace; --bg-canvas: rgb(22, 33, 44); --sidebar-row-height: 99px; --duration-micro: 900ms; font-size: 200%; }`,
    );
  await page.getByRole("button", { name: "Save changes" }).click();
  await expect(page.getByText("Custom styles saved", { exact: true })).toBeVisible();
  await expect(page.locator("html")).toHaveCSS("color-scheme", "dark");
  await expect(page.locator("html")).toHaveCSS("font-size", "17px");
  await expect(page.locator("html")).toHaveAttribute("data-density", "compact");
  await expect(page.locator("html")).toHaveAttribute("data-motion", "reduce");
  expect(
    await page
      .locator("html")
      .evaluate((root) => getComputedStyle(root).getPropertyValue("--sidebar-row-height").trim()),
  ).toBe("24px");
  expect(
    await page
      .locator("html")
      .evaluate((root) => getComputedStyle(root).getPropertyValue("--duration-micro").trim()),
  ).toBe("80ms");
  expect(
    await page
      .locator("html")
      .evaluate((root) => getComputedStyle(root).getPropertyValue("--accent")),
  ).toBe(before);
  expect(
    await page.locator("body").evaluate((body) => getComputedStyle(body).fontFamily),
  ).toContain("Source Serif 4");
  expect(
    await page
      .locator("html")
      .evaluate((root) => getComputedStyle(root).getPropertyValue("--bg-canvas").trim()),
  ).toBe("rgb(22, 33, 44)");
});
