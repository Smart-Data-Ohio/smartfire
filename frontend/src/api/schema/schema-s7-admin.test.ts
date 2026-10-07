import { describe, expect, it } from "@effect/vitest";
import { DateTime, Schema } from "effect";
import {
  AuditLogPage,
  CreateIcon,
  CustomStyles,
  IntegrationsHealth,
  PeoplePage,
  PersonChange,
  PersonRemoved,
  PersonRole,
  UpdateLogo,
  UpdatePerson,
  UpdateWorkspace,
  Workspace,
  WorkspaceIconList,
} from "./admin.ts";

// The wire JSON below mirrors crates/api_types/src/tests_s7_admin.rs.
const person = {
  id: 8,
  name: "Grace Hopper",
  avatarUrl: "/users/8/avatar?v=1700000000",
  role: "administrator",
  banned: false,
  you: false,
  twoFactorEnabled: true,
  emailAddress: "grace@example.com",
  googleIdentityEmail: null,
  offerGoogleEmailLink: true,
} as const;

const issue = { subject: "ada", detail: "token revoked" };

const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

describe("S7 admin schemas", () => {
  it("round-trip the workspace and its writes", () => {
    roundTrips(Workspace, {
      name: "Smart Data",
      logoUrl: "/account/logo?v=1700000000",
      logoAttached: false,
      joinUrl: "https://chat.example/join/abc-123",
      canAdminister: true,
      restrictRoomCreationToAdministrators: false,
      version: "2.0.0",
    });
    roundTrips(UpdateWorkspace, { name: null, restrictRoomCreationToAdministrators: true });
    roundTrips(UpdateLogo, { signedId: "blob-1" });
  });

  it("round-trip people and the changes to them", () => {
    roundTrips(PeoplePage, { people: [person], nextPage: "2" });
    roundTrips(UpdatePerson, { role: "member" });
    roundTrips(PersonChange, { person, notice: null });
    roundTrips(PersonRemoved, { id: 8 });

    expect(() => Schema.decodeUnknownSync(PersonRole)("bot")).toThrow();
  });

  it("round-trip custom styles and icons", () => {
    roundTrips(CustomStyles, { css: "body { color: red }" });
    roundTrips(WorkspaceIconList, {
      icons: [
        {
          id: 3,
          name: "acme",
          title: "Acme Corp",
          creatorName: "Ada Lovelace",
          imageUrl: "/icons/acme",
        },
      ],
    });
    roundTrips(CreateIcon, { name: "acme", title: "Acme Corp", signedId: null });
  });

  it("decode an audit log page", () => {
    const wire = {
      filters: { actor: "ada", action: null, targetType: "User", from: "2026-10-01", to: null },
      entries: [
        {
          id: 41,
          createdAt: "2026-10-06T10:00:00.000Z",
          action: "user.role_change",
          actor: "Ada Lovelace",
          target: "Grace Hopper",
          targetType: "User",
          changes: "role: member → administrator",
          ipAddress: null,
        },
      ],
      nextPage: null,
      actions: ["user.role_change"],
      targetTypes: ["User"],
      exportUrl: "/account/audit_log.csv?actor=ada",
      exportTruncated: false,
      exportLimit: 5000,
    };

    const page = Schema.decodeUnknownSync(AuditLogPage)(wire);
    const entry = page.entries[0];

    expect(entry && DateTime.toEpochMillis(entry.createdAt)).toBe(Date.UTC(2026, 9, 6, 10));
    roundTrips(AuditLogPage, wire);
  });

  it("round-trip integrations health", () => {
    roundTrips(IntegrationsHealth, {
      github: {
        workspaceToken: true,
        appConfigured: false,
        webhookSecret: true,
        connected: 3,
        appTokens: 1,
        deliveries24h: 12,
        disconnected: [issue],
        lastErrors: [],
        fetchErrors: [],
      },
      google: {
        configured: true,
        connected: 2,
        pushEnabled: false,
        pushChannels: 0,
        disconnected: [],
        entryErrors: [],
        expiring: [{ userId: 7, expiresAt: null, error: null }],
      },
      fizzy: { configured: false, note: "No Fizzy integration is configured in this workspace." },
      agentDelivery: { pending: 0, failed24h: 1, recentErrors: [issue] },
      email: { enabled: true, roomsWithAddresses: 1 },
    });
  });
});
