/**
 * GitHub repository subscriptions and inbound email for the room settings dialog.
 * Authorization matches the API: the creator and administrators, never a direct room, and
 * inbound email never a board. The mock does not call GitHub; a valid subscribe succeeds.
 */
import type { GithubEventChoice } from "../../src/gen/GithubEventChoice.ts";
import type { GithubSubscription } from "../../src/gen/GithubSubscription.ts";
import type { GithubSubscriptionList } from "../../src/gen/GithubSubscriptionList.ts";
import type { InboundEmail } from "../../src/gen/InboundEmail.ts";
import type { User } from "../../src/gen/User.ts";
import { forbidden, notFound, ok, refusal, validation } from "../http.ts";
import { field, isBoolean, isRecord, isString, type Json } from "../json.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { ROOM_IDS, rowTimestamp, timestamp, VIEWER_ID, type World } from "../seed.ts";

const EVENT_KEYS = [
  "opened",
  "merged",
  "closed",
  "review_requested",
  "review_submitted",
  "checks_failed",
] as const;

const DEFAULT_EVENTS = ["opened", "merged", "review_requested", "checks_failed"];

const DOMAIN = "mail.campfire.test";

/** The GitHub bot the classic subscribe inserts into the room, and the last unsubscribe removes. */
export const GITHUB_BOT_ID = 42;

/** 32 hex characters, stable so the e2e can read the seeded address. */
const LAUNCH_TOKEN = "a1b2c3d4e5f67890a1b2c3d4e5f67890";

const SUBSET =
  "must be a subset of opened, merged, closed, review_requested, review_submitted, and checks_failed";

interface StoredSubscription {
  readonly id: number;
  readonly fullName: string;
  readonly events: readonly string[];
}

interface Integrations {
  readonly subscriptions: Map<number, StoredSubscription[]>;
  nextSubscriptionId: number;
  readonly inboundTokens: Map<number, string>;
}

const integrations = new WeakMap<World, Integrations>();

function eventLabel(key: string): string {
  const spaced = key.replaceAll("_", " ");

  return `${spaced.charAt(0).toUpperCase()}${spaced.slice(1)}`;
}

function catalog(): GithubEventChoice[] {
  const events: GithubEventChoice[] = [];

  for (const key of EVENT_KEYS) {
    events.push({
      key,
      label: eventLabel(key),
      selectedByDefault: DEFAULT_EVENTS.includes(key),
    });
  }

  return events;
}

function knownEvent(value: string): boolean {
  for (const key of EVENT_KEYS) {
    if (key === value) return true;
  }

  return false;
}

function integrationsFor(world: World): Integrations {
  const existing = integrations.get(world);

  if (existing !== undefined) return existing;

  const created: Integrations = {
    subscriptions: new Map(),
    nextSubscriptionId: 1,
    inboundTokens: new Map([[ROOM_IDS.launchPlanning, LAUNCH_TOKEN]]),
  };

  integrations.set(world, created);

  return created;
}

function validName(name: string): boolean {
  if (name === "" || name === "." || name === "..") return false;

  for (const char of name) {
    const ok =
      (char >= "a" && char <= "z") ||
      (char >= "0" && char <= "9") ||
      char === "_" ||
      char === "." ||
      char === "-";

    if (!ok) return false;
  }

  return true;
}

function splitName(fullName: string) {
  const trimmed = fullName.trim();
  const slash = trimmed.indexOf("/");
  const owner = (slash === -1 ? trimmed : trimmed.slice(0, slash)).trim().toLowerCase();
  const repo = (slash === -1 ? "" : trimmed.slice(slash + 1)).trim().toLowerCase();

  return { owner, repo };
}

function readEvents(body: Json, creating: boolean): string[] {
  const raw = field(body, "events");

  if (raw === undefined || (Array.isArray(raw) && raw.length === 0)) {
    if (creating) return [...DEFAULT_EVENTS];

    throw refusal("events", "Could not update: Events must include at least one event.");
  }

  if (!Array.isArray(raw)) {
    throw refusal("events", `Could not subscribe: Events ${SUBSET}.`);
  }

  const events: string[] = [];

  for (const value of raw) {
    if (!isString(value) || !knownEvent(value)) {
      const prefix = creating ? "Could not subscribe" : "Could not update";

      throw refusal("events", `${prefix}: Events ${SUBSET}.`);
    }

    if (!events.includes(value)) events.push(value);
  }

  return events;
}

