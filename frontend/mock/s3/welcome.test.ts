import { describe, expect, it } from "vitest";
import type { WelcomeAdmin } from "../../src/gen/WelcomeAdmin.ts";
import type { WorkspaceWelcome } from "../../src/gen/WorkspaceWelcome.ts";
import type { Json } from "../json.ts";
import { collect, errorOf, expectStatus, get, harness, send } from "../s2/testing.ts";
import { JOINABLE_OPEN_ROOM, ROOM_IDS } from "../seed.ts";

const room = (roomId: number, description = "") => ({ roomId, emoji: null, description });

const note = (title: string, body: string) => ({ title, emoji: null, roomId: null, body });

describe("the welcome card and server guide", () => {
  it("lets only administrators read or replace the configuration", async () => {
    const { server } = harness();
    const empty = await get<WelcomeAdmin>(server, "/api/v1/admin/welcome");

    expect(empty.settings).toEqual({ message: "", rooms: [], guide: [] });
    expect(empty.choices.map((choice) => choice.id)).toContain(JOINABLE_OPEN_ROOM.id);

    await send(server, "POST", "/__mock/viewer-role", { role: "member" });

    expect((await server.handle({ method: "GET", path: "/api/v1/admin/welcome" })).status).toBe(
      403,
    );
    await expectStatus(
      server,
      "PUT",
      "/api/v1/admin/welcome",
      { message: "Mine", rooms: [], guide: [] },
      403,
    );

    const seen = await get<WorkspaceWelcome>(server, "/api/v1/welcome");

    expect(seen.messageHtml).toBeNull();
    expect(seen.canAdminister).toBe(false);
  });

  it("trims and saves a valid configuration, renders its markdown and tells every client", async () => {
    const { server } = harness();
    const events = collect(server);

    const saved = await expectStatus<WelcomeAdmin>(
      server,
      "PUT",
      "/api/v1/admin/welcome",
      {
        message: "  Say **hi**  ",
        rooms: [{ roomId: ROOM_IDS.general, emoji: " 👋 ", description: " Everyone " }],
        guide: [
          note("Hours", "9 to *5*"),
          { title: "Ship it", emoji: null, roomId: ROOM_IDS.design, body: null },
        ],
      },
      200,
    );

    expect(saved.settings.message).toBe("Say **hi**");
    expect(saved.settings.rooms).toEqual([
      { roomId: ROOM_IDS.general, emoji: "👋", description: "Everyone" },
    ]);
    expect(events.some((event) => event.type === "workspace.updated")).toBe(true);

    const seen = await get<WorkspaceWelcome>(server, "/api/v1/welcome");

    expect(seen.messageHtml).toContain("<strong>hi</strong>");
    expect(seen.rooms).toEqual([
      {
        roomId: ROOM_IDS.general,
        name: "general",
        emoji: "👋",
        description: "Everyone",
        joined: true,
      },
    ]);
    expect(seen.guide.map((entry) => entry.title)).toEqual(["Hours", "Ship it"]);
    expect(seen.guide[0]?.bodyHtml).toContain("<em>5</em>");
    expect(seen.guide[1]?.room).toEqual({ id: ROOM_IDS.design, name: "design", joined: true });
  });

  it("refuses what the server refuses, naming the field, and keeps the last configuration", async () => {
    const { server } = harness();
    const good = { message: "Hello", rooms: [room(ROOM_IDS.general)], guide: [] };

    await expectStatus(server, "PUT", "/api/v1/admin/welcome", good, 200);

    const cases: readonly [string, Json][] = [
      ["message", { message: "x".repeat(1001), rooms: [], guide: [] }],
      [
        "rooms",
        { message: "", rooms: Array.from({ length: 6 }, (_, i) => room(i + 1)), guide: [] },
      ],
      [
        "rooms",
        { message: "", rooms: [room(ROOM_IDS.general), room(ROOM_IDS.general)], guide: [] },
      ],
      ["rooms", { message: "", rooms: [room(ROOM_IDS.general, "x".repeat(101))], guide: [] }],
      ["rooms", { message: "", rooms: [{ roomId: 1, emoji: "wave", description: "" }], guide: [] }],
      ["rooms", { message: "", rooms: [room(999_999)], guide: [] }],
      [
        "guide",
        { message: "", rooms: [], guide: Array.from({ length: 11 }, () => note("Tip", "Body")) },
      ],
      ["guide", { message: "", rooms: [], guide: [note(" ", "Body")] }],
      [
        "guide",
        {
          message: "",
          rooms: [],
          guide: [{ title: "Both", emoji: null, roomId: 1, body: "Body" }],
        },
      ],
      [
        "guide",
        {
          message: "",
          rooms: [],
          guide: [{ title: "Neither", emoji: null, roomId: null, body: " " }],
        },
      ],
    ];

    for (const [field, body] of cases) {
      const response = await send(server, "PUT", "/api/v1/admin/welcome", body);

      expect(response.status, JSON.stringify(body)).toBe(422);
      expect(errorOf(response.json).tag).toBe("Validation");
      expect(JSON.stringify(response.json)).toContain(`"${field}"`);
    }

    expect((await get<WelcomeAdmin>(server, "/api/v1/admin/welcome")).settings.message).toBe(
      "Hello",
    );
  });

  it("sends each viewer only the rooms they belong to or may join", async () => {
    const { server } = harness();

    await expectStatus(
      server,
      "PUT",
      "/api/v1/admin/welcome",
      {
        message: "",
        rooms: [
          room(ROOM_IDS.general),
          room(ROOM_IDS.launchPlanning, "Private chatter"),
          room(JOINABLE_OPEN_ROOM.id),
        ],
        guide: [
          { title: "Secret plans", emoji: null, roomId: ROOM_IDS.launchPlanning, body: null },
          note("Hours", "Nine to five"),
        ],
      },
      200,
    );

    // The viewer is taken out of the closed room: from now on nothing about it comes back.
    await expectStatus(
      server,
      "PATCH",
      `/api/v1/rooms/${ROOM_IDS.launchPlanning}`,
      { type: "closed", name: "launch-planning", iconName: null, userIds: [2, 3, 6] },
      200,
    );

    const response = await server.handle({ method: "GET", path: "/api/v1/welcome" });
    const text = JSON.stringify(response.json);

    for (const leak of [
      "launch-planning",
      "Private chatter",
      "Secret plans",
      `"roomId":${ROOM_IDS.launchPlanning},`,
    ]) {
      expect(text).not.toContain(leak);
    }

    const seen = await get<WorkspaceWelcome>(server, "/api/v1/welcome");

    expect(seen.rooms.map((card) => [card.roomId, card.joined])).toEqual([
      [ROOM_IDS.general, true],
      [JOINABLE_OPEN_ROOM.id, false],
    ]);
    expect(seen.guide.map((entry) => entry.title)).toEqual(["Hours"]);
  });

  it("forgets the configuration when the world resets", async () => {
    const { server } = harness();

    await send(server, "POST", "/__mock/welcome", { message: "Hi", rooms: [], guide: [] });
    expect((await get<WorkspaceWelcome>(server, "/api/v1/welcome")).messageHtml).not.toBeNull();

    await send(server, "POST", "/__mock/reset");
    expect((await get<WorkspaceWelcome>(server, "/api/v1/welcome")).messageHtml).toBeNull();
  });
});
