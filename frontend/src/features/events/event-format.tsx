import type { EventDetail } from "../../gen/EventDetail.ts";
import { eventTile } from "../cards/format.ts";

/** The month-and-day tile an event's row and page lead with, in the event's zone. */
export function EventTileMark({
  startsAt,
  timeZone,
  size = "md",
}: {
  readonly startsAt: string;
  readonly timeZone: string;
  readonly size?: "md" | "lg";
}) {
  const tile = eventTile(startsAt, timeZone);

  return (
    <div className="ev-tile" data-size={size} aria-hidden="true">
      <span className="ev-tile-month">{tile.month}</span>
      <span className="ev-tile-day tabular">{tile.day}</span>
    </div>
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
