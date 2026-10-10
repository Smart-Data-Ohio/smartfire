import { describe, expect, it } from "@effect/vitest";
import { DateTime, Schema } from "effect";
import { CreateForwards, ForwardDestinationList, PinList, SavedItem } from "./actions.ts";
import { CreateUpload, DirectUpload } from "./attachment.ts";
import {
  IconList,
  ScheduledMessageList,
  SlashCommandResult,
  UserSuggestionList,
} from "./composer.ts";
import { CreateDirect, DirectCandidateList, RenameDirect } from "./direct.ts";
import { MessageDTO } from "./message.ts";
import { FileList, MemberList } from "./panes.ts";
import { MessageReactions } from "./reaction.ts";
import { Switcher } from "./switcher.ts";
import { ServerFrame } from "./sync.ts";
import { CreateThread, ThreadCreated, ThreadDetail, ThreadList } from "./thread.ts";

// The wire JSON below mirrors crates/api_types/src/tests_s2.rs.
const userJson = {
  id: 7,
  name: "Ada Lovelace",
  role: "administrator",
  status: "active",
  bio: null,
  avatarUrl: "/users/7/avatar?v=1700000000",
  hasAvatar: true,
  customStatus: null,
  avatarIcon: null,
  agent: null,
  createdAt: "2026-09-26T12:26:46.848Z",
  updatedAt: "2026-09-26T12:26:46.848000Z",
} as const;

const attachmentJson = {
  filename: "roadmap.png",
  contentType: "image/png",
  byteSize: 48213,
  width: 1600,
  height: 900,
  preview: "image",
  url: "/rails/active_storage/blobs/redirect/eyJf--1/roadmap.png",
  downloadUrl: "/rails/active_storage/blobs/redirect/eyJf--1/roadmap.png?disposition=attachment",
  thumbnailUrl: "/rails/active_storage/representations/redirect/eyJf--1/eyJr--2/roadmap.png",
} as const;

const reactionJson = {
  content: "🎉",
  title: "Party popper",
  imageUrl: null,
  reactorIds: [7, 8],
} as const;

const boostJson = {
  id: 55,
  boosterId: 8,
  content: "nice work",
  createdAt: "2026-10-06T09:16:00.000Z",
} as const;

const indicatorJson = {
  threadId: 88,
  replyCount: 3,
  lastReplyAt: "2026-10-06T10:00:00.000Z",
  replierIds: [8, 7],
} as const;

const messageJson = {
  id: 9001,
  roomId: 12,
  threadId: null,
  creatorId: 7,
  clientMessageId: "4f1c7a0e-5b0e-4c55-9d0a-6f3b2d1e8c11",
  sound: null,
  bodyHtml: "<p>Hello <strong>there</strong></p>",
  markdownSource: "Hello **there**",
  systemNote: false,
  action: false,
  streaming: false,
  embedsSuppressed: false,
  replyToMessageId: null,
  forwardedFromMessageId: null,
  forwardedAt: "2026-10-06T09:15:30.000Z",
  forwardNote: "FYI",
  editedAt: null,
  attachment: attachmentJson,
  reactions: [reactionJson],
  boosts: [boostJson],
  pinned: true,
  thread: indicatorJson,
  poll: null,
  cards: [],
  cardsAsOf: "2026-10-06T09:15:00.200Z",
  steps: [],
  createdAt: "2026-10-06T09:15:00.123Z",
  updatedAt: "2026-10-06T09:15:00.123Z",
} as const;

const threadJson = {
  id: 88,
  roomId: 12,
  parentMessageId: 9001,
  creatorId: 7,
  name: "Hello there",
  status: "active",
  replyCount: 3,
  lastActivityAt: "2026-10-06T10:00:00.000Z",
  autoArchiveAfterMinutes: 4320,
  createdAt: "2026-10-06T09:20:00.000Z",
  work: null,
} as const;

const threadMembershipJson = {
  threadId: 88,
  involvement: "everything",
  unreadAt: "2026-10-06T10:00:00.000Z",
  joinedAt: "2026-10-06T09:20:00.000Z",
} as const;

