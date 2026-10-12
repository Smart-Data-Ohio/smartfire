import { afterEach, describe, expect, it, vi } from "vitest";
import type { TemplatePreview } from "../../src/gen/TemplatePreview.ts";
import * as folding from "../../src/lib/case-fold.ts";
import { dispatch, type S2Context } from "../s2/context.ts";
import { expectStatus, get, harness } from "../s2/testing.ts";
import { buildWorld } from "../s3/seed.ts";
import { VIEWER_ID } from "../seed.ts";
import { createRoomManagement } from "./rooms.ts";
import { createTemplates } from "./templates.ts";

function exportHarness() {
  const { clock } = harness();
  const world = buildWorld(clock.now(), 1);

  const unused = () => {
    throw new Error("Unused export context");
  };

  const ctx: S2Context = {
    world: () => world,
    now: () => clock.now(),
    scheduler: clock,
    publish: () => {},
    uuid: () => "",
    hex: () => "",
    mentionables: () => [],
    roomOr404: unused,
    usersFor: unused,
    displayName: unused,
    sidebarRow: unused,
    roomDetail: unused,
    postToRoom: unused,
  };

  const admin = { hasIcon: () => false, roomCreationRestricted: () => false };

  const management = createRoomManagement(ctx, admin, {
    roomRoles: () => new Map(),
    reviseRoom: () => {},
    removeRoom: () => {},
  });

  const templates = createTemplates(ctx, admin, management);

  const exportFile = async () => {
    const run = dispatch(
      templates.routes,
      "GET",
      "/admin/templates/export",
      new URLSearchParams(),
      undefined,
    );

    if (run === null) throw new Error("Missing export route");

    return (await run()).json;
  };

  return { world, exportFile };
}

afterEach(() => vi.restoreAllMocks());

describe("template review regressions", () => {
  it("exports no nonmember restricted room, topic, placement or count", async () => {
    const { world, exportFile } = exportHarness();
    world.workspaceCategories.push({ id: 1, name: "Shared category", position: 0 });
    const before = await exportFile();
    const source = world.rooms.values().next().value;

    if (source === undefined) throw new Error("Missing seed room");

    for (const kind of ["closed", "voice", "stage", "board"] as const) {
      const id = world.nextRoomId++;

      world.rooms.set(id, {
        ...source,
        room: {
          ...source.room,
          id,
          kind,
          name: `Confidential ${kind}`,
          topic: "Confidential acquisition details",
        },
        memberIds: [],
        membership: { ...source.membership, roomId: id },
      });
      world.workspacePlacements.set(id, { categoryId: 1, position: id });
    }

    const result = await exportFile();

    expect(JSON.stringify(result)).not.toContain("Confidential");
    expect(result).toEqual(before);

    for (const record of world.rooms.values()) {
      if (record.room.name?.startsWith("Confidential")) record.memberIds.push(VIEWER_ID);
    }

    expect(JSON.stringify(await exportFile())).toContain("Confidential acquisition details");
  });

  it("stops before any duplicate work on an oversized catalog fitting below 1 MiB", async () => {
    const { server } = harness();
    const before = await get(server, "/api/v1/admin/templates/export");
    const compare = vi.spyOn(folding, "caselessEqual");
    const fold = vi.spyOn(folding, "caseFold");

    const file = {
      format: "smartfire-template",
      version: 1,
      categories: [],
      rooms: [
        {
          kind: "board",
          name: "Huge catalog",
          iconName: null,
          topic: null,
          announcement: false,
          voiceBitrate: null,
          voiceUserLimit: null,
          board: {
            tags: Array.from({ length: 2000 }, (_, index) => ({
              name: `tag${index}`,
              emoji: null,
            })),
            tagsRequired: false,
            defaultTag: null,
          },
        },
      ],
    };

    const contents = JSON.stringify(file);

    expect(new TextEncoder().encode(contents).length).toBeLessThan(1024 * 1024);

    const preview = await expectStatus<TemplatePreview>(
      server,
      "POST",
      "/api/v1/admin/templates/preview",
      { contents },
      200,
    );

    expect(compare).not.toHaveBeenCalled();
    expect(fold).not.toHaveBeenCalled();
    expect(preview.template).toBeNull();
    expect(preview.changes).toEqual([]);
    expect(preview.errors).toHaveLength(1);
    await expectStatus(
      server,
      "POST",
      "/api/v1/admin/templates/import",
      { contents, expected: file },
      422,
    );
    expect(compare).not.toHaveBeenCalled();
    expect(fold).not.toHaveBeenCalled();
    expect(await get(server, "/api/v1/admin/templates/export")).toEqual(before);
  });
});
