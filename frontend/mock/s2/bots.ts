/**
 * Chat bots (S7): the classic bot pages' twin under `/admin/bots/*`, kept per world so `reset()`
 * starts it over. The world's bot is a workspace agent the viewer owns, in the rooms it belongs
 * to; a legacy bot with no agent sits beside it.
 */
import type { ApiError } from "../../src/gen/ApiError.ts";
import type { Bot } from "../../src/gen/Bot.ts";
import type { BotAgent } from "../../src/gen/BotAgent.ts";
import type { BotKey } from "../../src/gen/BotKey.ts";
import type { BotRoom } from "../../src/gen/BotRoom.ts";
import type { BotSummary } from "../../src/gen/BotSummary.ts";
import type { CredentialList } from "../../src/gen/CredentialList.ts";
import type { GrantList } from "../../src/gen/GrantList.ts";
import { HttpError, notFound, ok } from "../http.ts";
import { field, intField, type Json, stringField } from "../json.ts";
import { BOT_ID, rowTimestamp, timestamp, VIEWER_ID, type World } from "../seed.ts";
import { applied, botAvatarChange } from "./bot-avatar.ts";
import {
  type AgentRecord,
  agentRecord,
  type BotRecord,
  PERSONAL_CAPABILITIES,
} from "./bot-record.ts";
import { type Route, route, type S2Context } from "./context.ts";
import type { createMeAgents } from "./me-agents.ts";
import type { Uploads } from "./uploads.ts";

/** The capabilities a grant can give, in the classic order. */
const CAPABILITIES = [
  "read_messages",
  "post_messages",
  "react",
  "manage_threads",
  "external_action",
  "fizzy",
  "dm_anyone",
];

const ORIGIN = "http://127.0.0.1";

/** A legacy bot beside the world's: no agent, no webhook. */
const LEGACY_ID = 900;

interface State {
  bots: Map<number, BotRecord>;
  nextId: number;
}

const VALIDATION: ApiError["_tag"] = "Validation";

/** A 422 with the classic form's messages by field. */
const invalid = (fields: Readonly<Record<string, readonly string[]>>): HttpError =>
  new HttpError(422, {
    _tag: VALIDATION,
    message: `Validation failed: ${Object.values(fields).flat().join(", ")}`,
    fields: Object.fromEntries(Object.entries(fields).map(([key, value]) => [key, [...value]])),
  });

/** A refusal about one input, its message also the error's. */
const refusedField = (key: string, message: string): HttpError =>
  new HttpError(422, { _tag: VALIDATION, message, fields: Object.fromEntries([[key, [message]]]) });

/** The classic page's alert, a 422 naming no field. */
const refusal = (message: string): HttpError =>
  new HttpError(422, { _tag: VALIDATION, message, fields: {} });

function initialState(world: World, now: number): State {
  const bots = new Map<number, BotRecord>();
  const seeded = world.users.get(BOT_ID);

  if (seeded !== undefined) {
    bots.set(BOT_ID, {
      kind: "workspace",
      id: BOT_ID,
      name: seeded.name,
      avatarUrl: seeded.avatarUrl,
      avatarAttached: !seeded.avatarUrl.endsWith("/avatar"),
      iconName: null,
      botAvatar: seeded.botAvatar,
      webhookUrl: "https://agents.example/smartfire",
      signingSecret: "whsec_mock_signing_secret",
      key: `${BOT_ID}-mockkey`,
      github: null,
      agent: {
        ...agentRecord(1),
        provider: "Anthropic",
        runtime: "Claude",
        description: "Answers questions about the team's work.",
        caps: [200, null, 10],
      },
      removed: false,
    });
  }

  bots.set(LEGACY_ID, {
    kind: "workspace",
    id: LEGACY_ID,
    name: "Deploy Bot",
    avatarUrl: `/users/${LEGACY_ID}/avatar`,
    avatarAttached: false,
    iconName: null,
    botAvatar: null,
    webhookUrl: null,
    signingSecret: null,
    key: `${LEGACY_ID}-mockkey`,
    github: null,
    agent: null,
    removed: false,
  });

  world.users.set(LEGACY_ID, {
    id: LEGACY_ID,
    name: "Deploy Bot",
    role: "bot",
    status: "active",
    bio: null,
    avatarUrl: `/users/${LEGACY_ID}/avatar`,
    hasAvatar: false,
    customStatus: null,
    avatarIcon: null,
    agent: null,
    botAvatar: null,
    createdAt: timestamp(now),
    updatedAt: rowTimestamp(now),
    accountName: "Deploy Bot",
    pronouns: null,
    bannerUrl: null,
    bannerStillUrl: null,
  });

  return { bots, nextId: 901 };
}

