import type { WorkStatus } from "../../src/gen/WorkStatus.ts";
import { renderMarkdown } from "../markdown.ts";
import { createRandom } from "../random.ts";
import { buildMessage, iso, plainDraft, type ThreadRecord } from "../s2/model.ts";
import { BOT_ID, DEACTIVATED_ID, seededUuid, USER_IDS, VIEWER_ID, type World } from "../seed.ts";
import { emptyWorkDetail, newWorkFacts } from "./work.ts";

export const BOARD_ROOM_ID = 900;

/** How many messages the long discussion (Keyboard navigation in dialogs) holds. */
export const LONG_DISCUSSION = 64;

export const BOARD_TAG_RULE_IDS = { bug: 9101, design: 9102 } as const;

export const BOARD_POST_IDS = {
  onboardingChecklist: 9001,
  launchWeek: 9002,
  pricingPage: 9003,
  apiPagination: 9004,
  retryBug: 9005,
  keyboardNavigation: 9006,
  backupRestore: 9007,
  rateLimits: 9008,
  searchIndex: 9009,
  iconRefresh: 9010,
  releaseNotes: 9011,
  migrateWorkers: 9012,
} as const;

const POSTS: readonly {
  id: number;
  name: string;
  status: WorkStatus;
  owner: number | null;
  tags: string[];
  hoursAgo: number;
}[] = [
  {
    id: 9001,
    name: "Onboarding checklist in the sidebar",
    status: "in_progress",
    owner: VIEWER_ID,
    tags: ["design"],
    hoursAgo: 3,
  },
  {
    id: 9002,
    name: "Launch week plan",
    status: "planned",
    owner: USER_IDS.maya,
    tags: ["infra"],
    hoursAgo: 20,
  },
  {
    id: 9003,
    name: "Pricing page refresh",
    status: "done",
    owner: USER_IDS.lucia,
    tags: ["design"],
    hoursAgo: 72,
  },
  {
    id: 9004,
    name: "Cursor pagination for the API",
    status: "planned",
    owner: null,
    tags: ["api"],
    hoursAgo: 1,
  },
  {
    id: 9005,
    name: "Fix retries duplicating sends",
    status: "blocked",
    owner: BOT_ID,
    tags: ["api", "bug"],
    hoursAgo: 6,
  },
  {
    id: 9006,
    name: "Keyboard navigation in dialogs",
    status: "in_progress",
    owner: USER_IDS.maya,
    tags: ["design"],
    hoursAgo: 8,
  },
  {
    id: 9007,
    name: "Verify backup restore",
    status: "blocked",
    owner: DEACTIVATED_ID,
    tags: ["infra"],
    hoursAgo: 48,
  },
  {
    id: 9008,
    name: "Publish rate limit guidance",
    status: "done",
    owner: BOT_ID,
    tags: ["api"],
    hoursAgo: 12,
  },
  {
    id: 9009,
    name: "Rebuild the search index",
    status: "planned",
    owner: VIEWER_ID,
    tags: ["infra"],
    hoursAgo: 10,
  },
  {
    id: 9010,
    name: "Refresh workspace icons",
    status: "done",
    owner: null,
    tags: ["design"],
    hoursAgo: 30,
  },
  {
    id: 9011,
    name: "Prepare release notes",
    status: "in_progress",
    owner: null,
    tags: [],
    hoursAgo: 15,
  },
  {
    id: 9012,
    name: "Migrate background workers",
    status: "blocked",
    owner: VIEWER_ID,
    tags: ["bug", "infra"],
    hoursAgo: 4,
  },
];

