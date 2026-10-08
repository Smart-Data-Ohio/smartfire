import { rowVersionFixture, userFixture } from "../api/testing.ts";
import type { BoardListing } from "../gen/BoardListing.ts";
import type { Thread } from "../gen/Thread.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { WorkStatus } from "../gen/WorkStatus.ts";

export const BOARD = 900;

export function boardThread(
  id: number,
  status: WorkStatus = "planned",
  ownerId: number | null = null,
  tags: string[] = [],
  revision = 0,
): Thread {
  return {
    id,
    roomId: BOARD,
    parentMessageId: null,
    creatorId: 7,
    name: `Post ${id}`,
    status: "active",
    replyCount: 0,
    lastActivityAt: "2026-10-07T10:00:00.000Z",
    autoArchiveAfterMinutes: 1440,
    createdAt: "2026-10-07T09:00:00.000Z",
    work: {
      status,
      owner: ownerId === null ? null : userFixture(ownerId),
      ownerActive: ownerId !== null,
      runUrl: null,
      resultUpdatedAt: null,
      links: [],
      tags,
      messageCount: 0,
      updatedAt: rowVersionFixture(Date.UTC(2026, 9, 7, 10) + revision),
    },
  };
}

export function boardListing(threads: Thread[] = [boardThread(1)]): BoardListing {
  return {
    roomId: BOARD,
    status: "all",
    owner: "anyone",
    tag: "",
    page: 1,
    posts: threads.map((thread) => ({ thread, membership: null })),
    hasMore: false,
    anyPosts: true,
    ownerOptions: [{ userId: 7, agent: false }],
    tagCounts: [{ name: "api", count: 2 }],
    digest: { date: "2026-10-07", text: "Waiting for review" },
    canAdminister: true,
    users: [userFixture(7)],
  };
}

export function boardDetail(thread = boardThread(1)): ThreadDetail {
  return {
    thread,
    parentMessage: null,
    membership: null,
    permissions: {
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
      canRemoveWork: false,
    },
    work: {
      resultMarkdown: null,
      resultHtml: null,
      resultUpdatedById: null,
      steps: [],
      history: [],
      ownerCandidates: [{ userId: 7, provider: null, description: null }],
      handoffReceivers: [],
    },
    users: [userFixture(7)],
  };
}
