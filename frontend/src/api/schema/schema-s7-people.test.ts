import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import { DirectoryPerson, PeopleDirectory, PersonProfile, PersonStatus } from "./people.ts";

// The wire JSON below mirrors crates/api_types/src/tests_s7_people.rs.
const user = {
  id: 7,
  name: "Ada Lovelace",
  role: "administrator",
  status: "active",
  bio: null,
  avatarUrl: "/users/7/avatar?v=1700000000",
  hasAvatar: true,
  customStatus: { emoji: "🌴", text: "On a beach", expiresAt: null },
  avatarIcon: null,
  agent: null,
  createdAt: "2026-09-26T12:26:46.848Z",
  updatedAt: "2026-09-26T12:26:46.848Z",
} as const;

const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

describe("S7 people schemas", () => {
  it("round-trip the directory and its rows", () => {
    const row = { userId: 7, online: true, starred: true, agent: false };

    roundTrips(DirectoryPerson, row);
    roundTrips(PeopleDirectory, { people: [row], users: [user] });
    roundTrips(PeopleDirectory, { people: [], users: [] });
  });

  it("round-trip every presence and a status line", () => {
    for (const presence of ["online", "idle", "offline", "dnd"] as const) {
      roundTrips(PersonStatus, { presence, statusText: null });
    }

    roundTrips(PersonStatus, { presence: "dnd", statusText: "Writing" });
  });

  it("round-trip a full profile and an empty one", () => {
    roundTrips(PersonProfile, {
      user,
      status: { presence: "dnd", statusText: "Writing" },
      dndAllowed: true,
      emailAddress: "ada@example.com",
      transferUrl: "https://chat.example/session/transfers/token",
      transferQrSvg: "<svg/>",
      canBan: true,
    });
    roundTrips(PersonProfile, {
      user,
      status: null,
      dndAllowed: null,
      emailAddress: null,
      transferUrl: null,
      transferQrSvg: null,
      canBan: false,
    });
  });

  it("refuses an unknown presence", () => {
    expect(() =>
      Schema.decodeUnknownSync(PersonStatus)({ presence: "away", statusText: null }),
    ).toThrow();
  });
});
