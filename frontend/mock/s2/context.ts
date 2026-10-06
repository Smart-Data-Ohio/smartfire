/**
 * What the S2 modules get from the server (the world, the clock, the sync hub and the S1
 * helpers they build on) and the small router they declare their endpoints with.
 */
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { RoomDetail } from "../../src/gen/RoomDetail.ts";
import type { SidebarRow } from "../../src/gen/SidebarRow.ts";
import type { User } from "../../src/gen/User.ts";
import type { MockResponse } from "../http.ts";
import type { Json } from "../json.ts";
import type { Mentionable } from "../markdown.ts";
import type { Scheduler } from "../scheduler.ts";
import type { RoomRecord, World } from "../seed.ts";
import type { Outgoing } from "../sync.ts";
import type { MessageDraft } from "./model.ts";

/** The server's side of the S2 modules. The world is a getter: `reset()` replaces it. */
export interface S2Context {
  readonly world: () => World;
  /** The clock, in ms since the epoch. */
  readonly now: () => number;
  readonly scheduler: Scheduler;
  /** Publishes events on the sync hub, in order. */
  readonly publish: (events: readonly Outgoing[]) => void;
  /** A fresh UUID-formatted id from the seeded PRNG. */
  readonly uuid: () => string;
  /** A fresh hex string from the seeded PRNG, `length` characters long. */
  readonly hex: (length: number) => string;
  /** The room, if the viewer belongs to it; 404 otherwise. */
  readonly roomOr404: (roomId: number) => RoomRecord;
  /** The users with these ids, once each, by id. */
  readonly usersFor: (ids: Iterable<number>) => User[];
  /** Who `@name` and `@[Name]` can refer to. */
  readonly mentionables: () => Mentionable[];
  readonly sidebarRow: (record: RoomRecord) => SidebarRow;
  readonly roomDetail: (roomId: number) => RoomDetail;
  /** The room's name, or a direct room's other members' names. */
  readonly displayName: (record: RoomRecord) => string;
  /** Posts on a room's root timeline with everything that follows (unread, sidebar, bot). */
  readonly postToRoom: (record: RoomRecord, draft: MessageDraft) => MessageDTO;
}

/** A matched request, as a handler sees it. */
export interface RouteRequest {
  /** The pattern's capture groups, as numbers (every S2 path parameter is an id). */
  readonly ids: readonly number[];
  readonly query: URLSearchParams;
  readonly body: Json | undefined;
}

/** Answers a matched request. */
export type RouteHandler = (request: RouteRequest) => MockResponse | Promise<MockResponse>;

/** One endpoint: a method and a path pattern under `/api/v1`. */
export interface Route {
  readonly method: string;
  readonly pattern: RegExp;
  readonly handle: RouteHandler;
}

/** Declares an endpoint; `pattern` must match the whole path, ids in capture groups. */
export function route(method: string, pattern: RegExp, handle: RouteHandler): Route {
  return { method, pattern, handle };
}

/** The handler for a request, or `null` when no route matches. */
export function dispatch(
  routes: readonly Route[],
  method: string,
  path: string,
  query: URLSearchParams,
  body: Json | undefined,
): (() => MockResponse | Promise<MockResponse>) | null {
  for (const candidate of routes) {
    if (candidate.method !== method) continue;

    const match = candidate.pattern.exec(path);

    if (match === null) continue;

    const ids = match.slice(1).map(Number);

    return () => candidate.handle({ ids, query, body });
  }

  return null;
}

/** The first captured id. */
export function firstId(request: RouteRequest): number {
  return request.ids[0] ?? 0;
}
