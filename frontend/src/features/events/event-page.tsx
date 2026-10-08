import { Link, useMatchRoute, useNavigate, useParams } from "@tanstack/react-router";
import { type ReactNode, useEffect, useRef, useState } from "react";
import type { AttendanceResponse } from "../../gen/AttendanceResponse.ts";
import type { EventDetail } from "../../gen/EventDetail.ts";
import type { EventScope } from "../../gen/EventScope.ts";
import { actions } from "../../sync/runtime.ts";
import { Avatar } from "../../ui/avatar.tsx";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PageFrame } from "../destinations/page-frame.tsx";
import { PaneEmpty, PaneError } from "../panes/pane-states.tsx";
import { EventFormDialog } from "./event-form-dialog.tsx";
import { EventTileMark, EventWhen } from "./event-format.tsx";
import { overPageState, useCloseOverlay } from "./overlay-history.ts";
import { ScopeChoice } from "./scope-choice.tsx";
import { useEventChanges } from "./use-event-changes.ts";
import { type Landing, type Loaded, useLoad } from "./use-load.ts";
import "./events.css";

const RESPONSES = [
  { response: "going", label: "Going" },
  { response: "maybe", label: "Maybe" },
  { response: "declined", label: "Declined" },
] as const satisfies readonly { response: AttendanceResponse; label: string }[];

const RESPONSE_LABEL = {
  going: "Going",
  maybe: "Maybe",
  declined: "Declined",
} as const satisfies Record<AttendanceResponse, string>;

function InfoRow({
  icon,
  label,
  children,
}: {
  readonly icon: IconName;
  readonly label: string;
  readonly children: ReactNode;
}) {
  return (
    <div className="ev-info-row">
      <dt>
        <Icon name={icon} size={16} />
        <span className="visually-hidden">{label}</span>
      </dt>
      <dd>{children}</dd>
    </div>
  );
}

/** Where, Meet, how it repeats (with the series' neighbours) and the calendar copy. */
function Facts({ detail }: { readonly detail: EventDetail }) {
  const { event } = detail;
  const venue = event.venue;

  return (
    <dl className="ev-info">
      <InfoRow icon="calendar" label="When">
        <span>
          <EventWhen event={event} />
        </span>
        <span className="ev-info-sub">Organized by {event.organizerName}</span>
      </InfoRow>
      {venue === null ? null : (
        <InfoRow icon="map-pin" label="Where">
          <span className="ev-info-line">
            <Icon name={venue.kind === "stage" ? "radio" : "volume"} size={14} />
            {venue.member ? (
              <Link to="/r/$roomId" params={{ roomId: venue.roomId }} className="ev-info-link">
                {venue.name}
              </Link>
            ) : (
              <span>{venue.name}</span>
            )}
            {venue.liveUser === null ? null : (
              <span className="ev-tag" data-tone="success">
                {venue.liveUser} is live
              </span>
            )}
          </span>
          {venue.member ? null : (
            <span className="ev-info-sub">Join the channel to take part from here.</span>
          )}
        </InfoRow>
      )}
      {detail.meetLink === null ? null : (
        <InfoRow icon="video" label="Google Meet">
          <a
            className="ev-info-link"
            href={detail.meetLink}
            target="_blank"
            rel="noopener noreferrer"
          >
            Join with Google Meet
          </a>
        </InfoRow>
      )}
      {detail.recurrencePhrase === null ? null : (
        <InfoRow icon="repeat-2" label="Repeats">
          <span>
            Repeats {detail.recurrencePhrase}
            {detail.recurrenceUntil === null ? "" : ` until ${detail.recurrenceUntil}`}
          </span>
          {detail.previousOccurrenceId === null && detail.nextOccurrenceId === null ? null : (
            <span className="ev-info-neighbours">
              {detail.previousOccurrenceId === null ? null : (
                <Link
                  to="/r/$roomId/events/$eventId"
                  params={{ roomId: detail.roomId, eventId: detail.previousOccurrenceId }}
                  className="ev-info-link"
                >
                  <Icon name="chevron-left" size={14} />
                  Previous
                </Link>
              )}
              {detail.nextOccurrenceId === null ? null : (
                <Link
                  to="/r/$roomId/events/$eventId"
                  params={{ roomId: detail.roomId, eventId: detail.nextOccurrenceId }}
                  className="ev-info-link"
                >
                  Next
                  <Icon name="chevron-right" size={14} />
                </Link>
              )}
            </span>
          )}
        </InfoRow>
      )}
      {detail.calendarCopy ? (
        <InfoRow icon="calendar-clock" label="Your calendar">
          <span className="ev-info-sub">Added to your Google Calendar</span>
        </InfoRow>
      ) : null}
    </dl>
  );
}

