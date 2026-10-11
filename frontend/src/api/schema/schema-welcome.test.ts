import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import { manualScheduler } from "../../../mock/scheduler.ts";
import { createMockServer } from "../../../mock/server.ts";
import { WelcomeAdmin, WorkspaceWelcome } from "./welcome.ts";

// The wire JSON below mirrors crates/api_types/src/tests_s7_admin.rs.
const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

describe("the welcome schemas", () => {
  it("round-trip the configuration and what a viewer reads", () => {
    roundTrips(WelcomeAdmin, {
      settings: {
        message: "Say **hi**",
        rooms: [{ roomId: 3, emoji: "👋", description: "Introduce yourself" }],
        guide: [
          { title: "Start here", emoji: null, roomId: 3, body: null },
          { title: "Hours", emoji: "🕘", roomId: null, body: "9 to 5" },
        ],
      },
      choices: [{ id: 3, name: "general" }],
    });
    roundTrips(WorkspaceWelcome, {
      name: "Smart Data",
      description: "",
      messageHtml: '<div class="markdown-body"><p>Hi</p></div>',
      rooms: [
        {
          roomId: 3,
          name: "general",
          emoji: null,
          description: "Introduce yourself",
          joined: false,
        },
      ],
      guide: [
        {
          title: "Start here",
          emoji: null,
          bodyHtml: null,
          room: { id: 3, name: "general", joined: true },
        },
      ],
      canAdminister: false,
    });
  });

  it("decode what the mock serves", async () => {
    const clock = manualScheduler(Date.UTC(2026, 9, 6, 16, 30, 0));
    const mock = createMockServer({ now: () => clock.now(), seed: 1, scheduler: clock });

    const saved = await mock.handle({
      method: "PUT",
      path: "/api/v1/admin/welcome",
      body: {
        message: "Hello",
        rooms: [{ roomId: 1, emoji: null, description: "" }],
        guide: [{ title: "Hours", emoji: null, roomId: null, body: "Nine to five" }],
      },
      headers: { "X-CSRF-Token": mock.csrfToken() },
    });

    expect(saved.status).toBe(200);
    expect(() => Schema.decodeUnknownSync(WelcomeAdmin)(saved.json)).not.toThrow();

    const seen = await mock.handle({ method: "GET", path: "/api/v1/welcome" });

    expect(Schema.decodeUnknownSync(WorkspaceWelcome)(seen.json).rooms).toHaveLength(1);
  });
});
