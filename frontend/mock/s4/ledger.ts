/**
 * Agents' event ledgers (S4): `GET /agents/:agentId/events?outcome=&before=`, newest first by
 * `(created_at, id)`, 50 a page, for administrators and the agent's owner only (a 403 for anyone
 * else, a 404 for an unknown agent). The ledger has no live updates.
 *
 * The seed gives Ember a page and more of every kind of entry (plus one type this build doesn't
 * know, which the client must tolerate), some about real seeded messages, a few in a room the
 * viewer isn't in, and two pairs sharing a timestamp (one across the page boundary). Scout and
 * Atlas get a few each; the others none.
 *
 * Entries are presented per viewer: `roomName`, `detail`, `external.message`, `handoffSummary`
 * and `content` are `null` unless the viewer is an administrator or a member of the entry's room
 * (an entry with no room is never gated).
 */
import type { AgentDeliveryOutcome } from "../../src/gen/AgentDeliveryOutcome.ts";
import type { AgentExternalResult } from "../../src/gen/AgentExternalResult.ts";
import type { AgentLedgerEvent } from "../../src/gen/AgentLedgerEvent.ts";
import type { AgentLedgerEventType } from "../../src/gen/AgentLedgerEventType.ts";
import type { AgentWebhookStatus } from "../../src/gen/AgentWebhookStatus.ts";
import { forbidden, notFound, ok } from "../http.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { iso } from "../s2/model.ts";
import { beforeOf, keysetPage, plainText } from "../s3/model.ts";
import { ROOM_IDS, USER_IDS } from "../seed.ts";
import { AGENT_IDS, type Agents, HIDDEN_ROOM, viewerManages, viewerRoomName } from "./agents.ts";

const MINUTE = 60_000;

const HOUR = 60 * MINUTE;

/** At most this many a page, as the server pages. */
export const LEDGER_PAGE_SIZE = 50;

/** How many entries Ember's seeded ledger holds (more than a page). */
export const EMBER_LEDGER_SIZE = 84;

/** A type this build doesn't know, as a newer server might send. */
export const UNKNOWN_LEDGER_TYPE = "budget_threshold_crossed";

/** The cut `content` and `handoffSummary` get: 140 characters, the last three `...`. */
function cut(text: string): string {
  const chars = [...text];

  return chars.length > 140 ? `${chars.slice(0, 137).join("")}...` : text;
}

/** An entry as the server keeps it (before the viewer's gate). */
interface LedgerRecord {
  readonly id: number;
  readonly agentId: number;
  /** A known type, or (for tolerance) one this build doesn't know. */
  readonly eventType: AgentLedgerEventType | typeof UNKNOWN_LEDGER_TYPE;
  readonly outcome: AgentDeliveryOutcome | null;
  readonly createdAt: number;
  readonly roomId: number | null;
  readonly actorId: number | null;
  readonly messageId: number | null;
  readonly hop: number;
  readonly detail: string | null;
  readonly webhookStatus: AgentWebhookStatus;
  readonly webhookAttempts: number;
  readonly webhookLastError: string | null;
  readonly external: AgentExternalResult | null;
  readonly handoffSummary: string | null;
  /** Whether the agent could read the message (a member with `read_messages` there). */
  readonly readable: boolean;
}

/** An entry as the wire carries it; `eventType` may be one this build doesn't know. */
type LedgerWire = Omit<AgentLedgerEvent, "eventType"> & { readonly eventType: string };

/** One shape of entry; `index` spreads them over rooms, people and messages. */
type Template = (index: number) => Omit<LedgerRecord, "id" | "agentId" | "createdAt">;

const BLANK = {
  roomId: null,
  actorId: null,
  messageId: null,
  hop: 0,
  detail: null,
  webhookStatus: "delivered",
  webhookAttempts: 1,
  webhookLastError: null,
  external: null,
  handoffSummary: null,
  readable: true,
} as const;

const PEOPLE = [USER_IDS.riel, USER_IDS.maya, USER_IDS.jonah, USER_IDS.priya, USER_IDS.theo];

function person(index: number): number {
  return PEOPLE[index % PEOPLE.length] ?? USER_IDS.riel;
}

/** The ledger module. */
export interface Ledger {
  readonly routes: readonly Route[];
  /** Seeds the ledgers; after the world, since entries quote its messages. */
  seed(): void;
}