const createMessageJson = {
  clientMessageId: "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c11",
  markdownSource: "First!",
  replyToMessageId: null,
  replyNotifyAuthor: null,
  attachmentSignedId: null,
} as const;

/** Decoding then encoding gives back exactly the wire JSON. */
const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

describe("S2 DTO schemas", () => {
  it("decode a message's attachment, reactions, boosts, pin and thread indicator", () => {
    const message = Schema.decodeUnknownSync(MessageDTO)(messageJson);

    expect(message.attachment?.preview).toBe("image");
    expect(message.reactions[0]?.reactorIds).toEqual([7, 8]);
    expect(message.boosts[0]?.content).toBe("nice work");
    expect(message.thread?.replyCount).toBe(3);
    expect(message.thread && DateTime.toEpochMillis(message.thread.lastReplyAt)).toBe(
      Date.UTC(2026, 9, 6, 10),
    );
    expect(Schema.encodeSync(MessageDTO)(message)).toEqual(messageJson);
  });

  it("round-trip the message actions: reactions, pins, saves, forwards and uploads", () => {
    roundTrips(MessageReactions, {
      messageId: 9001,
      roomId: 12,
      threadId: null,
      reactions: [reactionJson],
      boosts: [boostJson],
      updatedAt: "2026-10-06T09:16:00.000Z",
    });
    roundTrips(PinList, {
      pins: [{ messageId: 9001, pinnerId: 8, pinnedAt: "2026-10-06T11:00:00.000Z" }],
      messages: [messageJson],
      users: [userJson],
    });
    roundTrips(SavedItem, {
      id: 31,
      messageId: 9001,
      status: "in_progress",
      remindAt: null,
      remindedAt: null,
      createdAt: "2026-10-06T11:00:00.000Z",
    });
    roundTrips(ForwardDestinationList, {
      destinations: [
        {
          roomId: 3,
          name: "engineering",
          direct: false,
          threads: [{ id: 88, name: "Hello there", status: "active" }],
        },
      ],
    });
    roundTrips(CreateForwards, {
      note: null,
      destinations: [
        { roomId: 3, threadId: null },
        { roomId: 4, threadId: 88 },
      ],
    });
    roundTrips(CreateUpload, {
      filename: "roadmap.png",
      byteSize: 48213,
      checksum: "1B2M2Y8AsgTpgAmY7PhCfg==",
      contentType: "image/png",
    });
    roundTrips(DirectUpload, {
      signedId: "eyJfcmFpbHMiOnt9--abc",
      uploadUrl: "/rails/active_storage/disk/eyJ0b2tlbiI6MX0",
    });
  });

  it("round-trip the composer's suggestions, commands and scheduled messages", () => {
    roundTrips(UserSuggestionList, {
      suggestions: [{ user: userJson, mentionToken: "@[Ada Lovelace]" }],
    });
    roundTrips(IconList, {
      icons: [
        { name: "tada", title: "Tada", kind: "emoji", character: "🎉", imageUrl: null },
        {
          name: "shipit",
          title: "Ship it",
          kind: "custom",
          character: null,
          imageUrl: "/icons/shipit",
        },
      ],
    });
    roundTrips(SlashCommandResult, { status: "open_poll" });
    roundTrips(SlashCommandResult, { status: "posted", messageId: 9002, notice: null });
    roundTrips(SlashCommandResult, { status: "start_huddle", roomId: 12, roomName: "general" });
    expect(() =>
      Schema.decodeUnknownSync(SlashCommandResult)({ status: "openUrl" }),
    ).toThrowError();
    roundTrips(ScheduledMessageList, {
      scheduledMessages: [
        {
          id: 4,
          roomId: 12,
          threadId: 88,
          replyToMessageId: null,
          replyTarget: null,
          markdownSource: "Standup in 5",
          sendAt: "2026-10-07T13:55:00.000Z",
          state: "pending",
          sendable: true,
          sentAt: null,
          sentMessageId: null,
          droppedAt: null,
          dropReason: null,
          createdAt: "2026-10-06T11:00:00.000Z",
        },
      ],
      conversations: [
        {
          roomId: 12,
          threadId: 88,
          roomKind: "open",
          roomName: "general",
          roomIconName: null,
          threadName: "Hello there",
        },
      ],
      nextCursor: null,
    });
  });

  it("round-trip threads", () => {
    const detail = {
      thread: threadJson,
      membership: threadMembershipJson,
      parentMessage: messageJson,
      permissions: {
        canRename: true,
        canClose: true,
        canReopen: false,
        canLock: false,
        canUnlock: false,
        canDelete: false,
        canConvertWork: false,
        canManageWork: false,
        canUpdateWorkStatus: false,
        canAssignWork: false,
        canRemoveWork: false,
      },
      work: null,
      users: [userJson],
    } as const;

    roundTrips(ThreadList, { threads: [{ thread: threadJson, membership: null }], users: [] });
    roundTrips(ThreadDetail, detail);
    roundTrips(ThreadCreated, { detail, message: messageJson });
    roundTrips(CreateThread, { parentMessageId: 9001, name: null, message: createMessageJson });
  });

  it("round-trip the panes, direct messages and the switcher", () => {
    roundTrips(MemberList, {
      members: [{ userId: 7, presence: "idle", statusText: null, starred: true }],
      users: [userJson],
    });
    roundTrips(FileList, {
      files: [
        {
          messageId: 9001,
          threadId: null,
          creatorId: 7,
          attachment: attachmentJson,
          createdAt: "2026-10-06T09:15:00.123Z",
        },
      ],
      users: [],
      nextPage: 2,
    });
    roundTrips(DirectCandidateList, {
      candidates: [{ userId: 9, agent: true, starred: false }],
      users: [],
    });
    roundTrips(CreateDirect, { userIds: [8, 9] });
    roundTrips(RenameDirect, { name: null });
    roundTrips(Switcher, {
      rooms: [
        {
          roomId: 11,
          name: "Maya Chen and Jonah Park",
          kind: "group",
          iconName: null,
          unread: true,
          muted: false,
          favorite: false,
        },
      ],
      people: [{ userId: 8, directRoomId: 9 }],
      threads: [{ threadId: 88, name: "Hello there", roomId: 12, roomName: "general" }],
      users: [],
    });
  });
});

