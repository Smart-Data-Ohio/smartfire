import { expect, openApp, PHONE_TOUCH, ROOM_IDS, test, USER_IDS } from "./support.ts";

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
  await expect(page.locator(".room-header .room-title-name")).toHaveText(row.displayName);
  await expect.poll(() => page.title()).toContain(`${row.displayName} · Smartfire`);
  await expect(name).toHaveText(label);
});

test("direct status sits at the trailing edge without moving on hover", async ({ page }) => {
  const roomId = ROOM_IDS.dmMaya;
  const state = await (await page.request.get("/__mock/state")).json();
  const headers = { "X-CSRF-Token": state.csrfToken };

  const muted = await page.request.put(`/api/v1/rooms/${roomId}/involvement`, {
    headers,
    data: { involvement: "muted" },
  });

  const joined = await page.request.post("/__mock/huddle-join", {
    headers,
    data: { roomId, userId: USER_IDS.maya },
  });

  expect(muted.ok()).toBe(true);
  expect(joined.ok()).toBe(true);
  await openApp(page, "");

  const row = page.locator(`.sidebar-row-item[data-room-id="${roomId}"]`);
  const link = row.locator(".sidebar-row");
  const mute = row.locator(".sidebar-row-muted");
  const call = row.locator(".sidebar-call-mark");
  const name = row.locator(".sidebar-row-name");

  await expect(mute).toBeVisible();
  await expect(call).toBeVisible();

  const geometry = () =>
    link.evaluate((element) => {
      const trailing = element.querySelector(".sidebar-call-mark")?.getBoundingClientRect();
      const bell = element.querySelector(".sidebar-row-muted")?.getBoundingClientRect();

      return {
        inset: element.getBoundingClientRect().right - (trailing?.right ?? 0),
        gap: (trailing?.left ?? 0) - (bell?.right ?? 0),
      };
    });

  const before = await geometry();
  const nameBefore = await name.boundingBox();
  expect(before.inset).toBeLessThan(12);
  expect(before.gap).toBeGreaterThanOrEqual(0);
  await link.hover();
  await expect(row.locator(".sidebar-row-close")).toHaveCSS("opacity", "1");
  expect(await geometry()).toEqual(before);
  expect(await name.boundingBox()).toEqual(nameBefore);
});

test.describe("touch direct rows", () => {
  test.use(PHONE_TOUCH);

  test("hide the close button on touch screens", async ({ page }) => {
    await openApp(page, "");

    const row = page.locator(`.sidebar-row-item[data-room-id="${ROOM_IDS.dmMaya}"]`);

    await expect(row).toBeVisible();
    await expect(row.locator(".sidebar-row-close")).toHaveCSS("display", "none");
  });
});