/** Creates the ledger module. */
export function createLedger(ctx: S2Context, agents: Agents): Ledger {
  let records: LedgerRecord[] = [];

  /** The `back`-th newest root message of a room (0 the newest), or `null`. */
  const messageIn = (roomId: number, back: number) => {
    const messages = ctx.world().rooms.get(roomId)?.messages ?? [];

    return messages.at(-1 - (back % Math.max(messages.length, 1))) ?? null;
  };

  const TEMPLATES: readonly Template[] = [
    (index) => ({
      ...BLANK,
      eventType: "mention",
      outcome: "acknowledged",
      roomId: ROOM_IDS.engineering,
      actorId: person(index),
      messageId: messageIn(ROOM_IDS.engineering, index)?.id ?? null,
    }),
    (index) => ({
      ...BLANK,
      eventType: "direct_message",
      outcome: "delivered",
      roomId: ROOM_IDS.dmEmber,
      actorId: USER_IDS.riel,
      messageId: messageIn(ROOM_IDS.dmEmber, index)?.id ?? null,
    }),
    () => ({
      ...BLANK,
      eventType: "github_action_completed",
      outcome: "delivered",
      roomId: ROOM_IDS.engineering,
      external: {
        action: "merge_pull_request",
        status: "succeeded",
        message: "Merged #318 into main (3 commits, squash)",
      },
    }),
    (index) => ({
      ...BLANK,
      eventType: "approval_decided",
      outcome: "delivered",
      roomId: ROOM_IDS.engineering,
      actorId: person(index + 3),
      detail: "Approved: Merge PR #318 into main",
    }),
    (index) => ({
      ...BLANK,
      eventType: "reply",
      outcome: "pending",
      roomId: ROOM_IDS.general,
      actorId: person(index + 1),
      messageId: messageIn(ROOM_IDS.general, index)?.id ?? null,
      webhookStatus: "pending",
      webhookAttempts: 0,
    }),
    () => ({
      ...BLANK,
      eventType: "fizzy_action_completed",
      outcome: "delivered",
      roomId: ROOM_IDS.design,
      external: { action: "create_card", status: "created", message: "Card #42 on Launch board" },
    }),
    (index) => ({
      ...BLANK,
      eventType: "work_handed_off",
      outcome: "acknowledged",
      roomId: ROOM_IDS.engineering,
      actorId: person(index),
      hop: 1,
      handoffSummary: cut(
        "Release 2.0.1 is green on staging; the only open item is the Safari reconnect bug, which Scout is reproducing. Production deploy waits on Riel's approval in #engineering.",
      ),
    }),
    () => ({
      ...BLANK,
      eventType: "posted",
      outcome: null,
      roomId: ROOM_IDS.dmEmber,
      messageId: messageIn(ROOM_IDS.dmEmber, 0)?.id ?? null,
      webhookStatus: "none",
      webhookAttempts: 0,
    }),
    (index) => ({
      ...BLANK,
      eventType: "delivery_suppressed_rate_limit",
      outcome: "suppressed",
      roomId: ROOM_IDS.general,
      actorId: person(index + 2),
      detail: "Rate limit: more than 30 deliveries in a minute",
      webhookStatus: "none",
      webhookAttempts: 0,
    }),
    (index) => ({
      ...BLANK,
      eventType: "mention",
      outcome: "delivered",
      roomId: ROOM_IDS.engineering,
      actorId: person(index + 4),
      messageId: messageIn(ROOM_IDS.engineering, index + 7)?.id ?? null,
      webhookStatus: "failed",
      webhookAttempts: 3,
      webhookLastError: "HTTP 502 from hooks.example.dev after 10s",
    }),
    (index) => ({
      ...BLANK,
      eventType: "slash_command",
      outcome: "delivered",
      roomId: ROOM_IDS.general,
      actorId: person(index),
      detail: "/ember summarize since yesterday",
    }),
    (index) => ({
      ...BLANK,
      eventType: "work_assigned",
      outcome: "delivered",
      roomId: ROOM_IDS.engineering,
      actorId: person(index + 1),
      detail: "Fix the Safari websocket reconnect",
    }),
    () => ({
      ...BLANK,
      eventType: "delivery_suppressed_hop_limit",
      outcome: "suppressed",
      roomId: ROOM_IDS.engineering,
      hop: 3,
      detail: "Hop limit: 3 agent-to-agent hops",
      webhookStatus: "none",
      webhookAttempts: 0,
    }),
    // In a room the viewer isn't in: gated unless they're an administrator.
    (index) => ({
      ...BLANK,
      eventType: "mention",
      outcome: "delivered",
      roomId: HIDDEN_ROOM.id,
      actorId: person(index + 3),
      messageId: 120_000 + index,
      detail: "Paged for the 02:00 disk alert on the primary database",
    }),
    () => ({
      ...BLANK,
      eventType: UNKNOWN_LEDGER_TYPE,
      outcome: "delivered",
      detail: "Daily message budget 80% used",
      webhookStatus: "none",
      webhookAttempts: 0,
    }),
    (index) => ({
      ...BLANK,
      eventType: "work_unassigned",
      outcome: "delivered",
      roomId: ROOM_IDS.engineering,
      actorId: person(index + 2),
      detail: "Write the 2.0.1 release notes",
    }),
    () => ({
      ...BLANK,
      eventType: "delivery_suppressed_revoked",
      outcome: "suppressed",
      roomId: ROOM_IDS.design,
      detail: "Grant revoked: read_messages in #design",
      webhookStatus: "none",
      webhookAttempts: 0,
    }),
    (index) => ({
      ...BLANK,
      eventType: "reply",
      outcome: "delivered",
      roomId: ROOM_IDS.general,
      actorId: person(index),
      messageId: messageIn(ROOM_IDS.general, index + 3)?.id ?? null,
      readable: false,
    }),
  ];

  const seed = () => {
    const now = ctx.now();

    records = [];

    for (let index = 0; index < EMBER_LEDGER_SIZE; index++) {
      const template = TEMPLATES[index % TEMPLATES.length];

      if (template === undefined) continue;

      // Two pairs share a timestamp: 6 and 7, and 49 and 50 (across the first page's end).
      const slot = index === 7 || index === 50 ? index - 1 : index;

      records.push({
        ...template(index),
        id: 5000 - index,
        agentId: AGENT_IDS.ember,
        createdAt: now - 3 * MINUTE - slot * 47 * MINUTE,
      });
    }

    const few = (agentId: number, first: number, picks: readonly number[]) => {
      picks.forEach((pick, index) => {
        const template = TEMPLATES[pick];

        if (template === undefined) return;

        records.push({
          ...template(index),
          id: first - index,
          agentId,
          createdAt: now - (index + 1) * 5 * HOUR,
        });
      });
    };

    few(AGENT_IDS.scout, 6000, [0, 4, 8, 15]);
    few(AGENT_IDS.atlas, 7000, [2, 12]);
  };

  const present = (record: LedgerRecord): LedgerWire => {
    const roomName = viewerRoomName(ctx, record.roomId);
    const open = record.roomId === null || roomName !== null;
    const message = record.messageId === null ? null : findMessage(record.roomId, record.messageId);

    return {
      id: record.id,
      eventType: record.eventType,
      outcome: record.outcome,
      createdAt: iso(record.createdAt),
      roomId: record.roomId,
      roomName,
      actorId: record.actorId,
      messageId: record.messageId,
      hop: record.hop,
      detail: open ? record.detail : null,
      webhookStatus: record.webhookStatus,
      webhookAttempts: record.webhookAttempts,
      webhookLastError: record.webhookLastError,
      external:
        record.external === null
          ? null
          : { ...record.external, message: open ? record.external.message : null },
      handoffSummary: open ? record.handoffSummary : null,
      content:
        open && record.readable && message !== null ? cut(plainText(message.markdownSource)) : null,
    };
  };

  const findMessage = (roomId: number | null, messageId: number) =>
    roomId === null
      ? null
      : (ctx
          .world()
          .rooms.get(roomId)
          ?.messages.find((message) => message.id === messageId) ?? null);

  const list = (agentId: number, query: URLSearchParams) => {
    const row = agents.record(agentId);

    if (row === null) throw notFound("Agent not found");

    if (!viewerManages(ctx, row))
      throw forbidden("Only an administrator or the agent's owner can see its activity");

    const outcome = query.get("outcome");

    const known = (["pending", "delivered", "acknowledged", "suppressed"] as const).find(
      (candidate) => candidate === outcome,
    );

    const matching = records.filter(
      (record) => record.agentId === agentId && (known === undefined || record.outcome === known),
    );

    const page = keysetPage(
      matching,
      (record) => ({ at: iso(record.createdAt), id: record.id }),
      beforeOf(query),
      LEDGER_PAGE_SIZE,
    );

    const userIds = [
      row.userId,
      ...page.rows.flatMap((record) => (record.actorId === null ? [] : [record.actorId])),
    ];

    return {
      events: page.rows.map(present),
      users: ctx.usersFor(userIds),
      nextCursor: page.nextCursor,
    };
  };

  return {
    routes: [
      route("GET", /^\/agents\/(\d+)\/events$/, (request) =>
        ok(list(firstId(request), request.query)),
      ),
    ],
    seed,
  };
}