describe("S2 sync events", () => {
  it("decode in a batch", () => {
    const frame = Schema.decodeUnknownSync(ServerFrame)({
      t: "batch",
      events: [
        {
          seq: 1,
          topic: "room:12",
          type: "message.reactions",
          data: {
            messageId: 9001,
            roomId: 12,
            threadId: null,
            reactions: [reactionJson],
            boosts: [],
            updatedAt: "2026-10-06T09:16:00.000Z",
          },
        },
        {
          seq: 2,
          topic: "room:12",
          type: "message.pinned",
          data: { messageId: 9001, roomId: 12, pinned: false, pinCount: 3 },
        },
        {
          seq: 3,
          topic: "room:12",
          type: "thread.indicator",
          data: { roomId: 12, parentMessageId: 9001, thread: indicatorJson },
        },
        { seq: 4, topic: "room:12", type: "thread.created", data: threadJson },
        { seq: 5, topic: "thread:88", type: "thread.updated", data: threadJson },
        { seq: 6, topic: "thread:88", type: "thread.removed", data: { threadId: 88, roomId: 12 } },
        {
          seq: 7,
          topic: "user",
          type: "thread.unread",
          data: { threadId: 88, roomId: 12, refreshOnly: false },
        },
        { seq: 8, topic: "user", type: "thread.read", data: { threadId: 88, roomId: 12 } },
        {
          seq: 9,
          topic: "user",
          type: "saved.changed",
          data: { messageId: 9001, item: null },
        },
      ],
    });

    expect(frame.t === "batch" && frame.events.map((event) => event.type)).toEqual([
      "message.reactions",
      "message.pinned",
      "thread.indicator",
      "thread.created",
      "thread.updated",
      "thread.removed",
      "thread.unread",
      "thread.read",
      "saved.changed",
    ]);
  });
});