/** Appended after the earlier slices so every existing seed id and random sequence stays stable. */
export function seedBoards(world: World, now: number, seed: number): void {
  const random = createRandom(seed * 104729 + 19);
  const at = iso(now - 30 * 86400000);
  const memberIds = [VIEWER_ID, USER_IDS.maya, USER_IDS.lucia, USER_IDS.priya, BOT_ID];
  const people = [...world.users.values()].map((user) => ({ id: user.id, name: user.name }));
  world.rooms.set(BOARD_ROOM_ID, {
    room: {
      id: BOARD_ROOM_ID,
      kind: "board",
      name: "Roadmap",
      iconName: null,
      creatorId: VIEWER_ID,
      createdAt: at,
      updatedAt: at,
    },
    memberIds,
    membership: {
      id: 9900,
      roomId: BOARD_ROOM_ID,
      userId: VIEWER_ID,
      involvement: "everything",
      unreadAt: null,
      lastReadMessageId: null,
      roomCategoryId: null,
      favoritePosition: null,
      stageRole: null,
    },
    messages: [],
    mentionCount: 0,
    boardAutomations: {
      tagRules: [
        { id: BOARD_TAG_RULE_IDS.bug, tag: "bug", assigneeId: BOT_ID },
        { id: BOARD_TAG_RULE_IDS.design, tag: "design", assigneeId: USER_IDS.maya },
      ],
      slaTimers: [{ status: "planned", nudgeAfterMinutes: 1440, escalateAfterMinutes: 2880 }],
      nextTagRuleId: 9103,
    },
    boardDigest: {
      date: iso(now).slice(0, 10),
      text: "Verify backup restore and Fix retries duplicating sends are waiting for their owners.",
    },
  });

  for (const [id, title, hours] of [
    [9104, "API review", 24],
    [9105, "Roadmap planning", 48],
    [9106, "Launch readiness", 72],
  ] as const) {
    world.workLinkEvents.set(id, {
      id,
      title,
      roomId: BOARD_ROOM_ID,
      startsAt: iso(now + hours * 3600000),
      endsAt: iso(now + (hours + 1) * 3600000),
      timeZone: "America/New_York",
      cancelled: false,
    });
  }

  for (const post of POSTS) {
    const createdAt = iso(now - (post.hoursAgo + 24) * 3600000);
    const lastActivityAt = iso(now - post.hoursAgo * 3600000);
    const owner = post.owner === null ? null : (world.users.get(post.owner) ?? null);

    const facts = {
      ...newWorkFacts(post.status),
      owner,
      ownerActive: owner !== null && memberIds.includes(owner.id),
      tags: post.tags,
    };

    const result =
      post.status === "done" ? `Shipped **${post.name}**. Verified with the team.` : null;

    const thread: ThreadRecord = {
      id: post.id,
      roomId: BOARD_ROOM_ID,
      isBoard: true,
      parentMessageId: null,
      creatorId: post.id % 2 === 0 ? USER_IDS.maya : VIEWER_ID,
      name: post.name,
      closed: post.id === 9003,
      locked: post.id === 9010,
      lastActivityAt,
      autoArchiveAfterMinutes: 1440,
      createdAt,
      updatedAt: lastActivityAt,
      messages: [],
      memberIds: new Set([VIEWER_ID, USER_IDS.maya]),
      viewerMembership: {
        threadId: post.id,
        involvement: "everything",
        unreadAt: null,
        joinedAt: createdAt,
      },
      work: {
        ...facts,
        resultUpdatedAt: result === null ? null : lastActivityAt,
        runUrl: post.id === 9005 ? "https://ci.example.com/runs/42" : null,
      },
      workDetail: {
        ...emptyWorkDetail,
        resultMarkdown: result,
        resultHtml: result === null ? null : renderMarkdown(result, people),
        resultUpdatedById: result === null ? null : VIEWER_ID,
        history: [
          ...(result === null
            ? []
            : [
                {
                  id: post.id * 10 + 2,
                  kind: "result" as const,
                  createdAt: lastActivityAt,
                  actorId: VIEWER_ID,
                  fromStatus: post.status,
                  toStatus: post.status,
                  fromOwner: owner === null ? null : { userId: owner.id, name: owner.name },
                  toOwner: owner === null ? null : { userId: owner.id, name: owner.name },
                  note: null,
                  handoff: null,
                },
              ]),
          {
            id: post.id * 10 + 1,
            kind: "update",
            createdAt,
            actorId: post.id === 9007 ? null : VIEWER_ID,
            fromStatus: null,
            toStatus: post.status,
            fromOwner: null,
            toOwner: owner === null ? null : { userId: owner.id, name: owner.name },
            note: post.id === 9007 ? "Waiting for infrastructure access" : null,
            handoff: null,
          },
        ],
      },
    };

    const lines = [
      `Brief: ${post.name}. Discuss the approach here.`,
      "I have checked the current behavior.",
      "The acceptance criteria are in the linked notes.",
      // A discussion longer than one page of replies, so the post opens short of its start.
      ...(post.id === BOARD_POST_IDS.keyboardNavigation
        ? Array.from({ length: LONG_DISCUSSION - 3 }, (_, at) => `Focus order note ${at + 1}.`)
        : []),
    ];

    for (const [index, markdown] of lines.entries()) {
      thread.messages.push(
        buildMessage(
          800_000 + (post.id - 9000) * 100 + index,
          BOARD_ROOM_ID,
          post.id,
          plainDraft(index === 0 ? thread.creatorId : USER_IDS.maya, markdown, seededUuid(random)),
          index === 0
            ? createdAt
            : iso(Date.parse(lastActivityAt) - (lines.length - 1 - index) * 60000),
          people,
        ),
      );
    }

    if (post.id === 9005 && thread.work !== null && thread.work !== undefined) {
      thread.work.links = [
        {
          id: 90001,
          kind: "pull_request",
          label: "smartfire/smartfire#42",
          url: "https://github.com/smartfire/smartfire/pull/42",
          pullRequestState: "open",
          title: "Deduplicate sends",
          eventStartsAt: null,
          eventTimeZone: null,
          eventCancelled: false,
        },
        {
          id: 90002,
          kind: "event",
          label: "API review",
          url: "/rooms/900/events/9104",
          pullRequestState: null,
          title: null,
          eventStartsAt: iso(now + 86400000),
          eventTimeZone: "America/New_York",
          eventCancelled: false,
        },
        {
          id: 90003,
          kind: "drive_file",
          label: "Retry design notes",
          url: "https://docs.google.com/document/d/retry-notes/edit",
          pullRequestState: null,
          title: null,
          eventStartsAt: null,
          eventTimeZone: null,
          eventCancelled: false,
        },
      ];

      if (thread.workDetail != null) {
        const step = (id: number, name: string, status: "done" | "running", position: number) => ({
          id,
          messageId: null,
          threadId: post.id,
          name,
          status,
          inputSummary: position === 0 ? "send_message retries after a timeout" : null,
          outputSummary: position === 0 ? "Two sends share one client id" : null,
          durationMs: position === 0 ? 1250 : null,
          position,
          createdAt: lastActivityAt,
          updatedAt: lastActivityAt,
        });

        thread.workDetail = {
          ...thread.workDetail,
          steps: [
            step(90011, "Reproduce the duplicate", "done", 0),
            step(90012, "Write the fix", "running", 1),
          ],
        };
      }
    }

    world.threads.set(post.id, thread);
  }

  world.nextThreadId = Math.max(world.nextThreadId, 9013);
}
