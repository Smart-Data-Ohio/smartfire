/**
 * Chat bots (S7): the classic bot pages' twin under `/admin/bots/*`, kept per world so `reset()`
 * starts it over. The world's bot is a workspace agent the viewer owns, in the rooms it belongs
 * to; a legacy bot with no agent sits beside it. Bots made here live only in this module.
 */
import type { ApiError } from "../../src/gen/ApiError.ts";
import type { Bot } from "../../src/gen/Bot.ts";
import type { BotAgent } from "../../src/gen/BotAgent.ts";
import type { BotGithub } from "../../src/gen/BotGithub.ts";
import type { BotKey } from "../../src/gen/BotKey.ts";
import type { BotRoom } from "../../src/gen/BotRoom.ts";
import type { BotSummary } from "../../src/gen/BotSummary.ts";
import type { Credential } from "../../src/gen/Credential.ts";
import type { CredentialList } from "../../src/gen/CredentialList.ts";
import type { Grant } from "../../src/gen/Grant.ts";
import type { GrantList } from "../../src/gen/GrantList.ts";
import { HttpError, notFound, ok } from "../http.ts";
import { field, intField, type Json, stringField } from "../json.ts";
import { BOT_ID, rowTimestamp, timestamp, VIEWER_ID, type World } from "../seed.ts";
import { type Route, route, type S2Context } from "./context.ts";
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

interface AgentRecord {
  id: number;
  provider: string | null;
  runtime: string | null;
  description: string | null;
  caps: [number | null, number | null, number | null];
  suspended: boolean;
  credentials: Credential[];
  grants: Grant[];
  everGranted: boolean;
}

interface BotRecord {
  id: number;
  name: string;
  avatarUrl: string;
  iconName: string | null;
  webhookUrl: string | null;
  signingSecret: string | null;
  key: string;
  github: BotGithub | null;
  agent: AgentRecord | null;
  removed: boolean;
}

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

function agentRecord(id: number): AgentRecord {
  return {
    id,
    provider: null,
    runtime: null,
    description: null,
    caps: [null, null, null],
    suspended: false,
    credentials: [],
    grants: [],
    everGranted: false,
  };
}

function initialState(world: World, now: number): State {
  const bots = new Map<number, BotRecord>();
  const seeded = world.users.get(BOT_ID);

  if (seeded !== undefined) {
    bots.set(BOT_ID, {
      id: BOT_ID,
      name: seeded.name,
      avatarUrl: seeded.avatarUrl,
      iconName: null,
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
    id: LEGACY_ID,
    name: "Deploy Bot",
    avatarUrl: `/users/${LEGACY_ID}/avatar`,
    iconName: null,
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
    createdAt: timestamp(now),
    updatedAt: rowTimestamp(now),
    accountName: "Deploy Bot",
    pronouns: null,
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
export function createBots(ctx: S2Context, uploads: Uploads, requireSudo: () => void): BotsModule {
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
    const found = current().bots.get(id);

    if (found === undefined || found.removed) throw notFound();

    return found;
  };

  const rooms = (id: number): BotRoom[] =>
    [...ctx.world().rooms.values()].flatMap((record) => {
      if (!record.memberIds.includes(id) || record.room.kind === "direct") return [];

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
    ownership:
      record.agent === null ? "no owner recorded" : `Workspace agent · Owned by ${viewerName()}`,
    rooms: rooms(record.id),
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
    avatarAttached: !record.avatarUrl.endsWith("/avatar"),
    iconName: record.iconName,
    icon:
      record.iconName === null ? null : { kind: "emoji", title: record.iconName, character: "🤖" },
    webhookUrl: record.webhookUrl,
    canAdminister: true,
    agent: record.agent === null ? null : agentOf(record.agent),
    signingSecret: record.signingSecret,
    github: record.github,
  });

  const keyOf = (record: BotRecord): BotKey => ({
    id: record.id,
    name: record.name,
    key: record.key,
    exampleCommand: `curl -d 'Hello!' ${ORIGIN}/rooms/ROOM_ID/${record.key}/messages`,
  });

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

    const record: BotRecord = {
      id,
      name: name.trim(),
      avatarUrl: avatar === null ? `/users/${id}/avatar` : uploads.attachment(avatar).url,
      iconName: stringField(body, "iconName"),
      webhookUrl: stringField(body, "webhookUrl"),
      signingSecret: null,
      key: `${id}-${ctx.hex(12)}`,
      github: null,
      agent: agentRecord(id + 1000),
      removed: false,
    };

    held.bots.set(id, record);
    held.nextId += 1;

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

    const agent = field(body, "agent");

    const caps: [number | null, number | null, number | null] = [
      cap(stringField(agent, "dailyMessageCap"), "dailyMessageCap", errors),
      cap(stringField(agent, "dailyBoardPostCap"), "dailyBoardPostCap", errors),
      cap(stringField(agent, "dailyExternalActionCap"), "dailyExternalActionCap", errors),
    ];

    if (Object.keys(errors).length > 0) throw invalid(errors);

    if (name !== null) record.name = name.trim();

    const iconName = stringField(body, "iconName");

    if (iconName !== null) record.iconName = blank(iconName) ? null : iconName.trim();

    if (webhookUrl !== null) record.webhookUrl = blank(webhookUrl) ? null : webhookUrl.trim();

    const avatar = stringField(body, "avatar");

    if (avatar !== null) record.avatarUrl = uploads.attachment(avatar).url;

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
    credentials: [...agent.credentials].reverse(),
  });

  const grantList = (record: BotRecord, agent: AgentRecord): GrantList => ({
    botId: record.id,
    botName: record.name,
    canGrant: true,
    legacy: !agent.everGranted,
    grants: [...agent.grants].sort((a, b) => Number(a.revoked) - Number(b.revoked)),
    capabilities: CAPABILITIES,
    rooms: rooms(record.id).map((room) => ({ id: room.id, name: room.name })),
  });

  const issue = (id: number, body: Json | undefined) => {
    const record = botOr404(id);

    requireSudo();

    const agent = ensureAgent(record);
    const name = stringField(body, "name") ?? "";

    if (blank(name)) throw invalid({ name: ["can't be blank"] });

    const secret = `cfa_${ctx.hex(32)}`;
    const expiresAt = stringField(body, "expiresAt");
    const credentialId = agent.credentials.length + 1;

    agent.credentials.push({
      id: credentialId,
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

    const roomName =
      roomId === null
        ? "Workspace-wide"
        : (ctx.world().rooms.get(roomId)?.room.name ?? "Deleted room");

    const held = agent.grants.some(
      (each) => !each.revoked && each.capability === capability && each.roomName === roomName,
    );

    if (!held) {
      agent.grants.push({
        id: agent.grants.length + 1,
        capability,
        roomName,
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

    agent.grants[at] = { ...found, revoked: true };

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
          bots: [...current().bots.values()]
            .filter((record) => !record.removed)
            .sort((a, b) => a.name.localeCompare(b.name))
            .map(summary),
        }),
      ),
      route("POST", /^\/admin\/bots$/, ({ body }) => create(body)),
      route("GET", /^\/admin\/bots\/(\d+)$/, (request) => ok(bot(botOr404(ids(request)[0])))),
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
