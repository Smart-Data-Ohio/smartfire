/**
 * S3's share of the simulation: now and then something lands in the viewer's inbox: someone
 * starts a huddle, mentions the viewer in a live room, an agent asks for approval, or an event
 * invitation arrives. It runs on its own PRNG and timer loop, so the S1 and S2 sequences are
 * unchanged. (Due saved reminders arrive on their own timers, in the saved module.)
 */
import type { ActivitySource } from "../../src/gen/ActivitySource.ts";
import type { Random } from "../random.ts";
import type { S2Context } from "../s2/context.ts";
import { iso, plainDraft } from "../s2/model.ts";
import type { Scheduler } from "../scheduler.ts";
import { BOT_ID, ROOM_IDS, VIEWER_ID } from "../seed.ts";
import type { Activity } from "./activity.ts";
import { conversationTitle } from "./conversations.ts";
import { huddleSource, messageSource } from "./sources.ts";

const MENTIONS = [
  "@Riel quick one: can you sanity-check the numbers in the doc?",
  "@Riel are you around for five minutes after lunch?",
  "Adding @Riel here since this touches the pricing page.",
  "@Riel the build is green again, over to you.",
];

const APPROVALS = [
  "Restart the staging worker pool",
  "Post the release notes to #announcements",
  "Re-run the nightly backup check",
  "Label 6 new issues from the support queue",
];

const EVENTS = ["Pricing review", "Design sync", "Launch go/no-go", "Customer call prep"];

/** What one arrival needs from the server. */
export interface InboxAmbientHost {
  /** Rooms someone could start a huddle or mention the viewer in, with who could. */
  rooms(): readonly { readonly roomId: number; readonly people: readonly number[] }[];
  /** `userId` starts a huddle in a room (a `huddle_started` item). */
  huddle(roomId: number, userId: number): void;
  /** `userId` posts a message mentioning the viewer (a `mention` item). */
  mention(roomId: number, userId: number, markdown: string): void;
  /** The agent asks for approval (an `agent_approval_request` item). */
  approval(summary: string): void;
  /** `userId` invites the viewer to an event in a room (an `event_invitation` item). */
  invitation(roomId: number, userId: number, title: string): void;
  paused(): boolean;
}

/** The inbox's ambient loop. */
export interface InboxAmbient {
  start(): void;
  stop(): void;
  /** One arrival now, whatever the timer says (the `__mock/activity-arrival` control). */
  arrive(): void;
}

/** Creates the inbox's ambient loop. */
export function createInboxAmbient(
  host: InboxAmbientHost,
  scheduler: Scheduler,
  random: Random,
): InboxAmbient {
  let timer: number | null = null;
  let running = false;

  const arrive = () => {
    const rooms = host.rooms().filter((candidate) => candidate.people.length > 0);

    if (rooms.length === 0) return;

    const target = random.pick(rooms);
    const person = random.pick(target.people);
    const roll = random.next();

    if (roll < 0.35) {
      host.huddle(target.roomId, person);
    } else if (roll < 0.7) {
      host.mention(target.roomId, person, random.pick(MENTIONS));
    } else if (roll < 0.85) {
      host.approval(random.pick(APPROVALS));
    } else {
      host.invitation(target.roomId, person, random.pick(EVENTS));
    }
  };

  const tick = () => {
    timer = null;

    if (!running) return;

    if (!host.paused()) arrive();

    timer = scheduler.schedule(random.int(45_000, 120_000), tick);
  };

  return {
    start() {
      if (running) return;

      running = true;
      timer = scheduler.schedule(random.int(45_000, 120_000), tick);
    },
    stop() {
      running = false;

      if (timer !== null) scheduler.cancel(timer);

      timer = null;
    },
    arrive,
  };
}

/** The rooms the inbox's live arrivals come from. */
const LIVE_ROOMS = [
  ROOM_IDS.general,
  ROOM_IDS.design,
  ROOM_IDS.engineering,
  ROOM_IDS.launchPlanning,
  ROOM_IDS.lounge,
  ROOM_IDS.dmMaya,
  ROOM_IDS.groupDm,
];

/** Ids for live sources, clear of the seeded ones. */
const FIRST_LIVE_SOURCE_ID = 1000;

/** The inbox's ambient loop on a server: arrivals become real messages and inbox items. */
export function createServerInboxAmbient(
  ctx: S2Context,
  activity: Activity,
  paused: () => boolean,
  random: Random,
): InboxAmbient {
  let nextSourceId = FIRST_LIVE_SOURCE_ID;

  const plain = (
    sourceType: ActivitySource["sourceType"],
    roomId: number | null,
    creatorId: number,
    title: string,
    body: string,
    path: string,
    extra: Partial<ActivitySource> = {},
  ): ActivitySource => ({
    sourceType,
    sourceId: nextSourceId++,
    roomId,
    threadId: null,
    messageId: null,
    eventId: null,
    creatorId,
    title,
    body,
    occurredAt: iso(ctx.now()),
    approvalStatus: null,
    budgetCap: null,
    path,
    ...extra,
  });

  return createInboxAmbient(
    {
      rooms: () =>
        LIVE_ROOMS.flatMap((roomId) => {
          const record = ctx.world().rooms.get(roomId);

          return record === undefined
            ? []
            : [
                {
                  roomId,
                  people: record.memberIds.filter((id) => id !== VIEWER_ID && id !== BOT_ID),
                },
              ];
        }),
      huddle(roomId, userId) {
        activity.record({
          eventType: "huddle_started",
          source: huddleSource(ctx, nextSourceId++, roomId, userId, false, iso(ctx.now())),
        });
      },
      mention(roomId, userId, markdown) {
        const record = ctx.world().rooms.get(roomId);

        if (record === undefined) return;

        const message = ctx.postToRoom(record, plainDraft(userId, markdown, ctx.uuid()));

        activity.record({ eventType: "mention", source: messageSource(ctx, message) });
      },
      approval(summary) {
        activity.record({
          eventType: "agent_approval_request",
          source: plain(
            "agent_approval",
            ROOM_IDS.engineering,
            BOT_ID,
            conversationTitle(ctx, ROOM_IDS.engineering, null, "Ember"),
            summary,
            "/agents/1/approvals",
            { approvalStatus: "pending" },
          ),
        });
      },
      invitation(roomId, userId, title) {
        const source = plain(
          "event",
          roomId,
          userId,
          conversationTitle(ctx, roomId, null, title),
          `You are invited: ${new Date(ctx.now() + 2 * 86_400_000).toUTCString()}.`,
          "",
        );

        activity.record({
          eventType: "event_invitation",
          source: {
            ...source,
            eventId: source.sourceId,
            path: `/rooms/${roomId}/events/${source.sourceId}`,
          },
        });
      },
      paused,
    },
    ctx.scheduler,
    random,
  );
}