/**
 * Going / Maybe / Declined, what the viewer answered, and for a series whether the answer covers
 * the future ones (the head's answer always does; a later occurrence asks).
 */
function Response({
  detail,
  focusOnMount,
  begin,
}: {
  readonly detail: EventDetail;
  readonly focusOnMount: boolean;
  /** Starts an answer whose reply (the page's facts) replaces the page's, if still the newest. */
  readonly begin: () => Landing<EventDetail>;
}) {
  const [future, setFuture] = useState(false);
  const [pending, setPending] = useState<AttendanceResponse | null>(null);
  // The latest answer: an earlier one's reply doesn't clear the one still on its way.
  const asked = useRef(0);
  const sectionRef = useRef<HTMLElement | null>(null);
  const current = pending ?? detail.currentResponse;

  useEffect(() => {
    const section = sectionRef.current;

    if (!focusOnMount || section === null) {
      return;
    }

    section.scrollIntoView({ block: "center" });
    section.querySelector<HTMLElement>("button[aria-pressed='true'], button")?.focus({
      preventScroll: true,
    });
  }, [focusOnMount]);

  const respond = (response: AttendanceResponse) => {
    asked.current += 1;

    const mine = asked.current;
    const landing = begin();

    setPending(response);
    actions.events
      .respond(
        detail.roomId,
        detail.event.id,
        response,
        detail.head || (future && detail.canApplyToFuture),
      )
      .then(
        (next) => {
          if (asked.current === mine) setPending(null);

          landing.land(next);
        },
        (failure: Error) => {
          if (asked.current === mine) setPending(null);

          landing.reread();
          toast({
            title: "Couldn't save your response",
            description: failure.message,
            tone: "danger",
          });
        },
      );
  };

  return (
    <section
      ref={sectionRef}
      className="ev-section ev-response"
      aria-labelledby="ev-response-title"
    >
      <h2 id="ev-response-title" className="ev-section-title">
        Your response
      </h2>
      {detail.event.cancelled ? (
        <p className="ev-muted">Responses are closed because this event was cancelled.</p>
      ) : (
        <>
          <fieldset className="ev-choices" aria-busy={pending !== null || undefined}>
            <legend className="visually-hidden">Your response</legend>
            {RESPONSES.map((choice) => (
              <Button
                key={choice.response}
                variant="pill"
                size="sm"
                className="ev-choice"
                data-response={choice.response}
                aria-pressed={current === choice.response}
                data-pending={pending === choice.response || undefined}
                disabled={!detail.respondable}
                onClick={() => respond(choice.response)}
              >
                {choice.label}
              </Button>
            ))}
          </fieldset>
          <p className="ev-muted" aria-live="polite">
            {current === null
              ? "You haven't answered yet."
              : `Currently: ${RESPONSE_LABEL[current]}`}
          </p>
          {detail.head && detail.event.series ? (
            <p className="ev-hint">
              Responding here applies your response to every future occurrence in this series.
            </p>
          ) : detail.canApplyToFuture && detail.respondable ? (
            <Checkbox
              checked={future}
              onCheckedChange={setFuture}
              label="Apply to all future occurrences"
            />
          ) : null}
        </>
      )}
    </section>
  );
}

