import { describe, expect, it } from "vitest";
import { meFixture } from "../../api/testing.ts";
import type { StageMember } from "../../gen/StageMember.ts";
import { soundsMuted } from "./sounds.ts";
import {
  SOLE_HOST_TITLE,
  type StageEntry,
  type StageViewer,
  stageActions,
  stageGroups,
} from "./stage.ts";

function member(
  membershipId: number,
  role: StageMember["role"],
  change: Partial<StageMember> = {},
): StageMember {
  return {
    membershipId,
    userId: membershipId,
    role,
    handRaisedAt: null,
    serverMuted: false,
    ...change,
  };
}

const NAMES = new Map([
  [1, "zoe"],
  [2, "Adam"],
  [3, "Bea"],
  [4, "Cy"],
  [5, "Dee"],
  [6, "Eli"],
]);

const groups = stageGroups(
  {
    roomId: 12,
    live: null,
    members: [
      member(1, "host"),
      member(2, "host"),
      member(3, "speaker"),
      member(4, "listener"),
      member(5, "listener", { handRaisedAt: "2026-10-06T10:00:05Z" }),
      member(6, "listener", { handRaisedAt: "2026-10-06T10:00:01Z" }),
    ],
  },
  (userId) => ({ name: NAMES.get(userId) ?? "?", administrator: userId === 2 }),
);

function entry(membershipId: number): StageEntry {
  const found = [...groups.hosts, ...groups.speakers, ...groups.listeners].find(
    (candidate) => candidate.member.membershipId === membershipId,
  );

  if (found === undefined) {
    throw new Error(`no member ${membershipId}`);
  }

  return found;
}

const host: StageViewer = { membershipId: 1, role: "host", administrator: false };

function labels(target: StageEntry, viewer: StageViewer, hosts = 2, inCall = true): string[] {
  return stageActions(target, viewer, hosts, inCall).map((action) => action.label);
}

describe("the roster", () => {
  it("groups by role, by name, with raised hands first in queue order", () => {
    expect(groups.hosts.map((entry) => entry.name)).toEqual(["Adam", "zoe"]);
    expect(groups.listeners.map((entry) => [entry.name, entry.queue])).toEqual([
      ["Eli", 1],
      ["Dee", 2],
      ["Cy", null],
    ]);
  });
});

describe("a manager's actions", () => {
  it("offers role moves by group", () => {
    expect(labels(entry(3), host)).toEqual([
      "Move to audience",
      "Make host",
      "Mute for everyone",
      "Remove from call",
    ]);
    expect(labels(entry(4), host)).toEqual(["Invite to speak", "Make host", "Remove from call"]);
    expect(labels(entry(5), host)).toEqual([
      "Invite to speak",
      "Lower hand",
      "Make host",
      "Remove from call",
    ]);
  });

  it("marks a raised hand's invitation as the primary action", () => {
    const invite = stageActions(entry(5), host, 2, true)[0];

    expect(invite?.kind === "role" && invite.primary).toBe(true);
  });

  it("keeps the last host from stepping down", () => {
    const actions = stageActions(entry(1), host, 1, true);

    expect(actions.slice(0, 2).map((action) => action.kind === "role" && action.disabled)).toEqual([
      SOLE_HOST_TITLE,
      SOLE_HOST_TITLE,
    ]);
  });

  it("never moderates oneself, and only administrators moderate administrators", () => {
    expect(labels(entry(1), host)).toEqual(["Move to speakers", "Move to audience"]);
    expect(labels(entry(2), host)).toEqual(["Move to speakers", "Move to audience"]);

    const admin: StageViewer = { membershipId: 9, role: "listener", administrator: true };

    expect(labels(entry(2), admin)).toContain("Mute for everyone");
  });

  it("lets a muted administrator lift their own mute", () => {
    const self = stageGroups(
      { roomId: 12, live: null, members: [member(2, "speaker", { serverMuted: true })] },
      () => ({ name: "Adam", administrator: true }),
    ).speakers[0];

    const viewer: StageViewer = { membershipId: 2, role: "speaker", administrator: true };

    expect(self === undefined ? [] : labels(self, viewer)).toContain("Unmute yourself");
  });

  it("offers removal only to people in the call, and nothing to non-managers", () => {
    expect(labels(entry(3), host, 2, false)).not.toContain("Remove from call");
    expect(labels(entry(3), { membershipId: 4, role: "listener", administrator: false })).toEqual(
      [],
    );
  });
});

describe("sound gates", () => {
  const at = new Date("2026-10-06T15:30:00Z");

  it("is quiet under do-not-disturb until it ends", () => {
    expect(soundsMuted(meFixture, at)).toBe(false);
    expect(soundsMuted({ ...meFixture, doNotDisturb: { enabled: true, until: null } }, at)).toBe(
      true,
    );
    expect(
      soundsMuted(
        { ...meFixture, doNotDisturb: { enabled: true, until: "2026-10-06T15:00:00Z" } },
        at,
      ),
    ).toBe(false);
  });

  it("is quiet inside quiet hours, overnight windows included", () => {
    // 15:30 UTC is 11:30 in New York.
    const quiet = (startMinute: number, endMinute: number) =>
      soundsMuted({ ...meFixture, quietHours: { startMinute, endMinute } }, at);

    expect(quiet(11 * 60, 12 * 60)).toBe(true);
    expect(quiet(12 * 60, 13 * 60)).toBe(false);
    expect(quiet(22 * 60, 12 * 60)).toBe(true);
    expect(quiet(600, 600)).toBe(false);
  });

  it("is quiet while out of office unless notifications are kept", () => {
    const away = (keepNotifications: boolean) =>
      soundsMuted(
        {
          ...meFixture,
          outOfOffice: { until: "2026-10-07T00:00:00Z", note: null, keepNotifications },
        },
        at,
      );

    expect(away(false)).toBe(true);
    expect(away(true)).toBe(false);
  });
});
