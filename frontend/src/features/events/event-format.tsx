import type { ChannelEvent } from "../../gen/ChannelEvent.ts";
import type { EventDetail } from "../../gen/EventDetail.ts";
import { eventTile, localEventWhen } from "../cards/format.ts";

/**
 * The month-and-day tile an event's row and page lead with, in the viewer's zone like the times
 * beside it (see `EventWhen`).
 */
export function EventTileMark({
  startsAt,
  size = "md",
}: {
  readonly startsAt: string;
  readonly size?: "md" | "lg";
}) {
  const tile = eventTile(startsAt, null);

  return (
    <div className="ev-tile" data-size={size} aria-hidden="true">
      <span className="ev-tile-month">{tile.month}</span>
      <span className="ev-tile-day tabular">{tile.day}</span>
    </div>
  );
}

/**
 * When an event is, as the classic calendar and event page show it: in the viewer's own zone,
 * then its `zoneLabel`, the times in the zone it's scheduled in ("(11:00 AM–11:45 AM EDT)").
 */
export function EventWhen({ event }: { readonly event: ChannelEvent }) {
  return (
    <>
      {localEventWhen(event.startsAt, event.endsAt)}{" "}
      <span className="ev-zone">{event.zoneLabel}</span>
    </>
  );
}

const DAY = 24 * 60 * 60 * 1000;

const RELATIVE = new Map([
  [0, "Today"],
  [1, "Tomorrow"],
]);

/** Midnight at the start of `millis`'s day, in the viewer's zone. */
function dayStart(millis: number): number {
  const at = new Date(millis);

  at.setHours(0, 0, 0, 0);

  return at.getTime();
}

/**
 * "Happening now", "Today", "Tomorrow" or "In 3 days" for an event that hasn't ended, in the
 * viewer's own days; nothing for one further out or over.
 */
export function relativeDay(
  startsAt: string,
  endsAt: string | null,
  now: number,
): { readonly label: string; readonly live: boolean } | null {
  const start = Date.parse(startsAt);
  const end = endsAt === null ? start : Date.parse(endsAt);

  if (start <= now && now < end) {
    return { label: "Happening now", live: true };
  }

  if (start < now) {
    return null;
  }

  const days = Math.round((dayStart(start) - dayStart(now)) / DAY);

  if (days >= 7) {
    return null;
  }

  return { label: RELATIVE.get(days) ?? `In ${days} days`, live: false };
}

/** The classic notice after scheduling: a series says it repeats. */
export function scheduledNotice(detail: EventDetail): string {
  return detail.event.series ? "Repeating event scheduled." : "Event scheduled.";
}