/** The bots module. */
export interface BotsModule {
  readonly routes: readonly Route[];
}

/**
 * Creates the bots module. `requireSudo` throws `SudoRequired` while the password confirmation
 * has lapsed (the admin module's switch).
 */
export function createBots(
  ctx: S2Context,
  uploads: Uploads,
  requireSudo: () => void,
  personalAgents: Pick<ReturnType<typeof createMeAgents>, "records" | "credentialId">,
  personalEnabled: () => boolean,
): BotsModule {
  let state: State | null = null;
  let stateWorld: World | null = null;

  const current = (): State => {
    const world = ctx.world();

    if (state === null || stateWorld !== world) {
      state = initialState(world, ctx.now());
      stateWorld = world;
    }

    return state;
  };

  const viewerName = () => ctx.world().users.get(VIEWER_ID)?.name ?? "You";

  const botOr404 = (id: number): BotRecord => {
    const found =
      current().bots.get(id) ?? personalAgents.records().find((record) => record.id === id);

    if (found === undefined || found.removed) throw notFound();

    return found;
  };

  const rooms = (bot: BotRecord): BotRoom[] =>
    [...ctx.world().rooms.values()].flatMap((record) => {
      const member =
        bot.kind === "personal"
          ? bot.roomIds.includes(record.room.id) && record.memberIds.includes(VIEWER_ID)
          : record.memberIds.includes(bot.id);

      if (!member || record.room.kind === "direct") return [];

      const url = `${ORIGIN}/rooms/${record.room.id}/BOT_KEY/messages`;

      return [
        {
          id: record.room.id,
          name: record.room.name ?? "",
          messageCommand: `curl -d 'Hello!' ${url}`,
          attachmentCommand: `curl -F "attachment=@/path/to/file" ${url}`,
        },
      ];
    });

  const summary = (record: BotRecord): BotSummary => ({
    id: record.id,
    name: record.name,
    avatarUrl: record.avatarUrl,
    icon: null,
    botAvatar: record.botAvatar,
    ownership:
      record.kind === "personal"
        ? `Personal agent · Owned by ${viewerName()}`
        : record.agent === null
          ? "no owner recorded"
          : `Workspace agent · Owned by ${viewerName()}`,
    supportsBotKey: record.kind === "workspace",
    rooms: record.kind === "personal" ? [] : rooms(record),
  });

  const agentOf = (agent: AgentRecord): BotAgent => {
    const [messages, posts, actions] = agent.caps;
    const of = (cap: number | null) => (cap === null ? "" : `/${cap}`);

    return {
      id: agent.id,
      provider: agent.provider,
      runtime: agent.runtime,
      description: agent.description,
      dailyMessageCap: messages,
      dailyBoardPostCap: posts,
      dailyExternalActionCap: actions,
      usage: `0${of(messages)} messages · 0${of(posts)} board posts · 0${of(actions)} external actions`,
      suspended: agent.suspended,
      ledgerUrl: `/agents/${agent.id}/events`,
      approvalsUrl: `/agents/${agent.id}/approvals`,
    };
  };

  const bot = (record: BotRecord): Bot => ({
    id: record.id,
    name: record.name,
    avatarUrl: record.avatarUrl,
    avatarAttached: record.avatarAttached,
    iconName: record.iconName,
    icon:
      record.iconName === null ? null : { kind: "emoji", title: record.iconName, character: "🤖" },
    botAvatar: record.botAvatar,
    webhookUrl: record.webhookUrl,
    canAdminister: true,
    supportsBotKey: record.kind === "workspace",
    agent: record.agent === null ? null : agentOf(record.agent),
    signingSecret: record.signingSecret,
    github: record.github,
  });

  const keyOf = (record: Extract<BotRecord, { kind: "workspace" }>): BotKey => ({
    id: record.id,
    name: record.name,
    key: record.key,
    exampleCommand: `curl -d 'Hello!' ${ORIGIN}/rooms/ROOM_ID/${record.key}/messages`,
  });

  /** A new picture reaches the bot's user (where the world has one), and every client, at once. */
  const showPicture = (record: BotRecord) => {
    const users = ctx.world().users;
    const user = users.get(record.id);

    if (user === undefined) return;

    const next = {
      ...user,
      avatarUrl: record.avatarUrl,
      hasAvatar: record.avatarAttached,
      botAvatar: record.botAvatar,
      updatedAt: rowTimestamp(ctx.now()),
    };

    users.set(record.id, next);
    ctx.publish([{ topic: "user", type: "user.updated", data: next }]);
  };

  const changed = (record: BotRecord, notice: string | null = null) =>
    ok({ bot: bot(record), notice });

  /** `ensure_agent`: a legacy bot gets its agent when its credentials or grants open. */
  const ensureAgent = (record: BotRecord): AgentRecord => {
    if (record.agent === null) {
      record.agent = agentRecord(record.id + 1000);
    }

    return record.agent;
  };

  const blank = (value: string | null) => value === null || value.trim() === "";

  const create = (body: Json | undefined) => {
    requireSudo();

    const name = stringField(body, "name") ?? "";

    if (blank(name)) throw invalid({ name: ["can't be blank"] });

    const held = current();
    const id = held.nextId;
    const avatar = stringField(body, "avatar");
    const picked = botAvatarChange(body, (message) => refusedField("botAvatar", message));

    const record: Extract<BotRecord, { kind: "workspace" }> = {
      kind: "workspace",
      id,
      name: name.trim(),
      avatarUrl: avatar === null ? `/users/${id}/avatar` : uploads.attachment(avatar).url,
      avatarAttached: avatar !== null,
      iconName: stringField(body, "iconName"),
      botAvatar: applied(null, picked),
      webhookUrl: stringField(body, "webhookUrl"),
      signingSecret: null,
      key: `${id}-${ctx.hex(12)}`,
      github: null,
      agent: agentRecord(id + 1000),
      removed: false,
    };

    held.bots.set(id, record);
    held.nextId += 1;
    // The server makes the bot a user: it can post, and the directory draws its avatar.
    ctx.world().users.set(id, {
      id,
      name: record.name,
      role: "bot",
      status: "active",
      bio: null,
      avatarUrl: record.avatarUrl,
      hasAvatar: record.avatarAttached,
      customStatus: null,
      avatarIcon: null,
      agent: null,
      botAvatar: record.botAvatar,
      createdAt: timestamp(ctx.now()),
      updatedAt: rowTimestamp(ctx.now()),
      accountName: record.name,
      pronouns: null,
      bannerUrl: null,
      bannerStillUrl: null,
    });

    return ok(keyOf(record));
  };

  const cap = (value: string | null, field: string, errors: Record<string, string[]>) => {
    if (value === null || value.trim() === "") return null;

    const number = Number(value);

    if (!Number.isInteger(number) || number < 1) {
      errors[field] = ["must be a whole number greater than 0"];

      return null;
    }

    return number;
  };

  const update = (id: number, body: Json | undefined) => {
    const record = botOr404(id);
    const webhookUrl = stringField(body, "webhookUrl");

    if (webhookUrl !== null && webhookUrl !== (record.webhookUrl ?? "")) requireSudo();

    const name = stringField(body, "name");
    const errors: Record<string, string[]> = {};

    if (name !== null && blank(name)) errors.name = ["can't be blank"];

    const picked = botAvatarChange(body, (message) => refusedField("botAvatar", message));
    const agent = field(body, "agent");

    const caps: [number | null, number | null, number | null] = [
      field(agent, "dailyMessageCap") === undefined
        ? (record.agent?.caps[0] ?? null)
        : cap(stringField(agent, "dailyMessageCap"), "dailyMessageCap", errors),
      field(agent, "dailyBoardPostCap") === undefined
        ? (record.agent?.caps[1] ?? null)
        : cap(stringField(agent, "dailyBoardPostCap"), "dailyBoardPostCap", errors),
      field(agent, "dailyExternalActionCap") === undefined
        ? (record.agent?.caps[2] ?? null)
        : cap(stringField(agent, "dailyExternalActionCap"), "dailyExternalActionCap", errors),
    ];

    if (record.kind === "personal" && webhookUrl !== null && !blank(webhookUrl))
      errors.webhookUrl = ["is not available to personal agents"];
    const description = stringField(agent, "description");

    if (description !== null && description.length > 500)
      errors.description = ["is too long (maximum is 500 characters)"];

    if (Object.keys(errors).length > 0) throw invalid(errors);

    if (name !== null) record.name = name.trim();

    const iconName = stringField(body, "iconName");

    if (iconName !== null) record.iconName = blank(iconName) ? null : iconName.trim();

    if (record.kind === "workspace" && webhookUrl !== null)
      record.webhookUrl = blank(webhookUrl) ? null : webhookUrl.trim();

    const avatar = stringField(body, "avatar");

    if (avatar !== null) {
      record.avatarUrl = uploads.attachment(avatar).url;
      record.avatarAttached = true;
    }

    record.botAvatar = applied(record.botAvatar, picked);

    if (avatar !== null || picked.kind !== "keep") showPicture(record);

    if (record.agent !== null && agent !== undefined && agent !== null) {
      const text = (key: string, fallback: string | null) => {
        const value = stringField(agent, key);

        return value === null ? fallback : blank(value) ? null : value.trim();
      };

      record.agent.provider = text("provider", record.agent.provider);
      record.agent.runtime = text("runtime", record.agent.runtime);
      record.agent.description = text("description", record.agent.description);
      record.agent.caps = caps;
    }

    return changed(record);
  };

  const credentialList = (record: BotRecord, agent: AgentRecord): CredentialList => ({
    botId: record.id,
    botName: record.name,
    canIssue: true,
    credentials: [...agent.credentials]
      .sort((a, b) => b.id - a.id)
      .map(
        ({ kind: _kind, connection: _connection, lastUsedIp: _lastUsedIp, ...credential }) =>
          credential,
      ),
  });

  const grantList = (record: BotRecord, agent: AgentRecord): GrantList => ({
    botId: record.id,
    botName: record.name,
    canGrant: true,
    legacy: record.kind === "workspace" && !agent.everGranted,
    grants: [...agent.grants]
      .sort((a, b) => Number(a.revoked) - Number(b.revoked))
      .map((grant) => ({
        id: grant.id,
        capability: grant.capability,
        roomName:
          grant.roomId === null
            ? "Workspace-wide"
            : (ctx.world().rooms.get(grant.roomId)?.room.name ?? "Deleted room"),
        grantedBy: grant.grantedBy,
        createdAt: grant.createdAt,
        revoked: grant.revoked,
        status: grant.revoked
          ? "revoked"
          : record.kind === "personal" &&
              (agent.suspended ||
                !personalEnabled() ||
                !rooms(record).some((room) => room.id === grant.roomId))
            ? "inactive"
            : "active",
      })),
    capabilities: CAPABILITIES,
    rooms: rooms(record).map((room) => ({ id: room.id, name: room.name })),
  });

  const issue = (id: number, body: Json | undefined) => {
    const record = botOr404(id);

    requireSudo();

    const agent = ensureAgent(record);
    const name = stringField(body, "name") ?? "";

    if (blank(name)) throw invalid({ name: ["can't be blank"] });

    const secret = `cfa_${ctx.hex(32)}`;
    const expiresAt = stringField(body, "expiresAt");

    const credentialId =
      record.kind === "personal" ? personalAgents.credentialId() : agent.credentials.length + 1;

    agent.credentials.unshift({
      id: credentialId,
      kind: "token",
      connection: null,
      lastUsedIp: null,
      name: name.trim(),
      lastFour: secret.slice(-4),
      createdBy: viewerName(),
      createdAt: timestamp(ctx.now()),
      lastUsedAt: null,
      expiresAt: blank(expiresAt) ? null : `${expiresAt}:00-04:00`,
      state: "active",
    });

    return ok({ secret, credentials: credentialList(record, agent) });
  };

  const revokeCredential = (id: number, credentialId: number) => {
    const record = botOr404(id);

    requireSudo();

    const agent = ensureAgent(record);
    const at = agent.credentials.findIndex((each) => each.id === credentialId);
    const found = agent.credentials[at];

    if (found === undefined) throw notFound();

    agent.credentials[at] = { ...found, state: "revoked" };

    return ok(credentialList(record, agent));
  };

  const grant = (id: number, body: Json | undefined) => {
    const record = botOr404(id);

    requireSudo();

    const agent = ensureAgent(record);
    const capability = stringField(body, "capability") ?? "";

    if (!CAPABILITIES.includes(capability))
      throw invalid({ capability: ["is not included in the list"] });

    const roomId = intField(body, "roomId");

    if (record.kind === "personal") {
      if (!PERSONAL_CAPABILITIES.includes(capability))
        throw invalid({ capability: ["is not available to personal agents"] });

      if (roomId === null)
        throw invalid({ room: ["must be a non-direct room for a personal agent"] });

      if (!rooms(record).some((room) => room.id === roomId))
        throw invalid({ roomId: ["isn't one of the bot's rooms"] });
    }

    const held = agent.grants.some(
      (each) => !each.revoked && each.capability === capability && each.roomId === roomId,
    );

    if (!held) {
      agent.grants.push({
        id: agent.grants.length + 1,
        capability,
        roomId,
        grantedBy: viewerName(),
        createdAt: timestamp(ctx.now()),
        revoked: false,
      });
      agent.everGranted = true;
    }

    return ok(grantList(record, agent));
  };

  const revokeGrant = (id: number, grantId: number) => {
    const record = botOr404(id);

    requireSudo();

    const agent = ensureAgent(record);
    const at = agent.grants.findIndex((each) => each.id === grantId);
    const found = agent.grants[at];

    if (found === undefined) throw notFound();

    found.revoked = true;

    return ok(grantList(record, agent));
  };

  const ids = (request: { readonly ids: readonly number[] }) => {
    const [first = 0, second = 0] = request.ids;

    return [first, second] as const;
  };

  return {
    routes: [
      route("GET", /^\/admin\/bots$/, () =>
        ok({
          bots: [
            ...[...current().bots.values()]
              .filter((record) => !record.removed)
              .sort((a, b) => a.name.localeCompare(b.name))
              .map(summary),
            ...personalAgents.records().map(summary),
          ].sort((a, b) => a.name.localeCompare(b.name)),
        }),
      ),
      route("POST", /^\/admin\/bots$/, ({ body }) => create(body)),
      route("GET", /^\/admin\/bots\/(\d+)$/, (request) => {
        const id = ids(request)[0];

        return ok(bot(botOr404(id)));
      }),
      route("PATCH", /^\/admin\/bots\/(\d+)$/, (request) => update(ids(request)[0], request.body)),
      route("DELETE", /^\/admin\/bots\/(\d+)$/, (request) => {
        const record = botOr404(ids(request)[0]);

        record.removed = true;

        if (record.agent !== null) record.agent.suspended = true;

        return ok({ id: record.id });
      }),
      route("POST", /^\/admin\/bots\/(\d+)\/kill_switch$/, (request) => {
        const record = botOr404(ids(request)[0]);

        if (record.agent === null) throw notFound();

        record.agent.suspended = true;

        return changed(record, "Agent suspended; 0 approvals cancelled.");
      }),
      route("PUT", /^\/admin\/bots\/(\d+)\/key$/, (request) => {
        requireSudo();

        const record = botOr404(ids(request)[0]);

        if (record.kind === "personal")
          throw refusal("Bot key is not available to personal agents");

        record.key = `${record.id}-${ctx.hex(12)}`;

        return ok(keyOf(record));
      }),
      route("POST", /^\/admin\/bots\/(\d+)\/webhook_secret$/, (request) => {
        const record = botOr404(ids(request)[0]);

        requireSudo();

        if (record.agent === null && record.webhookUrl === null) {
          throw refusal("Set a webhook URL before generating a signing secret.");
        }

        record.signingSecret = `whsec_${ctx.hex(24)}`;

        return changed(
          record,
          "Signing secret reset. Update the receiving service with the new secret.",
        );
      }),
      route("PUT", /^\/admin\/bots\/(\d+)\/github_connection$/, (request) => {
        const record = botOr404(ids(request)[0]);

        requireSudo();

        const token = stringField(request.body, "accessToken") ?? "";

        if (blank(token)) {
          const message = "Paste a token to connect GitHub.";

          throw refusedField("accessToken", message);
        }

        record.github = {
          login: `${record.name.split(" ")[0]?.toLowerCase() ?? "bot"}-bot`,
          usable: true,
          disconnectedReason: null,
        };

        return changed(record, `GitHub connected as ${record.github.login}.`);
      }),
      route("DELETE", /^\/admin\/bots\/(\d+)\/github_connection$/, (request) => {
        const record = botOr404(ids(request)[0]);

        requireSudo();
        record.github = null;

        return changed(record, "GitHub disconnected.");
      }),
      route("GET", /^\/admin\/bots\/(\d+)\/credentials$/, (request) => {
        const record = botOr404(ids(request)[0]);

        return ok(credentialList(record, ensureAgent(record)));
      }),
      route("POST", /^\/admin\/bots\/(\d+)\/credentials$/, (request) =>
        issue(ids(request)[0], request.body),
      ),
      route("DELETE", /^\/admin\/bots\/(\d+)\/credentials\/(\d+)$/, (request) =>
        revokeCredential(...ids(request)),
      ),
      route("GET", /^\/admin\/bots\/(\d+)\/grants$/, (request) => {
        const record = botOr404(ids(request)[0]);

        return ok(grantList(record, ensureAgent(record)));
      }),
      route("POST", /^\/admin\/bots\/(\d+)\/grants$/, (request) =>
        grant(ids(request)[0], request.body),
      ),
      route("DELETE", /^\/admin\/bots\/(\d+)\/grants\/(\d+)$/, (request) =>
        revokeGrant(...ids(request)),
      ),
    ],
  };
}
