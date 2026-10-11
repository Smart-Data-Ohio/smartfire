import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import { WorkspaceLayout } from "../../src/api/schema/workspace-layout.ts";
import type { Sidebar } from "../../src/gen/Sidebar.ts";
import type { WorkspaceCategory } from "../../src/gen/WorkspaceCategory.ts";
import type { WorkspaceLayout as WorkspaceLayoutType } from "../../src/gen/WorkspaceLayout.ts";
import { collect, errorOf, expectStatus, get, harness, send } from "../s2/testing.ts";
import { ROOM_IDS } from "../seed.ts";

type Server = ReturnType<typeof harness>["server"];

const layoutOf = async (server: Server) =>
  (await get<Sidebar>(server, "/api/v1/sidebar")).workspaceLayout;

/** The rooms of one shared group, in order, as `[roomId, position]`. */
const group = (layout: WorkspaceLayoutType, categoryId: number | null) =>
  layout.rooms
    .filter((room) => room.workspaceCategoryId === categoryId)
    .map((room) => [room.roomId, room.position]);

const create = (server: Server, name: string) =>
  expectStatus<WorkspaceCategory>(server, "POST", "/api/v1/workspace_categories", { name }, 201);

const move = (
  server: Server,
  roomId: number,
  workspaceCategoryId: number | null,
  position: number,
) =>
  send(server, "PUT", `/api/v1/rooms/${roomId}/workspace_category`, {
    workspaceCategoryId,
    position,
  });

describe("mock shared workspace layout", () => {
  it("starts with no shared categories and every non-direct room unplaced", async () => {
    const { server } = harness();
    const layout = await layoutOf(server);

    expect(() => Schema.decodeUnknownSync(WorkspaceLayout)(layout)).not.toThrow();
    expect(layout.categories).toEqual([]);
    expect(layout.rooms.every((room) => room.position === null)).toBe(true);
    expect(layout.rooms.map((room) => room.roomId)).not.toContain(ROOM_IDS.dmMaya);
  });

  it("creates categories at the end and publishes the layout", async () => {
    const { server } = harness();
    const events = collect(server);

    expect(await create(server, "Product")).toEqual({ id: 1, name: "Product", position: 0 });
    expect(await create(server, "Ops")).toEqual({ id: 2, name: "Ops", position: 1 });
    expect(events.map((event) => event.type)).toEqual([
      "workspace.layout.updated",
      "workspace.layout.updated",
    ]);
  });

  it("refuses a blank name and anyone but an administrator", async () => {
    const { server } = harness();

    const blank = await send(server, "POST", "/api/v1/workspace_categories", { name: " " });

    expect(blank.status).toBe(422);

    await send(server, "POST", "/__mock/viewer-role", { role: "member" });

    const refused = await send(server, "POST", "/api/v1/workspace_categories", { name: "Ops" });

    expect(refused.status).toBe(403);
    expect((await move(server, ROOM_IDS.general, null, 0)).status).toBe(403);
  });

  it("renumbers both groups a room moves between, clamping the position", async () => {
    const { server } = harness();
    const product = await create(server, "Product");

    expect((await move(server, ROOM_IDS.design, product.id, 0)).status).toBe(200);
    expect((await move(server, ROOM_IDS.general, product.id, 99)).status).toBe(200);
    expect((await move(server, ROOM_IDS.quiet, product.id, 1)).status).toBe(200);

    const layout = await layoutOf(server);

    expect(group(layout, product.id)).toEqual([
      [ROOM_IDS.design, 0],
      [ROOM_IDS.quiet, 1],
      [ROOM_IDS.general, 2],
    ]);

    await move(server, ROOM_IDS.quiet, null, 0);

    const after = await layoutOf(server);

    expect(group(after, product.id)).toEqual([
      [ROOM_IDS.design, 0],
      [ROOM_IDS.general, 1],
    ]);
    expect(group(after, null)[0]).toEqual([ROOM_IDS.quiet, 0]);
  });

  it("refuses to place a direct message", async () => {
    const { server } = harness();
    const response = await move(server, ROOM_IDS.dmMaya, null, 0);

    expect(response.status).toBe(422);
    expect(errorOf(response.json).tag).toBe("Validation");
  });

  it("answers a stale category order with 409 and keeps the order", async () => {
    const { server } = harness();
    const product = await create(server, "Product");
    const ops = await create(server, "Ops");

    const stale = await send(server, "PUT", "/api/v1/workspace_categories/order", {
      categoryIds: [ops.id],
    });

    expect(stale.status).toBe(409);

    await expectStatus(
      server,
      "PUT",
      "/api/v1/workspace_categories/order",
      {
        categoryIds: [ops.id, product.id],
      },
      200,
    );

    expect((await layoutOf(server)).categories.map((category) => category.name)).toEqual([
      "Ops",
      "Product",
    ]);
  });

  it("sends a deleted category's rooms to the end of the uncategorized ones", async () => {
    const { server } = harness();
    const product = await create(server, "Product");
    const ops = await create(server, "Ops");

    await move(server, ROOM_IDS.design, product.id, 0);
    await move(server, ROOM_IDS.general, null, 0);

    expect(
      (await send(server, "DELETE", `/api/v1/workspace_categories/${product.id}`)).status,
    ).toBe(204);

    const layout = await layoutOf(server);

    expect(layout.categories).toEqual([{ id: ops.id, name: "Ops", position: 0 }]);
    expect(group(layout, null).slice(0, 1)).toEqual([[ROOM_IDS.general, 0]]);
    // Positions count every room in the group, the ones this viewer isn't in too.
    expect(group(layout, null).at(-1)?.[0]).toBe(ROOM_IDS.design);
  });

  it("shows anyone else only the categories holding their rooms", async () => {
    const { server } = harness();

    await send(server, "POST", "/__mock/workspace-layout", {
      categories: ["Product", "Empty"],
      rooms: [{ roomId: ROOM_IDS.general, category: 0 }],
    });

    expect((await layoutOf(server)).categories.map((category) => category.name)).toEqual([
      "Product",
      "Empty",
    ]);

    await send(server, "POST", "/__mock/viewer-role", { role: "member" });

    expect((await layoutOf(server)).categories.map((category) => category.name)).toEqual([
      "Product",
    ]);
  });
});
