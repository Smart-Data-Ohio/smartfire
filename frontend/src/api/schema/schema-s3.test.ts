import { describe, expect, it } from "@effect/vitest";
import { DateTime, Schema } from "effect";
import { SavedChanged } from "./actions.ts";
import {
  ActivityEventType,
  ActivityItem,
  ActivityItemChanged,
  ActivityItemRemoved,
  ActivityList,
  ActivitySourceType,
  ActivityTab,
  ActivityUnreadCount,
  UpdateActivityItem,
} from "./activity.ts";
import {
  CreatePoll,
  EventAttendance,
  FizzyCardPreview,
  GithubPullRequestCard,
  MessageCards,
  PollResults,
  RespondToEvent,
  VotePoll,
} from "./cards.ts";
import {
  ScheduledMessageFilter,
  ScheduledMessageList,
  ScheduledMessageRemoved,
  ScheduledMessageState,
  UpdateScheduledMessage,
} from "./composer.ts";
import { MessageDTO } from "./message.ts";
import {
  AssignRoomCategory,
  CreateRoomCategory,
  FavoriteList,
  MoveFavorite,
  ReorderRoomCategories,
  RoomCategoryList,
  RoomCategoryRemoved,
  UpdateInvolvement,
  UpdateRoomCategory,
} from "./organize.ts";
import { SavedFilter, SavedItemList, UpdateSavedItem } from "./saved.ts";
import { RecentSearchList, RecordSearch, SearchOperator, SearchResults } from "./search.ts";
import { ServerFrame } from "./sync.ts";

// The wire JSON below mirrors crates/api_types/src/tests_s3.rs.
const userJson = {
  id: 7,
  name: "Ada Lovelace",
  role: "administrator",
  status: "active",
  bio: null,
  avatarUrl: "/users/7/avatar?v=1700000000",
  customStatus: null,
  createdAt: "2026-09-26T12:26:46.848Z",
} as const;

const messageJson = {
  id: 9001,
  roomId: 12,
  threadId: null,
  creatorId: 7,
  clientMessageId: "4f1c7a0e-5b0e-4c55-9d0a-6f3b2d1e8c11",
  bodyHtml: "<p>Hello <strong>there</strong></p>",
  markdownSource: "Hello **there**",
  systemNote: false,
  action: false,
  streaming: false,
  embedsSuppressed: false,
  replyToMessageId: null,
  forwardedFromMessageId: null,
  forwardedAt: null,
  forwardNote: null,
  editedAt: null,
  attachment: null,
  reactions: [],
  boosts: [],
  pinned: false,
  thread: null,
  poll: null,
  cards: [],
  createdAt: "2026-10-06T09:15:00.123Z",
  updatedAt: "2026-10-06T09:15:00.123Z",
} as const;

const activityItemJson = {
  id: 301,
  eventType: "mention",
  state: "unread",
  readAt: null,
  handledAt: null,
  createdAt: "2026-10-06T09:15:01.000Z",
  updatedAt: "2026-10-06T09:15:01.000Z",
  source: {
    sourceType: "message",
    sourceId: 9001,
    roomId: 12,
    threadId: null,
    messageId: 9001,
    eventId: null,
    creatorId: 7,
    title: "general",
    body: "Hello there @Grace",
    occurredAt: "2026-10-06T09:15:00.123Z",
    status: null,
    path: "/rooms/12/@9001",
  },
} as const;

const conversationJson = {
  roomId: 12,
  threadId: 88,
  roomKind: "open",
  roomName: "general",
  roomIconName: "campfire",
  threadName: "Hello there",
} as const;

const savedItemJson = {
  id: 41,
  messageId: 9001,
  status: "in_progress",
  remindAt: "2026-10-07T09:00:00.000Z",
  remindedAt: null,
  createdAt: "2026-10-06T09:20:00.000Z",
} as const;

const pollJson = {
  id: 5,
  messageId: 9001,
  multiple: false,
  anonymous: false,
  closesAt: "2026-10-08T17:00:00.000Z",
  closedAt: null,
  closed: false,
  totalVotes: 3,
  options: [
    { id: 51, label: "Tacos", votes: 2, voterIds: [7, 8] },
    { id: 52, label: "Pizza", votes: 1, voterIds: [9] },
  ],
} as const;

