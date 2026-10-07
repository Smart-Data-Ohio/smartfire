/**
 * The Saved page: the viewer's saved items newest first (by status, keyset-paged with their
 * messages, authors and conversation names), marking one done or reopening it, and reminders: a
 * timer per item with a pending `remindAt` that, when due, stamps `remindedAt` and records a
 * `message_reminder` in the inbox. Saving and unsaving stay in the S2 messages module, which
 * tells this one through `savedChanged`.
 */
import type { ActivityItem } from "../../src/gen/ActivityItem.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { SavedFilter } from "../../src/gen/SavedFilter.ts";
import type { SavedItem } from "../../src/gen/SavedItem.ts";
import type { SavedItemList } from "../../src/gen/SavedItemList.ts";
import type { SavedStatus } from "../../src/gen/SavedStatus.ts";
import { notFound, ok, validation } from "../http.ts";
import { stringField } from "../json.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { iso, locate } from "../s2/model.ts";
import type { Activity } from "./activity.ts";
import { conversationNames } from "./conversations.ts";
import { beforeOf, keysetPage, oneOf } from "./model.ts";
import { reminderSource } from "./sources.ts";

/** At most this many items a page. */
export const SAVED_PAGE_SIZE = 50;

const FILTERS: readonly SavedFilter[] = ["all", "in_progress", "done"];

const STATUSES: readonly SavedStatus[] = ["in_progress", "done"];

/** The saved-items module. */
export interface Saved {
  readonly routes: readonly Route[];
  /**
   * The S2 messages module saved, re-saved or unsaved a message (or deleted a saved one):
   * re-arms its reminder, or drops the reminder and its inbox items.
   */
  savedChanged(messageId: number, before: SavedItem | null, after: SavedItem | null): void;
  /** Sends every reminder that's due now (every pending one with `all`); answers how many. */
  remindDue(all: boolean): number;
  /** Cancels the timers and arms one per pending reminder in the current world. */
  arm(): void;
  /** Cancels every timer. */
  stop(): void;
}

/** Creates the saved-items module. */
export function createSaved(ctx: S2Context, activity: Activity): Saved {
  const timers = new Map<number, number>();

  /** The message, if the viewer can still reach it (a current member of a live room). */
  const reachable = (messageId: number): MessageDTO | null => {
    const location = locate(ctx.world(), messageId);

    if (location === null) return null;

    try {
      ctx.roomOr404(location.room.room.id);
    } catch {
      return null;
    }

    return location.message;
  };

  const itemOr404 = (savedItemId: number): SavedItem => {
    for (const item of ctx.world().saved.values()) {
      if (item.id === savedItemId && reachable(item.messageId) !== null) return item;
    }

    throw notFound("Saved item not found");
  };

  const keyOf = (item: SavedItem) => ({ at: item.createdAt, id: item.id });

  const list = (query: URLSearchParams): SavedItemList => {
    const world = ctx.world();
    const filter = oneOf(query.get("status"), FILTERS, "all");

    const matching = [...world.saved.values()].filter(
      (item) => (filter === "all" || item.status === filter) && reachable(item.messageId) !== null,
    );

    const page = keysetPage(matching, keyOf, beforeOf(query), SAVED_PAGE_SIZE);

    const messages = page.rows.flatMap((item) => reachable(item.messageId) ?? []);

    return {
      items: [...page.rows],
      messages,
      users: ctx.usersFor(messages.map((message) => message.creatorId)),
      conversations: conversationNames(ctx, messages),
      nextCursor: page.nextCursor,
    };
  };

  const update = (savedItemId: number, rawStatus: string | null): SavedItem => {
    const current = itemOr404(savedItemId);
    const status = STATUSES.find((candidate) => candidate === rawStatus);

    if (status === undefined) throw validation("status", "Status must be in_progress or done");

    const next: SavedItem = { ...current, status };

    ctx.world().saved.set(current.messageId, next);
    ctx.publish([
      { topic: "user", type: "saved.changed", data: { messageId: current.messageId, item: next } },
    ]);

    return next;
  };

  // --- reminders ---

  const disarm = (savedItemId: number) => {
    const timer = timers.get(savedItemId);

    if (timer !== undefined) ctx.scheduler.cancel(timer);

    timers.delete(savedItemId);
  };

  const remind = (item: SavedItem) => {
    disarm(item.id);

    const message = reachable(item.messageId);

    if (message === null) return;

    const next: SavedItem = { ...item, remindedAt: iso(ctx.now()) };

    ctx.world().saved.set(item.messageId, next);
    ctx.publish([
      { topic: "user", type: "saved.changed", data: { messageId: item.messageId, item: next } },
    ]);
    activity.record({
      eventType: "message_reminder",
      source: reminderSource(ctx, item.id, message),
    });
  };

  const armOne = (item: SavedItem) => {
    disarm(item.id);

    if (item.remindAt === null || item.remindedAt !== null) return;

    const wait = Math.max(0, Date.parse(item.remindAt) - ctx.now());

    timers.set(
      item.id,
      ctx.scheduler.schedule(wait, () => {
        timers.delete(item.id);

        const current = ctx.world().saved.get(item.messageId);

        if (current?.id === item.id && current.remindedAt === null) remind(current);
      }),
    );
  };

  const isReminderOf = (savedItemId: number) => (candidate: ActivityItem) =>
    candidate.source.sourceType === "saved_item" && candidate.source.sourceId === savedItemId;

  return {
    routes: [
      route("GET", /^\/saved$/, (request) => ok(list(request.query))),
      route("PATCH", /^\/saved\/(\d+)$/, (request) =>
        ok(update(firstId(request), stringField(request.body, "status"))),
      ),
    ],
    savedChanged(_messageId, before, after) {
      if (after === null) {
        if (before === null) return;

        disarm(before.id);
        activity.removeWhere(isReminderOf(before.id));

        return;
      }

      armOne(after);
    },
    remindDue(all) {
      const due = [...ctx.world().saved.values()].filter(
        (item) =>
          item.remindAt !== null &&
          item.remindedAt === null &&
          (all || Date.parse(item.remindAt) <= ctx.now()),
      );

      for (const item of due) remind(item);

      return due.length;
    },
    arm() {
      for (const id of [...timers.keys()]) disarm(id);

      for (const item of ctx.world().saved.values()) armOne(item);
    },
    stop() {
      for (const id of [...timers.keys()]) disarm(id);
    },
  };
}
