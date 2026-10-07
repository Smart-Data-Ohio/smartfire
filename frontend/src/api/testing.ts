/**
 * Test doubles for the API layer: wire fixtures typed as the generated DTOs, and `FakeApi`, an
 * `ApiClient` that answers from routes the test registers. Only tests import this module.
 */
import { Context, Effect, Layer, Ref, type Schema } from "effect";
import type { Me } from "../gen/Me.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { MessagePage } from "../gen/MessagePage.ts";
import type { RoomDetail } from "../gen/RoomDetail.ts";
import type { Sidebar } from "../gen/Sidebar.ts";
import type { SidebarRow } from "../gen/SidebarRow.ts";
import type { User } from "../gen/User.ts";
import { ApiClient, type ApiRequest } from "./client.ts";
import { type ApiFailure, NotFound, ServerError } from "./errors.ts";

export function userFixture(id: number, name = `User ${id}`): User {
  return {
    id,
    name,
    role: "member",
    status: "active",
    bio: null,
    avatarUrl: `/users/${id}/avatar`,
    hasAvatar: false,
    customStatus: null,
    avatarIcon: null,
    agent: null,
    createdAt: "2026-01-01T00:00:00.000Z",
    updatedAt: "2026-01-01T00:00:00.000000Z",
  };
}

/** A root message in `roomId`, created `id` seconds after midnight so ids and times agree. */
export function messageFixture(
  id: number,
  roomId: number,
  change?: Partial<MessageDTO>,
): MessageDTO {
  const at = new Date(Date.UTC(2026, 9, 6, 0, 0, id)).toISOString();

  return {
    id,
    roomId,
    threadId: null,
    creatorId: 7,
    clientMessageId: `client-${id}`,
    bodyHtml: `<p>Message ${id}</p>`,
    markdownSource: `Message ${id}`,
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
    cardsAsOf: at,
    steps: [],
    createdAt: at,
    updatedAt: at,
    ...change,
  };
}

export function pageFixture(
  messages: readonly MessageDTO[],
  before: number | null = null,
  after: number | null = null,
): MessagePage {
  return { messages: [...messages], users: [userFixture(7)], before, after, saved: [] };
}

export const meFixture: Me = {
  user: userFixture(7, "Ada Lovelace"),
  emailAddress: "ada@example.com",
  preferences: {
    theme: "system",
    textSize: "default",
    timeZone: "America/New_York",
    timeZoneExplicit: false,
    tourCompleted: true,
    voiceMode: "push_to_talk",
    pushToTalkKey: "`",
  },
  presenceSetting: "auto",
  doNotDisturb: { enabled: false, until: null },
  quietHours: null,
  outOfOffice: null,
  lastRoomId: 12,
};

export function sidebarRowFixture(
  roomId: number,
  name: string,
  kind: SidebarRow["room"]["kind"] = "open",
  directMemberIds: readonly number[] = [],
): SidebarRow {
  return {
    room: {
      id: roomId,
      kind,
      name: kind === "direct" ? null : name,
      iconName: null,
      creatorId: 7,
      createdAt: "2026-01-01T00:00:00.000Z",
      updatedAt: "2026-01-01T00:00:00.000Z",
    },
    membership: {
      id: roomId * 10,
      roomId,
      userId: 7,
      involvement: "everything",
      unreadAt: null,
      lastReadMessageId: null,
      roomCategoryId: null,
      favoritePosition: null,
      stageRole: null,
    },
    displayName: name,
    directMemberIds: [...directMemberIds],
    unreadCount: 0,
    mentionCount: 0,
    notificationCount: 0,
  };
}

export function sidebarFixture(
  rows: readonly SidebarRow[],
  placeholders: readonly number[] = [],
): Sidebar {
  return {
    rows: [...rows],
    categories: [],
    users: [userFixture(7, "Ada Lovelace")],
    directPlaceholderUserIds: [...placeholders],
    canCreateRooms: true,
  };
}

export function roomDetailFixture(roomId: number, unread: RoomDetail["unread"] = null): RoomDetail {
  const row = sidebarRowFixture(roomId, `room-${roomId}`);

  return {
    room: row.room,
    membership: row.membership,
    displayName: row.displayName,
    memberCount: 3,
    pinsCount: 0,
    directMemberIds: [],
    memberPreviewIds: [7],
    users: [userFixture(7, "Ada Lovelace")],
    unread,
  };
}

/** Answers one request, or fails like the real client would. */
export type FakeHandler = (request: ApiRequest) => Effect.Effect<Schema.Json, ApiFailure>;

/** `ApiClient` backed by test routes, keyed `"GET /rooms/12"` (path only, query ignored). */
export class FakeApi extends Context.Service<
  FakeApi,
  {
    /** Answers `key` with `handler` from now on (replacing any earlier route). */
    readonly route: (key: string, handler: FakeHandler) => Effect.Effect<void>;
    /** Shorthand: answers `key` with `json`. */
    readonly reply: (key: string, json: Schema.Json) => Effect.Effect<void>;
    /** Every request so far, oldest first. */
    readonly requests: Effect.Effect<readonly ApiRequest[]>;
    /** Records `request` and runs its route; an unknown route is a 404. */
    readonly dispatch: (request: ApiRequest) => Effect.Effect<Schema.Json, ApiFailure>;
  }
>()("smartfire/api/FakeApi") {
  static readonly layer = Layer.effect(
    FakeApi,
    Effect.gen(function* () {
      const routes = yield* Ref.make<ReadonlyMap<string, FakeHandler>>(new Map());
      const requests = yield* Ref.make<readonly ApiRequest[]>([]);

      const route = (key: string, handler: FakeHandler) =>
        Ref.update(routes, (current) => new Map(current).set(key, handler));

      const dispatch = Effect.fnUntraced(function* (request: ApiRequest) {
        yield* Ref.update(requests, (list) => [...list, request]);

        const handler = (yield* Ref.get(routes)).get(`${request.method} ${request.path}`);

        if (handler === undefined) {
          return yield* new NotFound({ message: `No route for ${request.method} ${request.path}` });
        }

        return yield* handler(request);
      });

      return FakeApi.of({
        route,
        reply: (key, json) => route(key, () => Effect.succeed(json)),
        requests: Ref.get(requests),
        dispatch,
      });
    }),
  );

  /** `FakeApi` plus the `ApiClient` it drives (success bodies still go through their decoder). */
  static readonly layerClient = Layer.effect(
    ApiClient,
    Effect.map(FakeApi, (fake) =>
      ApiClient.of({
        execute: (request, decode) =>
          fake.dispatch(request).pipe(
            Effect.flatMap(decode),
            Effect.catchTag("SchemaError", (error) =>
              Effect.fail(new ServerError({ status: 200, message: error.message })),
            ),
          ),
        setCsrfToken: () => Effect.void,
        csrfToken: Effect.succeed("test-csrf-token"),
      }),
    ),
  ).pipe(Layer.provideMerge(FakeApi.layer));
}