const scheduledJson = {
  id: 4,
  roomId: 12,
  threadId: null,
  replyToMessageId: null,
  markdownSource: "Standup in 5",
  sendAt: "2026-10-07T13:55:00.000Z",
  state: "dropped",
  sendable: false,
  sentAt: null,
  sentMessageId: null,
  droppedAt: "2026-10-07T13:55:12.000Z",
  dropReason: "its room was deleted",
  createdAt: "2026-10-06T11:00:00.000Z",
} as const;

const categoryJson = { id: 3, name: "Projects", collapsed: false, position: 1 } as const;

const cardsJson = [
  {
    kind: "github",
    data: {
      pullRequestId: 14,
      owner: "Smart-Data-Ohio",
      repo: "smartfire",
      number: 280,
      url: "https://github.com/Smart-Data-Ohio/smartfire/pull/280",
    },
  },
  {
    kind: "x",
    data: {
      fetch: "loaded",
      postId: "1843000000000000001",
      url: "https://x.com/ada/status/1843000000000000001",
      authorName: "Ada",
      authorHandle: "ada",
      authorAvatarUrl: "https://pbs.twimg.com/ada.jpg",
      text: "Shipped",
      postedAt: "2026-10-06T08:00:00.000Z",
      replies: 2,
      reposts: 1,
      likes: 30,
      media: [
        {
          kind: "photo",
          url: "https://pbs.twimg.com/media/1.jpg",
          thumbnailUrl: null,
          width: 1200,
          height: 675,
          alt: null,
        },
      ],
      quote: { url: null, authorName: "Grace", authorHandle: "grace", text: "Nice" },
    },
  },
  {
    kind: "event",
    data: {
      eventId: 21,
      roomId: 12,
      title: "Retro",
      organizerId: 7,
      startsAt: "2026-10-09T15:00:00.000Z",
      endsAt: "2026-10-09T16:00:00.000Z",
      timeZone: "America/New_York",
      recurring: true,
      cancelled: false,
      venueRoomId: 30,
      venueName: "Lounge",
      meetLink: null,
    },
  },
  {
    kind: "fizzy",
    data: {
      fizzyCardId: 2,
      accountId: "897362094",
      number: 17,
      url: "https://app.fizzy.do/897362094/cards/17",
    },
  },
  {
    kind: "linkedin",
    data: {
      url: "https://www.linkedin.com/posts/ada_activity-1",
      title: null,
      description: null,
      imageUrl: null,
      embedUrl: "https://www.linkedin.com/embed/feed/update/urn:li:activity:1",
    },
  },
  {
    kind: "link",
    data: {
      url: "https://example.com/post",
      siteName: "Example",
      title: "A post",
      description: null,
      imageUrl: "https://example.com/og.png",
    },
  },
] as const;

/** Decoding then encoding gives back exactly the wire JSON. */
const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

const accepts = <S extends Schema.Codec<unknown, unknown>>(
  schema: S,
  values: readonly S["Encoded"][],
) => {
  for (const value of values) roundTrips(schema, value);
};