interface RoomIntegrations {
  readonly routes: readonly Route[];
}

/** Routes for `/rooms/:id/github_subscriptions` and `/rooms/:id/inbound_email`. */
export function createRoomIntegrations(ctx: S2Context): RoomIntegrations {
  const viewer = () => ctx.world().users.get(VIEWER_ID);
  const isAdmin = () => viewer()?.role === "administrator";

  const canManage = (creatorId: number) => isAdmin() || creatorId === VIEWER_ID;

  const state = () => integrationsFor(ctx.world());

  const githubBot = () => {
    const world = ctx.world();
    const existing = world.users.get(GITHUB_BOT_ID);

    if (existing !== undefined) return existing;

    const now = ctx.now();

    const bot: User = {
      id: GITHUB_BOT_ID,
      name: "GitHub",
      role: "bot",
      status: "active",
      bio: null,
      avatarUrl: `/users/${GITHUB_BOT_ID}/avatar`,
      hasAvatar: false,
      customStatus: null,
      avatarIcon: null,
      agent: null,
      createdAt: timestamp(now),
      updatedAt: rowTimestamp(now),
    };

    world.users.set(GITHUB_BOT_ID, bot);

    return bot;
  };

  const addBot = (roomId: number) => {
    const record = ctx.roomOr404(roomId);
    const botId = githubBot().id;

    if (!record.memberIds.includes(botId)) record.memberIds = [...record.memberIds, botId];
  };

  const dropBot = (roomId: number) => {
    const record = ctx.roomOr404(roomId);

    record.memberIds = record.memberIds.filter((id) => id !== GITHUB_BOT_ID);
  };

  const githubRoom = (roomId: number) => {
    const record = ctx.roomOr404(roomId);

    if (record.room.kind === "direct") throw notFound();

    if (!canManage(record.room.creatorId)) throw forbidden("Not allowed");

    return record;
  };

  const emailRoom = (roomId: number) => {
    const record = ctx.roomOr404(roomId);

    if (record.room.kind === "direct" || record.room.kind === "board") throw notFound();

    if (!canManage(record.room.creatorId)) throw forbidden("Not allowed");

    return record;
  };

  const listed = (roomId: number): GithubSubscription[] => {
    const rows = [...(state().subscriptions.get(roomId) ?? [])];

    rows.sort((left, right) => left.fullName.localeCompare(right.fullName) || left.id - right.id);

    const subscriptions: GithubSubscription[] = [];

    for (const row of rows) {
      subscriptions.push({ id: row.id, fullName: row.fullName, events: [...row.events] });
    }

    return subscriptions;
  };

  const list = (roomId: number): GithubSubscriptionList => ({
    subscriptions: listed(roomId),
    administrator: isAdmin(),
    connectPath: "/github/app/connect",
    events: catalog(),
  });

  const subscribe = (roomId: number, body: Json | undefined) => {
    githubRoom(roomId);

    if (!isRecord(body)) throw validation("base", "Invalid subscription body");

    for (const key of Object.keys(body)) {
      if (key !== "fullName" && key !== "events" && key !== "skipAccessCheck") {
        throw validation("base", "Unknown subscription field");
      }
    }

    const rawName = field(body, "fullName");
    const rawSkip = field(body, "skipAccessCheck");

    if (!isString(rawName)) throw validation("fullName", "must be a string");

    if (rawSkip !== undefined && !isBoolean(rawSkip)) {
      throw validation("skipAccessCheck", "must be a boolean");
    }

    const { owner, repo } = splitName(rawName);

    if (!validName(owner) || !validName(repo)) {
      throw refusal(
        "fullName",
        "Could not subscribe: Owner can't be blank and Repo can't be blank.",
      );
    }

    const events = readEvents(body, true);
    const fullName = `${owner}/${repo}`;
    const current = state();
    const rows = current.subscriptions.get(roomId) ?? [];

    for (const row of rows) {
      if (row.fullName === fullName) {
        throw refusal("fullName", "Could not subscribe: Owner has already been taken.");
      }
    }

    const subscription: StoredSubscription = {
      id: current.nextSubscriptionId,
      fullName,
      events,
    };

    current.nextSubscriptionId += 1;
    current.subscriptions.set(roomId, [...rows, subscription]);
    addBot(roomId);

    return ok({ id: subscription.id, fullName, events: [...events] }, 201);
  };

  const find = (roomId: number, subscriptionId: number): StoredSubscription => {
    for (const row of state().subscriptions.get(roomId) ?? []) {
      if (row.id === subscriptionId) return row;
    }

    throw notFound();
  };

  const update = (roomId: number, subscriptionId: number, body: Json | undefined) => {
    githubRoom(roomId);

    if (!isRecord(body)) throw validation("base", "Invalid subscription body");

    for (const key of Object.keys(body)) {
      if (key !== "events") throw validation("base", "Unknown subscription field");
    }

    const events = readEvents(body, false);
    const current = state();
    const rows = current.subscriptions.get(roomId) ?? [];
    const next: StoredSubscription[] = [];
    let updated: StoredSubscription | null = null;

    for (const row of rows) {
      if (row.id === subscriptionId) {
        updated = { id: row.id, fullName: row.fullName, events };
        next.push(updated);
      } else {
        next.push(row);
      }
    }

    if (updated === null) throw notFound();

    current.subscriptions.set(roomId, next);

    return ok({ id: updated.id, fullName: updated.fullName, events: [...updated.events] });
  };

  const remove = (roomId: number, subscriptionId: number) => {
    githubRoom(roomId);

    const removed = find(roomId, subscriptionId);
    const current = state();
    const next: StoredSubscription[] = [];

    for (const row of current.subscriptions.get(roomId) ?? []) {
      if (row.id !== subscriptionId) next.push(row);
    }

    current.subscriptions.set(roomId, next);

    if (next.length === 0) dropBot(roomId);

    return ok({ id: removed.id, fullName: removed.fullName, events: [...removed.events] });
  };

  const emailView = (roomId: number): InboundEmail => {
    const token = state().inboundTokens.get(roomId);

    return {
      enabled: true,
      address: token === undefined || token === "" ? null : `room-${token}@${DOMAIN}`,
    };
  };

  const rotate = (roomId: number) => {
    emailRoom(roomId);

    const current = state();
    let token = ctx.hex(32);

    for (let attempt = 0; attempt < 4; attempt += 1) {
      let used = false;

      for (const existing of current.inboundTokens.values()) {
        if (existing === token) used = true;
      }

      if (!used) break;

      token = ctx.hex(32);
    }

    current.inboundTokens.set(roomId, token);

    return ok(emailView(roomId));
  };

  return {
    routes: [
      route("GET", /^\/rooms\/(\d+)\/github_subscriptions$/, (request) => {
        const roomId = firstId(request);

        githubRoom(roomId);

        return ok(list(roomId));
      }),
      route("POST", /^\/rooms\/(\d+)\/github_subscriptions$/, (request) =>
        subscribe(firstId(request), request.body),
      ),
      route("PATCH", /^\/rooms\/(\d+)\/github_subscriptions\/(\d+)$/, (request) =>
        update(request.ids[0] ?? 0, request.ids[1] ?? 0, request.body),
      ),
      route("DELETE", /^\/rooms\/(\d+)\/github_subscriptions\/(\d+)$/, (request) =>
        remove(request.ids[0] ?? 0, request.ids[1] ?? 0),
      ),
      route("GET", /^\/rooms\/(\d+)\/inbound_email$/, (request) => {
        const roomId = firstId(request);

        emailRoom(roomId);

        return ok(emailView(roomId));
      }),
      route("POST", /^\/rooms\/(\d+)\/inbound_email$/, (request) => rotate(firstId(request))),
    ],
  };
}