/** Everyone who answered, grouped by their answer. */
function Attendees({ detail }: { readonly detail: EventDetail }) {
  const { counts } = detail.event;

  return (
    <section className="ev-section" aria-labelledby="ev-attendees-title">
      <h2 id="ev-attendees-title" className="ev-section-title">
        Attendees
        <span className="ev-counts tabular">
          {counts.going} going · {counts.maybe} maybe · {counts.declined} declined
        </span>
      </h2>
      {detail.attendees.length === 0 ? (
        <p className="ev-muted">No responses yet.</p>
      ) : (
        <div className="ev-attendee-groups">
          {RESPONSES.map(({ response, label }) => {
            const people = detail.attendees.filter((attendee) => attendee.response === response);

            return people.length === 0 ? null : (
              <div key={response} className="ev-attendee-group" data-response={response}>
                <h3 className="ev-attendee-heading">
                  {label} <span className="tabular">{people.length}</span>
                </h3>
                <ul className="ev-attendees">
                  {people.map((attendee) => (
                    <li key={attendee.name} className="ev-attendee">
                      <Avatar name={attendee.name} size={24} decorative />
                      <span>{attendee.name}</span>
                    </li>
                  ))}
                </ul>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

/** "Cancel this event?", with this-or-following for a series. */
function CancelDialog({
  detail,
  open,
  begin,
  onClose,
  onCancelled,
}: {
  readonly detail: EventDetail;
  readonly open: boolean;
  /** Starts the cancel, whose reply (the page's facts) replaces the page's, if still the newest. */
  readonly begin: () => Landing<EventDetail>;
  readonly onClose: () => void;
  readonly onCancelled: () => void;
}) {
  const [scope, setScope] = useState<EventScope>("this_event");
  const [busy, setBusy] = useState(false);

  const cancel = () => {
    const landing = begin();

    setBusy(true);
    actions.events
      .cancel(detail.roomId, detail.event.id, { cancelScope: detail.event.series ? scope : null })
      .then(
        (next) => {
          setBusy(false);
          landing.land(next);
          onCancelled();
        },
        (failure: Error) => {
          setBusy(false);
          landing.reread();
          toast({
            title: "Couldn't cancel the event",
            description: failure.message,
            tone: "danger",
          });
        },
      );
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onClose();
      }}
      role="alertdialog"
      size="sm"
      title="Cancel this event?"
      description="Attendees will be notified."
      footer={
        <>
          <Button variant="secondary" onClick={onClose} data-autofocus>
            Keep event
          </Button>
          <Button variant="danger" loading={busy} loadingLabel="Cancelling…" onClick={cancel}>
            Cancel event
          </Button>
        </>
      }
    >
      {detail.event.series ? (
        <ScopeChoice
          legend="Cancel"
          value={scope}
          options={["this_event", "this_and_following"]}
          onChange={setScope}
        />
      ) : null}
    </Dialog>
  );
}

/** The event still loading, gone, or failed to load. */
function DetailPending({
  state,
  onRetry,
}: {
  readonly state: Loaded<EventDetail>;
  readonly onRetry: () => void;
}) {
  if (state.status !== "error") {
    return <DetailSkeleton />;
  }

  if (state.missing) {
    return (
      <PaneEmpty
        icon="calendar"
        title="This event isn't here"
        text="It may have been removed, or it's in a conversation you're not in."
      />
    );
  }

  return <PaneError message={`The event couldn't be loaded: ${state.message}`} onRetry={onRetry} />;
}

function DetailSkeleton() {
  return (
    <div className="ev-detail" role="status" aria-busy="true" aria-label="Loading the event">
      <div className="ev-hero">
        <Skeleton width={56} height={60} radius="md" />
        <div className="ev-hero-text">
          <Skeleton width="60%" height={20} />
          <Skeleton width="40%" height={12} />
        </div>
      </div>
      <Skeleton width="100%" height={96} radius="md" />
      <Skeleton width="50%" height={28} radius="pill" />
    </div>
  );
}

/**
 * `/app/r/$roomId/events/$eventId`: an event's page (the classic show page). When and where, how
 * it repeats, the description, the viewer's response and who's coming; the organizer and admins
 * edit it at `…/edit` and cancel it here. `…/attendance` opens on the response.
 */
export function EventRoute() {
  const { roomId, eventId } = useParams({ from: "/shell/r/$roomId/events/$eventId" });
  const navigate = useNavigate();
  const matchRoute = useMatchRoute();
  const editing = matchRoute({ to: "/r/$roomId/events/$eventId/edit" }) !== false;
  const answering = matchRoute({ to: "/r/$roomId/events/$eventId/attendance" }) !== false;
  const [cancelling, setCancelling] = useState(false);
  const read = () => actions.events.read(roomId, eventId);
  const { state, reload, refresh, begin } = useLoad(`${roomId}/${eventId}`, read);
  const detail = state.status === "ready" ? state.value : null;

  useEventChanges(roomId, refresh);

  const closeEdit = useCloseOverlay(() => {
    void navigate({ to: "/r/$roomId/events/$eventId", params: { roomId, eventId }, replace: true });
  });

  // `current` is false once the viewer dismissed the form while it saved: the dismissal already
  // left the form, so only the page's facts change. `landing` is the turn the save took when it
  // started, so the reply loses to an answer or a save made meanwhile, and never lands on (or
  // holds up) another event the viewer went to.
  const saved = (next: EventDetail, current: boolean, landing: Landing<EventDetail> | null) => {
    toast({ title: "Event updated.", tone: "success" });

    if (next.event.id === eventId) {
      landing?.land(next);

      if (current) closeEdit();
    } else if (current) {
      // "This and following" splits the series: the edited event is the new one's head.
      void navigate({
        to: "/r/$roomId/events/$eventId",
        params: { roomId, eventId: next.event.id },
        replace: true,
      });
    } else {
      landing?.reread();
    }
  };

  const manage =
    detail?.manageable === true && !detail.event.cancelled ? (
      <>
        <Link
          to="/r/$roomId/events/$eventId/edit"
          params={{ roomId, eventId }}
          state={overPageState()}
          className="button"
          data-variant="secondary"
          data-size="sm"
        >
          <Icon name="pencil" size={14} />
          Edit
        </Link>
        <Button variant="ghost" size="sm" icon="ban" onClick={() => setCancelling(true)}>
          Cancel event
        </Button>
      </>
    ) : null;

  return (
    <PageFrame
      title={detail?.event.title ?? "Event"}
      icon="calendar"
      meta={
        detail?.event.cancelled === true ? (
          <span className="ev-tag" data-tone="danger">
            Cancelled
          </span>
        ) : null
      }
      tools={manage ?? undefined}
    >
      <div className="ev-scroll">
        <Link to="/r/$roomId/events" params={{ roomId }} className="ev-crumb">
          <Icon name="chevron-left" size={14} />
          {detail === null ? "All events" : `All events in ${detail.roomName}`}
        </Link>
        {detail === null ? (
          <DetailPending state={state} onRetry={reload} />
        ) : (
          <article key={detail.event.id} className="ev-detail enter-fade">
            {detail.event.cancelled ? (
              <p className="ev-banner" role="status">
                <Icon name="ban" size={16} />
                This event was cancelled.
              </p>
            ) : null}
            <header className="ev-hero">
              <EventTileMark startsAt={detail.event.startsAt} size="lg" />
              <div className="ev-hero-text">
                <h2 className="ev-hero-title">{detail.event.title}</h2>
                <p className="ev-hero-when">
                  <EventWhen event={detail.event} />
                </p>
              </div>
            </header>
            <Facts detail={detail} />
            {detail.descriptionHtml === null ? null : (
              <div
                className="ev-description"
                // biome-ignore lint/security/noDangerouslySetInnerHtml: descriptionHtml is the server's sanitized simple_format output, the HTML the classic show page renders
                dangerouslySetInnerHTML={{ __html: detail.descriptionHtml }}
              />
            )}
            <Response
              key={`${detail.event.id}`}
              detail={detail}
              focusOnMount={answering}
              begin={begin}
            />
            <Attendees detail={detail} />
          </article>
        )}
      </div>
      {detail === null ? null : (
        <>
          <EventFormDialog
            roomId={roomId}
            eventId={eventId}
            open={editing && detail.manageable}
            onClose={closeEdit}
            begin={begin}
            onSaved={saved}
          />
          <CancelDialog
            detail={detail}
            open={cancelling}
            begin={begin}
            onClose={() => setCancelling(false)}
            onCancelled={() => {
              setCancelling(false);
              toast({ title: "Event cancelled.", tone: "success" });
            }}
          />
        </>
      )}
    </PageFrame>
  );
}
