import { describe, expect, it } from "vitest";
import { get, harness } from "../../../mock/s2/testing.ts";
import type { IntegrationChange } from "../../gen/IntegrationChange.ts";
import type { Settings } from "../../gen/Settings.ts";
import type { StatusSettings } from "../../gen/StatusSettings.ts";
import {
  classicPage,
  connectionSummary,
  deviceName,
  fieldError,
  keywordLines,
  oooSummary,
  SECTIONS,
  sessionMeta,
  statusExpiry,
  withConnection,
  withDependents,
} from "./settings-format.ts";

const status: StatusSettings = {
  presenceSetting: "auto",
  customStatusEmoji: null,
  customStatusText: null,
  customStatusExpiresAt: null,
  meetingStatusEnabled: false,
  oooCalendarEnabled: false,
  oooUntil: null,
  oooManual: false,
  oooNote: null,
  calendarError: null,
};

describe("settings words", () => {
  it("lists every section once, the profile at the root", () => {
    expect(new Set(SECTIONS.map((section) => section.path)).size).toBe(SECTIONS.length);
    expect(SECTIONS[0]?.path).toBe("/settings");
  });

  it("reads keyword alerts one per line, trimmed, blanks dropped", () => {
    expect(keywordLines("  deploy freeze \n\nprod\n   \n")).toEqual(["deploy freeze", "prod"]);
    expect(keywordLines("")).toEqual([]);
  });

  it("words a field's messages as the classic forms print them", () => {
    const fields = { currentPassword: ["is incorrect"], bio: ["is too long", "is rude"] };

    expect(fieldError(fields, "currentPassword", "Current password")).toBe(
      "Current password is incorrect.",
    );
    expect(fieldError(fields, "bio", "Bio")).toBe("Bio is too long and is rude.");
    expect(fieldError(fields, "name", "Name")).toBeUndefined();
  });

  it("says where out of office comes from, and nothing when not out", () => {
    expect(oooSummary(status)).toBeNull();

    const until = "2026-10-09T17:00:00.000Z";

    expect(oooSummary({ ...status, oooUntil: until, oooManual: true })).toMatch(
      /^Out of office until .+\.$/,
    );
    expect(oooSummary({ ...status, oooUntil: until })).toMatch(/from your Google Calendar\.$/);
    expect(statusExpiry(status)).toBeNull();
    expect(statusExpiry({ ...status, customStatusExpiresAt: until })).toMatch(/^Clears /);
  });

  it("describes a session the way the classic sessions page does", () => {
    const now = Date.parse("2026-10-06T12:00:00Z");

    const session = {
      id: 3,
      current: true,
      description: "Firefox on Linux",
      ipAddress: "203.0.113.9",
      lastActiveAt: "2026-10-06T11:55:00Z",
      createdAt: "2026-10-01T09:00:00Z",
    };

    expect(sessionMeta(session, now)).toMatch(
      /^Last active 5 minutes ago · 203\.0\.113\.9 · signed in .*2026$/,
    );
    expect(sessionMeta({ ...session, ipAddress: null }, now)).not.toContain("203");
  });

  it("names devices and connections", () => {
    expect(
      deviceName({ id: 1, endpoint: "e", browser: "Chrome", version: "141", platform: "Android" }),
    ).toBe("Chrome 141 on Android");
    expect(
      connectionSummary("GitHub", {
        state: "connected",
        name: "ada",
        workspace: null,
        appToken: true,
      }),
    ).toBe("Connected as ada through the GitHub App.");
    expect(
      connectionSummary("Fizzy", {
        state: "connected",
        name: "ada",
        workspace: "37s",
        appToken: false,
      }),
    ).toBe("Connected as ada (37s).");
    expect(connectionSummary("Fizzy", { state: "rejected", reason: "revoked" })).toBe(
      "Fizzy rejected the connection (revoked). Paste a new token to reconnect.",
    );
    expect(connectionSummary("GitHub", { state: "rejected", reason: null })).toBe(
      "GitHub rejected the connection. Reconnect below.",
    );
    expect(connectionSummary("GitHub", { state: "missing" })).toBeNull();
  });

  it("keeps classic links on the classic page", () => {
    expect(classicPage("/users/me/profile")).toBe("/users/me/profile?classic=1");
    expect(classicPage("/users/me/profile?tab=x", "github-connection-title")).toBe(
      "/users/me/profile?tab=x&classic=1#github-connection-title",
    );
  });

  it("takes only the changed service from a connection answer, and only dependents from a load", async () => {
    const page = await get<Settings>(harness().server, "/api/v1/settings");
    const { integrations } = page;

    // An older GitHub answer still says Fizzy is missing; a Fizzy connect landed since.
    const current: Settings = {
      ...page,
      integrations: {
        ...integrations,
        fizzy: { state: "connected", name: "Riel", workspace: null, appToken: false },
      },
    };

    const stale: IntegrationChange = {
      notice: "GitHub disconnected.",
      integrations: {
        ...integrations,
        github: { state: "missing" },
        fizzy: { state: "missing" },
      },
    };

    const next = withConnection(current, "github", stale);

    expect(next.integrations.github).toEqual({ state: "missing" });
    expect(next.integrations.fizzy).toEqual(current.integrations.fizzy);

    const fresh: Settings = {
      ...page,
      profile: { ...page.profile, githubLogin: null },
      status: { ...page.status, oooUntil: null },
      integrations: { ...integrations, github: { state: "missing" } },
    };

    const merged = withDependents(next, fresh);

    expect(merged.profile.githubLogin).toBeNull();
    expect(merged.status.oooUntil).toBeNull();
    expect(merged.integrations).toBe(next.integrations);
  });
});
