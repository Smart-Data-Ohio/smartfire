/**
 * The mock's sense of life: people type and post now and then, presence drifts, and the bot
 * answers the viewer with a streamed reply. Timer driven through the injected scheduler.
 */
import type { MessageDTO } from "../src/gen/MessageDTO.ts";
import type { Presence } from "../src/gen/Presence.ts";
import { BOT_REPLIES, LIVE_LINES } from "./corpus.ts";
import type { Random } from "./random.ts";
import type { Scheduler } from "./scheduler.ts";

/** Delay before the bot starts typing, after the viewer's message. */
export const BOT_TYPING_DELAY_MS = 500;

/** How long the bot "types" before its message appears. */
export const BOT_TYPING_MS = 1500;

/** The gap between streamed updates of the bot's message. */
export const BOT_STREAM_STEP_MS = 400;

/** What the simulation needs from the server. */
export interface SimulationHost {
  typing(roomId: number, userId: number, on: boolean): void;
  post(roomId: number, userId: number, markdown: string, streaming: boolean): MessageDTO | null;
  /** Rewrites a message's Markdown (an agent's growing reply) and publishes `message.updated`. */
  update(messageId: number, markdown: string, streaming: boolean): void;
  setPresence(userId: number, presence: Presence): void;
  /** Rooms some connection is subscribed to. */
  subscribedRooms(): number[];
  /** Rooms the ambient chatter may post in. */
  liveRooms(): number[];
  /** People (not the viewer, not the bot) who may post in a room. */
  posters(roomId: number): number[];
  /** First names of a room's other members, for `{name}` in canned lines. */
  firstNames(roomId: number, exceptUserId: number): string[];
  /** Humans whose presence may change. */
  presencePeople(): number[];
}

export interface Simulation {
  /** Starts the ambient loop (idempotent). */
  start(): void;
  /** Stops everything and cancels every timer. */
  stop(): void;
  pause(): void;
  resume(): void;
  paused(): boolean;
  /** The bot types, then streams a reply in `roomId`. Runs even while paused. */
  botReply(roomId: number): void;
}

export interface SimulationOptions {
  readonly host: SimulationHost;
  readonly scheduler: Scheduler;
  readonly random: Random;
  readonly botId: number;
}

const PRESENCES: readonly Presence[] = ["online", "online", "idle", "dnd", "offline"];

export function createSimulation(options: SimulationOptions): Simulation {
  const { host, scheduler, random, botId } = options;
  const timers = new Set<number>();
  let running = false;
  let paused = false;
  let replyIndex = 0;

  const later = (delayMs: number, run: () => void) => {
    const id = scheduler.schedule(delayMs, () => {
      timers.delete(id);
      run();
    });

    timers.add(id);
  };

  const fillNames = (line: string, roomId: number, authorId: number) => {
    const names = host.firstNames(roomId, authorId);

    return line.replace(/\{(@?)name\}/g, (_match, at: string) =>
      names.length > 0 ? `${at}${random.pick(names)}` : "everyone",
    );
  };

  const pickRoom = (): number | null => {
    const subscribed = host.subscribedRooms().filter((id) => host.liveRooms().includes(id));
    const pool = subscribed.length > 0 && random.chance(0.6) ? subscribed : host.liveRooms();

    return pool.length > 0 ? random.pick(pool) : null;
  };

  const chatter = () => {
    const roomId = pickRoom();

    if (roomId === null) return;

    const posters = host.posters(roomId);

    if (posters.length === 0) return;

    const authorId = random.pick(posters);
    const line = fillNames(random.pick(LIVE_LINES), roomId, authorId);

    host.typing(roomId, authorId, true);
    later(random.int(2000, 4000), () => {
      host.typing(roomId, authorId, false);
      host.post(roomId, authorId, line, false);
    });
  };

  const drift = () => {
    const people = host.presencePeople();

    if (people.length > 0) host.setPresence(random.pick(people), random.pick(PRESENCES));
  };

  const tick = () => {
    if (!running) return;

    if (!paused) {
      chatter();

      if (random.chance(0.2)) drift();
    }

    later(random.int(6000, 20_000), tick);
  };

  return {
    start() {
      if (running) return;
      running = true;
      later(random.int(6000, 20_000), tick);
    },
    stop() {
      running = false;

      for (const id of timers) scheduler.cancel(id);

      timers.clear();
    },
    pause() {
      paused = true;
    },
    resume() {
      paused = false;
    },
    paused: () => paused,
    botReply(roomId) {
      const phrases = BOT_REPLIES[replyIndex % BOT_REPLIES.length] ?? ["On it."];

      replyIndex += 1;
      later(BOT_TYPING_DELAY_MS, () => {
        host.typing(roomId, botId, true);
        later(BOT_TYPING_MS, () => {
          host.typing(roomId, botId, false);

          const message = host.post(roomId, botId, phrases[0] ?? "", phrases.length > 1);

          if (message === null) return;

          let text = phrases[0] ?? "";

          phrases.slice(1).forEach((phrase, index) => {
            later(BOT_STREAM_STEP_MS * (index + 1), () => {
              text += phrase;
              host.update(message.id, text, index < phrases.length - 2);
            });
          });
        });
      });
    },
  };
}
