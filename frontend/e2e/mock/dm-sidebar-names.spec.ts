import { expect, openApp, ROOM_IDS, test, USER_IDS } from "./support.ts";

test("DM sidebar labels keep full names and expose clipped labels on hover", async ({ page }) => {
  await openApp(page, "");

  const maya = page.locator(
    `.sidebar-row-item[data-room-id="${ROOM_IDS.dmMaya}"] .sidebar-row-name`,
  );

  await expect(maya).toHaveText("Maya Okafor");
  await expect(maya).toHaveAttribute("title", "Maya Okafor");

  const state = await (await page.request.get("/__mock/state")).json();
  const headers = { "X-CSRF-Token": state.csrfToken };

  const created = await page.request.post("/api/v1/directs", {
    headers,
    data: { userIds: [USER_IDS.theo, USER_IDS.sam, USER_IDS.priya, USER_IDS.maya, USER_IDS.jonah] },
  });

  expect(created.ok()).toBe(true);

  const row = await created.json();
  const label = "Jonah Lindqvist, Maya Okafor, Priya Raman +2";
  const name = page.locator(`.sidebar-row-item[data-room-id="${row.room.id}"] .sidebar-row-name`);

  await expect(name).toHaveText(label);
  await expect(name).toHaveAttribute("title", label);

  const layout = await name.evaluate((element) => {
    const style = getComputedStyle(element);

    return {
      whiteSpace: style.whiteSpace,
      overflow: style.overflow,
      textOverflow: style.textOverflow,
      clipped: element.scrollWidth > element.clientWidth,
      height: element.closest(".sidebar-row")?.getBoundingClientRect().height,
    };
  });

  expect(layout).toMatchObject({
    whiteSpace: "nowrap",
    overflow: "hidden",
    textOverflow: "ellipsis",
    clipped: true,
  });
  expect(layout.height).toBe(
    await maya.evaluate(
      (element) => element.closest(".sidebar-row")?.getBoundingClientRect().height,
    ),
  );

  const renamed = await page.request.patch(`/api/v1/directs/${row.room.id}`, {
    headers,
    data: { name: "Launch crew" },
  });

  expect(renamed.ok()).toBe(true);
  await expect(name).toHaveText("Launch crew");

  const cleared = await page.request.patch(`/api/v1/directs/${row.room.id}`, {
    headers,
    data: { name: null },
  });

  expect(cleared.ok()).toBe(true);
  await expect(name).toHaveText(label);
  await name.click();
  await expect(page.locator(".room-header .room-title-name")).toHaveText("Jonah, Maya, Priya +2");
  await expect(page).toHaveTitle(/Jonah, Maya, Priya \+2 · Smartfire$/);
  await expect(name).toHaveText(label);
});