describe("S3 DTO schemas", () => {
  it("round-trip the activity inbox", () => {
    accepts(ActivityEventType, [
      "mention",
      "reply",
      "thread_activity",
      "keyword_alert",
      "work_update",
      "work_assignment",
      "work_sla",
      "huddle_started",
      "huddle_missed",
      "event_invitation",
      "event_update",
      "event_cancelled",
      "event_reminder",
      "pr_review_request",
      "agent_approval_request",
      "agent_budget_exceeded",
      "message_reminder",
      "scheduled_message_dropped",
      "two_factor_lockout",
      "new_sign_in",
    ]);
    accepts(ActivitySourceType, [
      "message",
      "saved_item",
      "work_thread_event",
      "board_sla_nudge",
      "huddle_grant",
      "event",
      "agent_approval",
      "agent_budget_notice",
      "scheduled_message",
      "two_factor_credential",
      "session",
    ]);
    accepts(ActivityTab, [
      "all",
      "mentions",
      "threads",
      "events",
      "agents",
      "github",
      "huddles",
      "reminders",
      "security",
    ]);
    expect(() => Schema.decodeUnknownSync(ActivityEventType)("newSignIn")).toThrowError();

    roundTrips(ActivityList, {
      items: [activityItemJson],
      users: [userJson],
      unreadCount: 4,
      nextCursor: 301,
    });
    roundTrips(ActivityItem, {
      ...activityItemJson,
      eventType: "new_sign_in",
      state: "handled",
      readAt: "2026-10-06T10:00:00.000Z",
      handledAt: "2026-10-06T10:00:00.000Z",
      source: null,
    });
    roundTrips(ActivityUnreadCount, { unreadCount: 4 });
    roundTrips(UpdateActivityItem, { state: "handled" });
    roundTrips(ActivityItemChanged, { item: activityItemJson, unreadCount: 3 });
    roundTrips(ActivityItemRemoved, { id: 301, unreadCount: 2 });

    const item = Schema.decodeUnknownSync(ActivityItem)(activityItemJson);

    expect(DateTime.toEpochMillis(item.updatedAt)).toBe(Date.UTC(2026, 9, 6, 9, 15, 1));
  });

  it("round-trip saved items and scheduled messages", () => {
    accepts(SavedFilter, ["all", "in_progress", "done"]);
    roundTrips(SavedItemList, {
      items: [savedItemJson],
      messages: [messageJson],
      users: [userJson],
      conversations: [conversationJson],
      nextCursor: null,
    });
    roundTrips(UpdateSavedItem, { status: "done" });
    roundTrips(SavedChanged, { messageId: 9001, item: savedItemJson });
    roundTrips(SavedChanged, { messageId: 9001, item: null });

    accepts(ScheduledMessageState, ["pending", "sending", "sent", "dropped"]);
    accepts(ScheduledMessageFilter, ["pending", "past"]);
    roundTrips(ScheduledMessageList, {
      scheduledMessages: [scheduledJson],
      conversations: [{ ...conversationJson, threadId: null, threadName: null }],
      nextCursor: 4,
    });
    roundTrips(UpdateScheduledMessage, {
      markdownSource: null,
      sendAt: "2026-10-07T14:00:00.000Z",
    });
    roundTrips(ScheduledMessageRemoved, { id: 4, roomId: 12 });
  });

  it("round-trip search results and recent searches", () => {
    accepts(SearchOperator, ["from", "in", "has", "before", "after", "on", "is"]);
    roundTrips(SearchResults, {
      query: "launch from:@ada has:file",
      chips: [
        {
          operator: "from",
          value: "ada",
          token: "from:@ada",
          label: "from: ada",
          removeQuery: "launch has:file",
        },
        {
          operator: "has",
          value: "file",
          token: "has:file",
          label: "has: file",
          removeQuery: "launch from:@ada",
        },
      ],
      messages: [{ ...messageJson, threadId: 88 }],
      users: [userJson],
      conversations: [conversationJson],
      before: 9001,
      sections: [
        {
          kind: "work_threads",
          rows: [
            {
              id: 88,
              roomId: 12,
              roomKind: "open",
              title: "Launch checklist",
              time: "2026-10-06T10:00:00.000Z",
              workStatus: "in_progress",
              cancelled: false,
            },
          ],
        },
      ],
    });
    roundTrips(RecentSearchList, {
      searches: [{ id: 6, query: "launch", searchedAt: "2026-10-06T10:05:00.000Z" }],
    });
    roundTrips(RecordSearch, { query: "launch" });
  });

  it("round-trip the sidebar organisation requests", () => {
    roundTrips(CreateRoomCategory, { name: "Projects", collapsed: null });
    roundTrips(UpdateRoomCategory, { name: null, collapsed: true });
    roundTrips(ReorderRoomCategories, { categoryIds: [4, 3] });
    roundTrips(RoomCategoryList, { categories: [categoryJson] });
    roundTrips(RoomCategoryRemoved, { id: 3 });
    roundTrips(AssignRoomCategory, { roomCategoryId: null });
    roundTrips(MoveFavorite, { position: 0 });
    roundTrips(FavoriteList, { rows: [] });
    roundTrips(UpdateInvolvement, { involvement: "muted" });
  });

  it("round-trip polls and attendance", () => {
    const question = Schema.decodeUnknownSync(MessageDTO)({ ...messageJson, poll: pollJson });

    expect(question.poll?.options.map((option) => option.votes)).toEqual([2, 1]);
    roundTrips(MessageDTO, { ...messageJson, poll: pollJson });
    roundTrips(PollResults, {
      poll: {
        ...pollJson,
        anonymous: true,
        options: pollJson.options.map((option) => ({ ...option, voterIds: [] })),
      },
      myOptionIds: [51],
    });
    roundTrips(CreatePoll, {
      question: "Lunch?",
      options: ["Tacos", "Pizza"],
      multiple: false,
      anonymous: false,
      closesAt: null,
    });
    roundTrips(VotePoll, { optionIds: [] });
    roundTrips(EventAttendance, {
      eventId: 21,
      response: "maybe",
      goingCount: 4,
      maybeCount: 1,
      declinedCount: 0,
      respondable: true,
      canApplyToFuture: true,
    });
    roundTrips(RespondToEvent, { response: "declined", applyToFuture: false });
  });

  it("round-trip every card kind and the per-viewer previews", () => {
    const carded = Schema.decodeUnknownSync(MessageDTO)({ ...messageJson, cards: cardsJson });

    expect(carded.cards.map((card) => card.kind)).toEqual([
      "github",
      "x",
      "event",
      "fizzy",
      "linkedin",
      "link",
    ]);
    expect(Schema.encodeSync(MessageDTO)(carded)).toEqual({ ...messageJson, cards: cardsJson });
    roundTrips(MessageCards, { messageId: 9001, roomId: 12, threadId: null, cards: cardsJson });
    expect(() =>
      Schema.decodeUnknownSync(MessageCards)({
        messageId: 9001,
        roomId: 12,
        threadId: null,
        cards: [{ kind: "youtube", data: {} }],
      }),
    ).toThrowError();

    roundTrips(GithubPullRequestCard, { state: "hidden" });
    roundTrips(GithubPullRequestCard, { state: "loading" });
    roundTrips(GithubPullRequestCard, { state: "failed", message: "Not Found" });
    roundTrips(GithubPullRequestCard, {
      state: "loaded",
      owner: "Smart-Data-Ohio",
      repo: "smartfire",
      number: 280,
      title: "Build the S2 client",
      url: "https://github.com/Smart-Data-Ohio/smartfire/pull/280",
      status: "merged",
      authorLogin: "ada",
      authorAvatarUrl: null,
      baseBranch: "main",
      headBranch: "frontend/s2-ui",
      review: "changes_requested",
      checks: "passing",
      githubUpdatedAt: "2026-10-06T12:00:00.000Z",
      discussionThreadId: null,
    });

    roundTrips(FizzyCardPreview, { state: "not_connected" });
    roundTrips(FizzyCardPreview, { state: "not_found" });
    roundTrips(FizzyCardPreview, { state: "loading" });
    roundTrips(FizzyCardPreview, { state: "failed", message: "timeout" });
    roundTrips(FizzyCardPreview, {
      state: "loaded",
      title: "Ship S3",
      url: "https://app.fizzy.do/897362094/cards/17",
      boardName: "Front end",
      status: "column",
      columnName: "Doing",
      assignees: [{ name: "Ada", avatarUrl: null }],
      hasMoreAssignees: false,
      tags: ["s3"],
      stepsTotal: 4,
      stepsCompleted: 1,
      lastActiveAt: null,
    });
  });
});

describe("S3 sync events", () => {
  it("round-trip in a batch", () => {
    const events = [
      { type: "activity.item", data: { item: activityItemJson, unreadCount: 3 } },
      { type: "activity.removed", data: { id: 301, unreadCount: 2 } },
      { type: "scheduled.changed", data: scheduledJson },
      { type: "scheduled.removed", data: { id: 4, roomId: 12 } },
      { type: "sidebar.category.upserted", data: categoryJson },
      { type: "sidebar.category.removed", data: { id: 3 } },
      { type: "poll.updated", data: pollJson },
      { type: "message.cards", data: { messageId: 9001, roomId: 12, threadId: null, cards: [] } },
      { type: "saved.changed", data: { messageId: 9001, item: savedItemJson } },
    ].map((event, index) => ({
      seq: index + 1,
      topic: event.type === "poll.updated" || event.type === "message.cards" ? "room:12" : "user",
      ...event,
    }));

    const wire = { t: "batch", events } as const;
    const frame = Schema.decodeUnknownSync(ServerFrame)(wire);

    expect(frame.t === "batch" && frame.events.map((event) => event.type)).toEqual(
      events.map((event) => event.type),
    );
    expect(Schema.encodeSync(ServerFrame)(frame)).toEqual(wire);
  });
});
