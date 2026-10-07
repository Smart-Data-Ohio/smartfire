/** Work facts, details, permissions and list fixtures for the work tests. */

import { userFixture } from "../../api/testing.ts";
import type { ThreadDetail } from "../../gen/ThreadDetail.ts";
import type { ThreadPermissions } from "../../gen/ThreadPermissions.ts";
import type { User } from "../../gen/User.ts";
import type { WorkDetail } from "../../gen/WorkDetail.ts";
import type { WorkFacts } from "../../gen/WorkFacts.ts";
import type { WorkHistoryEntry } from "../../gen/WorkHistoryEntry.ts";
import type { WorkLink } from "../../gen/WorkLink.ts";
import type { WorkListRow } from "../../gen/WorkListRow.ts";
import { threadFixture } from "../threads/test-fixtures.ts";

/** Agent 9, "Ember", a workspace agent. */
export function agentFixture(id = 9, name = "Ember"): User {
  return {
    ...userFixture(id, name),
    agent: { agentId: id, kind: "workspace", status: "idle", suspended: false },
  };
}

/** The people and agent the work fixtures name: users 2 and 3, and agent 9. */
export const WORK_USERS: readonly User[] = [userFixture(2), userFixture(3), agentFixture()];

export function factsFixture(extra: Partial<WorkFacts> = {}): WorkFacts {
  return {
    status: "in_progress",
    owner: userFixture(2),
    ownerActive: true,
    runUrl: null,
    resultUpdatedAt: null,
    links: [],
    ...extra,
  };
}

export function linkFixture(id: number, extra: Partial<WorkLink> = {}): WorkLink {
  return {
    id,
    kind: "pull_request",
    label: `acme/app#${id}`,
    url: `https://github.com/acme/app/pull/${id}`,
    pullRequestState: "open",
    title: `Change ${id}`,
    eventStartsAt: null,
    eventTimeZone: null,
    eventCancelled: false,
    ...extra,
  };
}

export function historyFixture(
  id: number,
  extra: Partial<WorkHistoryEntry> = {},
): WorkHistoryEntry {
  return {
    id,
    kind: "update",
    createdAt: "2026-10-06T09:00:00.000Z",
    actorId: 2,
    fromStatus: "planned",
    toStatus: "in_progress",
    fromOwner: null,
    toOwner: { userId: 2, name: "User 2" },
    note: null,
    handoff: null,
    ...extra,
  };
}

export function workDetailFixture(extra: Partial<WorkDetail> = {}): WorkDetail {
  return {
    resultMarkdown: null,
    resultHtml: null,
    resultUpdatedById: null,
    steps: [],
    history: [],
    ownerCandidates: [
      { userId: 2, provider: null, description: null },
      { userId: 3, provider: null, description: null },
      { userId: 9, provider: "Anthropic", description: "Workspace agent" },
    ],
    handoffReceivers: [{ agentId: 9, userId: 9 }],
    ...extra,
  };
}

/** Every flag on, as a moderator sees a tracked thread; override the ones a test needs. */
export function workPermissionsFixture(extra: Partial<ThreadPermissions> = {}): ThreadPermissions {
  return {
    canRename: true,
    canClose: true,
    canReopen: false,
    canLock: true,
    canUnlock: false,
    canDelete: true,
    canConvertWork: false,
    canManageWork: true,
    canUpdateWorkStatus: true,
    canAssignWork: true,
    canRemoveWork: true,
    ...extra,
  };
}

/** A thread's detail: tracked with `facts` and `work`, or untracked when both are `null`. */
export function threadDetailFixture(
  id: number,
  facts: WorkFacts | null,
  work: WorkDetail | null,
  permissions: ThreadPermissions = workPermissionsFixture(),
): ThreadDetail {
  return {
    thread: threadFixture(id, { work: facts }),
    membership: null,
    parentMessage: null,
    permissions,
    work,
    users: work === null ? [] : [...WORK_USERS],
  };
}

export function rowFixture(id: number, extra: Partial<WorkListRow> = {}): WorkListRow {
  return {
    thread: threadFixture(id, { work: factsFixture() }),
    roomName: "general",
    board: false,
    updatedAt: "2026-10-06T09:00:00.000Z",
    ...extra,
  };
}

/**
 * The `"unknown"` a tolerant field decodes to (a status, kind or state added after this build),
 * which the generated types leave out.
 */
export function tolerated<T extends string>(): T {
  // SAFETY: tolerantLiterals decodes any unrecognised wire value to "unknown", so the store can
  // hold it in a field whose generated type lists only the known values.
  return "unknown" as T;
}
