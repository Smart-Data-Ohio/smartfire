import { Link } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import type { AttendanceResponse } from "../../gen/AttendanceResponse.ts";
import type { EventAttendance } from "../../gen/EventAttendance.ts";
import type { EventCard as EventCardData } from "../../gen/EventCard.ts";
import { AnimatedNumber } from "../../motion/animated-number.tsx";
import { attendanceKey, shownAttendance } from "../../store/cards.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { toast } from "../../ui/toast-store.ts";
import { useUser } from "../people/people.ts";
import { eventTile, eventWhen } from "./format.ts";
import { usePreview } from "./use-preview.ts";

interface Choice {
  readonly response: AttendanceResponse;
  readonly label: string;
  readonly count: (attendance: EventAttendance) => number;
}

const CHOICES: readonly Choice[] = [
  { response: "going", label: "Going", count: (attendance) => attendance.goingCount },
  { response: "maybe", label: "Maybe", count: (attendance) => attendance.maybeCount },
  { response: "declined", label: "Can't go", count: (attendance) => attendance.declinedCount },
];

function respondError(error: Error): void {
  toast({ title: "Couldn't save your response", description: error.message, tone: "danger" });
}

/**
 * Going / Maybe / Can't go with their counts, and "all future occurrences" for a series. While an
 * answer is on its way (`pending`) the group says it's busy and the chosen pill dims a little;
 * the pills stay enabled (focus stays put), and another answer just queues behind it.
 */
function Attendance({
  event,
  attendance,
  pending,
}: {
  readonly event: EventCardData;
  readonly attendance: EventAttendance;
  readonly pending: boolean;
}) {
  const [future, setFuture] = useState(false);

  const respond = (response: AttendanceResponse) => {
    actions.cards
      .respond(event.roomId, event.eventId, response, future && attendance.canApplyToFuture)
      .catch(respondError);
  };

  return (
    <div className="event-attendance">
      <fieldset className="event-choices" aria-busy={pending || undefined}>
        <legend className="visually-hidden">Your response</legend>
        {CHOICES.map((choice) => (
          <Button
            key={choice.response}
            variant="pill"
            size="sm"
            className="event-choice"
            data-response={choice.response}
            aria-pressed={attendance.response === choice.response}
            data-pending={(pending && attendance.response === choice.response) || undefined}
            disabled={!attendance.respondable}
            onClick={() => respond(choice.response)}
          >
            {choice.label}
            <span className="event-choice-count tabular">
              <AnimatedNumber value={choice.count(attendance)} />
            </span>
          </Button>
        ))}
      </fieldset>
      {attendance.canApplyToFuture && attendance.respondable ? (
        <Checkbox
          checked={future}
          onCheckedChange={setFuture}
          label="Apply to all future occurrences"
        />
      ) : null}
    </div>
  );
}

/** The event's classic page, as the classic card links it. */
function eventPath(event: EventCardData): string {
  return `/rooms/${event.roomId}/events/${event.eventId}`;
}

/** Where it's held: the voice room in the app, and/or a Meet link. */
function Venue({ event }: { readonly event: EventCardData }) {
  if (event.venueRoomId === null && event.meetLink === null) {
    return null;
  }

  return (
    <div className="event-venue">
      {event.venueRoomId === null ? null : (
        <Link
          to="/r/$roomId"
          params={{ roomId: event.venueRoomId }}
          className="card-link event-venue-room"
          preload={false}
        >
          <Icon name="map-pin" size={14} />
          {event.venueName ?? "Voice room"}
        </Link>
      )}
      {event.meetLink === null ? null : (
        <a
          className="card-link event-meet"
          href={event.meetLink}
          target="_blank"
          rel="noopener noreferrer"
        >
          <Icon name="video" size={14} />
          Join with Google Meet
        </a>
      )}
    </div>
  );
}

/**
 * A calendar event the message links to: a date tile, when and where, and the viewer's own
 * response with everyone's counts (fetched per viewer, answered at once, put back if refused).
 * A cancelled event keeps its details struck through and takes no responses.
 */
export function EventCard({ event }: { readonly event: EventCardData }) {
  const organizer = useUser(event.organizerId);

  useEffect(() => {
    actions.ensureUsers([event.organizerId]).catch(() => undefined);
  }, [event.organizerId]);
  const tile = eventTile(event.startsAt);

  const load = useCallback(
    () => actions.cards.loadAttendance(event.roomId, event.eventId),
    [event.roomId, event.eventId],
  );

  const preview = usePreview("attendance", attendanceKey(event.eventId), load);
  const pending = useStore((state) => state.cards.pendingAnswers[event.eventId]);
  const fetched = preview?.value ?? null;
  const attendance = shownAttendance(fetched, pending);

  return (
    <section
      className="card event-card"
      data-cancelled={event.cancelled || undefined}
      aria-label={`Event: ${event.title}`}
    >
      <div className="event-tile" aria-hidden="true">
        <span className="event-tile-month">{tile.month}</span>
        <span className="event-tile-day tabular">{tile.day}</span>
      </div>
      <div className="event-main">
        <div className="event-heading">
          {/* The classic event page: not an SPA route yet, so a plain link that leaves the app. */}
          <a className="event-title" href={eventPath(event)}>
            {event.title}
          </a>
          {event.cancelled ? (
            <span className="card-tag" data-tone="danger">
              Cancelled
            </span>
          ) : null}
          {event.recurring ? (
            <span className="card-tag" data-tone="neutral">
              <Icon name="repeat-2" size={12} />
              Repeats
            </span>
          ) : null}
        </div>
        <div className="event-when">
          <Icon name="calendar" size={14} />
          <span>{eventWhen(event.startsAt, event.endsAt)}</span>
        </div>
        {organizer === undefined ? null : (
          <div className="card-subtle">Organized by {organizer.name}</div>
        )}
        <Venue event={event} />
        {event.cancelled ? null : preview?.status === "error" && attendance === null ? (
          <div className="card-error">
            <span>Couldn't load responses.</span>
            <Button variant="link" size="sm" onClick={() => load().catch(() => undefined)}>
              Retry
            </Button>
          </div>
        ) : (
          <SkeletonReveal
            loading={attendance === null}
            skeleton={
              <div className="event-choices">
                <Skeleton width={84} height={28} radius="pill" />
                <Skeleton width={84} height={28} radius="pill" />
                <Skeleton width={96} height={28} radius="pill" />
              </div>
            }
          >
            {attendance === null ? null : (
              <Attendance event={event} attendance={attendance} pending={pending !== undefined} />
            )}
          </SkeletonReveal>
        )}
      </div>
    </section>
  );
}
