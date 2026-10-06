/**
 * The stage roster as the classic `rooms/stage/_roster.html` lays it out: hosts, speakers, then
 * listeners with raised hands first (in the order they raised them, numbered as the queue), each
 * group by name otherwise; and the actions a host or administrator has on each row.
 */

import type { HuddleModeration } from "../../gen/HuddleModeration.ts";
import type { StageMember } from "../../gen/StageMember.ts";
import type { StageRole } from "../../gen/StageRole.ts";
import type { StageState } from "../../gen/StageState.ts";

export const STREAM_QUALITIES = ["720p15", "1080p15", "1080p30"] as const;

export const DEFAULT_STREAM_QUALITY = "1080p15";

export const SOLE_HOST_TITLE = "The stage needs at least one host";

export interface StageEntry {
  readonly member: StageMember;
  readonly name: string;
  readonly administrator: boolean;
  /** 1-based place in the raised-hand queue. */
  readonly queue: number | null;
}

export interface StageGroups {
  readonly hosts: readonly StageEntry[];
  readonly speakers: readonly StageEntry[];
  readonly listeners: readonly StageEntry[];
}

export interface StagePerson {
  readonly name: string;
  readonly administrator: boolean;
}

function byName(a: StageEntry, b: StageEntry): number {
  return a.name.toLocaleLowerCase().localeCompare(b.name.toLocaleLowerCase());
}

function handTime(entry: StageEntry): number {
  const raised = entry.member.handRaisedAt;

  return raised === null ? Number.POSITIVE_INFINITY : Date.parse(raised);
}

export function stageGroups(
  stage: StageState,
  person: (userId: number) => StagePerson,
): StageGroups {
  const entries = stage.members.map((member): StageEntry => {
    const { name, administrator } = person(member.userId);

    return { member, name, administrator, queue: null };
  });

  const of = (role: StageRole) => entries.filter((entry) => entry.member.role === role);

  const listeners = of("listener").toSorted((a, b) => {
    const raised = handTime(a) - handTime(b);

    return Number.isNaN(raised) || raised === 0 ? byName(a, b) : raised;
  });

  let queue = 0;

  return {
    hosts: of("host").toSorted(byName),
    speakers: of("speaker").toSorted(byName),
    listeners: listeners.map((entry) => {
      if (entry.member.handRaisedAt === null) {
        return entry;
      }

      queue += 1;

      return { ...entry, queue };
    }),
  };
}

export type StageAction =
  | {
      readonly kind: "role";
      readonly label: string;
      readonly role: StageRole;
      readonly primary: boolean;
      /** Why it can't be done, when it can't. */
      readonly disabled: string | null;
    }
  | { readonly kind: "moderate"; readonly label: string; readonly action: HuddleModeration }
  | { readonly kind: "lower"; readonly label: string };

export interface StageViewer {
  readonly membershipId: number | null;
  readonly role: StageRole | null;
  readonly administrator: boolean;
}

/** Hosts and administrators manage the stage. */
export function canManageStage(viewer: StageViewer): boolean {
  return viewer.role === "host" || viewer.administrator;
}

function role(label: string, to: StageRole, primary = false, disabled: string | null = null) {
  return { kind: "role", label, role: to, primary, disabled } as const;
}

/**
 * A row's actions for a manager (none for anyone else). Moderation never targets oneself, and
 * only administrators moderate administrators; an administrator may lift their own mute.
 * Removing someone needs them in the call.
 */
export function stageActions(
  entry: StageEntry,
  viewer: StageViewer,
  hostCount: number,
  inCall: boolean,
): StageAction[] {
  if (!canManageStage(viewer)) {
    return [];
  }

  const { member } = entry;
  const self = member.membershipId === viewer.membershipId;
  const moderate = !self && (viewer.administrator || !entry.administrator);
  const selfUnmute = self && viewer.administrator && member.serverMuted;
  const actions: StageAction[] = [];

  if (member.role === "host") {
    const sole = hostCount <= 1 ? SOLE_HOST_TITLE : null;

    actions.push(role("Move to speakers", "speaker", false, sole));
    actions.push(role("Move to audience", "listener", false, sole));
  } else if (member.role === "speaker") {
    actions.push(role("Move to audience", "listener"));
    actions.push(role("Make host", "host"));
  } else {
    const raised = member.handRaisedAt !== null;

    actions.push(role("Invite to speak", "speaker", raised));

    if (raised) {
      actions.push({ kind: "lower", label: "Lower hand" });
    }

    actions.push(role("Make host", "host"));
  }

  if (moderate) {
    if (member.role !== "listener") {
      actions.push(
        member.serverMuted
          ? { kind: "moderate", label: "Allow to speak", action: "unmute" }
          : { kind: "moderate", label: "Mute for everyone", action: "mute" },
      );
    }

    if (inCall) {
      actions.push({ kind: "moderate", label: "Remove from call", action: "disconnect" });
    }
  } else if (selfUnmute) {
    actions.push({ kind: "moderate", label: "Unmute yourself", action: "unmute" });
  }

  return actions;
}
