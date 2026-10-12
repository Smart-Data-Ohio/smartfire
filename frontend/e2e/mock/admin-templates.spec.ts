import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { DESKTOP, expect, expectNoHorizontalOverflow, PHONE_TOUCH, test } from "./support.ts";

const fixture = fileURLToPath(new URL("./fixtures/server-template.json", import.meta.url));

for (const phone of [false, true]) {
  test.describe(phone ? "templates at 360 px" : "templates on desktop", () => {
    test.use(phone ? PHONE_TOUCH : { viewport: DESKTOP });

    test("exports structure, previews a file and imports only after confirmation", async ({
      page,
      request,
    }) => {
      await page.emulateMedia({ reducedMotion: "reduce" });
      await page.goto("/app/admin/templates");
      await expect(page.getByRole("heading", { level: 1, name: "Templates" })).toBeVisible();
      const downloadEvent = page.waitForEvent("download");

      await page.getByRole("button", { name: "Export template" }).click();
      const download = await downloadEvent;
      const path = await download.path();

      expect(download.suggestedFilename()).toBe("smartfire-template-v1.json");

      if (path === null) throw new Error("Expected a downloaded template");
      const downloaded = JSON.parse(await readFile(path, "utf8"));

      expect(downloaded.format).toBe("smartfire-template");
      expect(downloaded.version).toBe(1);
      const before = await (await request.get("/api/v1/admin/templates/export")).json();

      await page.getByLabel("Template file").setInputFiles(fixture);
      const preview = page.getByRole("region", { name: "Template preview" });

      await expect(preview).toBeVisible();
      await expect(preview.getByText("Room general will be created as general (2)")).toBeVisible();
      expect(await (await request.get("/api/v1/admin/templates/export")).json()).toEqual(before);
      await preview.getByText("Town hall (stage)").click();
      await expect(preview.getByText("96000 bps")).toBeVisible();
      await expectNoHorizontalOverflow(page);
      await page.getByRole("button", { name: "Confirm import" }).focus();
      await page.keyboard.press("Enter");
      await expect(page.getByText("Created 1 categories and 3 rooms.")).toBeVisible();
      const after = await (await request.get("/api/v1/admin/templates/export")).json();

      expect(after.categories.at(-1).rooms.map((room: { name: string }) => room.name)).toEqual([
        "general (2)",
        "Town hall",
      ]);
      await expectNoHorizontalOverflow(page);
    });
  });
}
