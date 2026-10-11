/**
 * A room's incoming webhooks for the settings dialog's Webhooks tab (crates/api/src/webhooks.rs).
 * Management is the room's creator and administrators, never a direct room; only create and
 * regenerate return the token. Each webhook posts as its own bot account, and a test post
 * carries the webhook's name in `message.webhook`. A board refuses webhook posts.
 */
import type { ChannelWebhook } from "../../src/gen/ChannelWebhook.ts";
import type { ChannelWebhookCredential } from "../../src/gen/ChannelWebhookCredential.ts";
import { forbidden, noContent, notFound, ok, validation } from "../http.ts";
import { field, isRecord, isString, type Json } from "../json.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { plainDraft } from "../s2/model.ts";
import { rowTimestamp, timestamp, VIEWER_ID, type World } from "../seed.ts";

const ORIGIN = "http://127.0.0.1";

/** Webhook bot accounts sit above every seeded user. */
const FIRST_AUTHOR_ID = 9_000;

interface Stored {
  readonly webhook: ChannelWebhook;
  readonly authorId: number;
  token: string;
}

interface Webhooks {
  readonly byId: Map<number, Stored>;
  nextId: number;
}

const stores = new WeakMap<World, Webhooks>();

function storeFor(world: World): Webhooks {
  const existing = stores.get(world);

  if (existing !== undefined) return existing;

  const created: Webhooks = { byId: new Map(), nextId: 1 };

  stores.set(world, created);

  return created;
}

/** `ChannelWebhook.name`: trimmed, 1 to 80 characters. */
function webhookName(body: Json | undefined): string {
  const raw = isRecord(body) ? field(body, "name") : undefined;
  const name = isString(raw) ? raw.trim() : "";

  if (name === "" || [...name].length > 80) {
    throw validation("name", "must have 1 to 80 characters");
  }

  return name;
}

interface ChannelWebhookRoutes {
  readonly routes: readonly Route[];
}

/** Routes for `/rooms/:id/webhooks` and each webhook's rename, delete, regenerate and test. */
export function createChannelWebhooks(ctx: S2Context): ChannelWebhookRoutes {
  const state = () => storeFor(ctx.world());

  const managedRoom = (roomId: number) => {
    const record = ctx.roomOr404(roomId);

    if (record.room.kind === "direct") throw notFound();

    const viewer = ctx.world().users.get(VIEWER_ID);

    if (viewer?.role !== "administrator" && record.room.creatorId !== VIEWER_ID) {
      throw forbidden("Not allowed");
    }

    return record;
  };

  const managed = (roomId: number, webhookId: number) => {
    managedRoom(roomId);

    const stored = state().byId.get(webhookId);

    if (stored === undefined || stored.webhook.roomId !== roomId) throw notFound();

    return stored;
  };

  const credential = (stored: Stored): ChannelWebhookCredential => ({
    webhook: stored.webhook,
    token: stored.token,
    url: `${ORIGIN}/api/webhooks/${stored.webhook.id}/${stored.token}`,
  });

  const create = (roomId: number, body: Json | undefined) => {
    managedRoom(roomId);

    const name = webhookName(body);
    const world = ctx.world();
    const current = state();
    const id = current.nextId;
    const authorId = FIRST_AUTHOR_ID + id;
    const now = ctx.now();

    current.nextId += 1;
    world.users.set(authorId, {
      id: authorId,
      name,
      role: "bot",
      status: "active",
      bio: null,
      avatarUrl: `/users/${authorId}/avatar`,
      hasAvatar: false,
      customStatus: null,
      avatarIcon: null,
      agent: null,
      createdAt: timestamp(now),
      updatedAt: rowTimestamp(now),
      accountName: name,
      pronouns: null,
      bannerUrl: null,
      bannerStillUrl: null,
    });

    const stored: Stored = {
      webhook: {
        id,
        roomId,
        creatorId: VIEWER_ID,
        name,
        avatarUrl: null,
        lastUsedAt: null,
        createdAt: timestamp(now),
        updatedAt: timestamp(now),
      },
      authorId,
      token: ctx.hex(48),
    };

    current.byId.set(id, stored);

    return ok(credential(stored), 201);
  };

  const rename = (roomId: number, webhookId: number, body: Json | undefined) => {
    const stored = managed(roomId, webhookId);

    if (isRecord(body) && field(body, "name") === undefined) return ok(stored.webhook);

    const name = webhookName(body);
    const world = ctx.world();
    const author = world.users.get(stored.authorId);

    const next: Stored = {
      ...stored,
      webhook: { ...stored.webhook, name, updatedAt: timestamp(ctx.now()) },
    };

    state().byId.set(webhookId, next);

    if (author !== undefined) {
      world.users.set(stored.authorId, { ...author, name, updatedAt: rowTimestamp(ctx.now()) });
    }

    return ok(next.webhook);
  };

  const regenerate = (roomId: number, webhookId: number) => {
    const stored = managed(roomId, webhookId);

    stored.token = ctx.hex(48);

    return ok(credential(stored));
  };

  const remove = (roomId: number, webhookId: number) => {
    managed(roomId, webhookId);
    state().byId.delete(webhookId);

    return noContent();
  };

  const test = (roomId: number, webhookId: number) => {
    const stored = managed(roomId, webhookId);
    const record = ctx.roomOr404(roomId);

    if (record.room.kind === "board") {
      throw validation("room", "cannot accept webhook messages");
    }

    const message = ctx.postToRoom(record, {
      ...plainDraft(stored.authorId, "This is a test webhook message.", ctx.uuid()),
      webhook: { name: stored.webhook.name, username: null, avatarUrl: stored.webhook.avatarUrl },
    });

    state().byId.set(webhookId, {
      ...stored,
      webhook: { ...stored.webhook, lastUsedAt: message.createdAt },
    });

    return ok(message);
  };

  return {
    routes: [
      route("GET", /^\/rooms\/(\d+)\/webhooks$/, (request) => {
        const roomId = firstId(request);

        managedRoom(roomId);

        const webhooks: ChannelWebhook[] = [];

        for (const stored of state().byId.values()) {
          if (stored.webhook.roomId === roomId) webhooks.push(stored.webhook);
        }

        webhooks.sort((left, right) => left.id - right.id);

        return ok({ webhooks });
      }),
      route("POST", /^\/rooms\/(\d+)\/webhooks$/, (request) =>
        create(firstId(request), request.body),
      ),
      route("PATCH", /^\/rooms\/(\d+)\/webhooks\/(\d+)$/, (request) =>
        rename(request.ids[0] ?? 0, request.ids[1] ?? 0, request.body),
      ),
      route("DELETE", /^\/rooms\/(\d+)\/webhooks\/(\d+)$/, (request) =>
        remove(request.ids[0] ?? 0, request.ids[1] ?? 0),
      ),
      route("POST", /^\/rooms\/(\d+)\/webhooks\/(\d+)\/regenerate_token$/, (request) =>
        regenerate(request.ids[0] ?? 0, request.ids[1] ?? 0),
      ),
      route("POST", /^\/rooms\/(\d+)\/webhooks\/(\d+)\/test$/, (request) =>
        test(request.ids[0] ?? 0, request.ids[1] ?? 0),
      ),
    ],
  };
}
