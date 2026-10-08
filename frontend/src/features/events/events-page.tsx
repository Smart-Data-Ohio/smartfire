import { Link, useLocation, useMatchRoute, useNavigate, useParams } from "@tanstack/react-router";
import { useId, useState } from "react";
import type { ChannelEvent } from "../../gen/ChannelEvent.ts";
import type { EventDetail } from "../../gen/EventDetail.ts";
import type { EventList } from "../../gen/EventList.ts";
import { actions } from "../../sync/runtime.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { Tabs, tabId } from "../../ui/tabs.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PageFrame } from "../destinations/page-frame.tsx";
import { PaneEmpty, PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { useNow } from "../threads/use-now.ts";
import { EventFormDialog } from "./event-form-dialog.tsx";
import { EventTileMark, EventWhen, relativeDay, scheduledNotice } from "./event-format.tsx";
import { overPageState, useCloseOverlay } from "./overlay-history.ts";
import { useEventChanges } from "./use-event-changes.ts";
import { type Loaded, useLoad } from "./use-load.ts";
import "./events.css";

const SECTIONS = ["upcoming", "past", "cancelled"] as const;

type Section = (typeof SECTIONS)[number];

const EMPTY = {
  upcoming: {
    title: "Nothing coming up",
    text: "Schedule a check-in, a demo or a launch. Members get an invitation and answer from the event or its message.",
  },
  past: { title: "No past events", text: "Events move here once they're over." },
  cancelled: { title: "Nothing cancelled", text: "Cancelled events stay here for the record." },
} as const satisfies Record<Section, { title: string; text: string }>;

/** A row's second line: how many are going, maybe, and can't. */
function Counts({ event }: { readonly event: ChannelEvent }) {
  const { going, maybe, declined } = event.counts;

  if (going + maybe + declined === 0) {
    return <span className="ev-row-counts">No responses yet</span>;
  }

  return (
    <span className="ev-row-counts tabular">
      {going} going{maybe > 0 ? ` · ${maybe} maybe` : ""}
      {declined > 0 ? ` · ${declined} declined` : ""}
    </span>
  );
}

function EventRow({ event, now }: { readonly event: ChannelEvent; readonly now: number }) {
  const day = event.cancelled ? null : relativeDay(event.startsAt, event.endsAt, now);

  return (
    <li className="ev-row" data-cancelled={event.cancelled || undefined}>
      <EventTileMark startsAt={event.startsAt} />
      <div className="ev-row-main">
        <div className="ev-row-heading">
          <Link
            to="/r/$roomId/events/$eventId"
            params={{ roomId: event.roomId, eventId: event.id }}
            className="ev-row-title"
          >
            {event.title}
          </Link>
          {day === null ? null : (
            <span className="ev-tag" data-tone={day.live ? "success" : "neutral"}>
              {day.label}
            </span>
          )}
          {event.series ? (
            <span className="ev-tag" data-tone="neutral">
              <Icon name="repeat-2" size={12} />
              {event.recurrenceLabel ?? "Repeats"}
            </span>
          ) : null}
        </div>
        <p className="ev-row-when">
          <span>
            <EventWhen event={event} />
          </span>
          {event.remainingOccurrences === null || event.remainingOccurrences <= 1 ? null : (
            <span>{event.remainingOccurrences} more to come</span>
          )}
        </p>
        <p className="ev-row-meta">
          {event.venue === null ? null : (
            <span className="ev-row-venue">
              <Icon name={event.venue.kind === "stage" ? "radio" : "volume"} size={12} />
              {event.venue.name}
            </span>
          )}
          <span>by {event.organizerName}</span>
          <Counts event={event} />
        </p>
      </div>
    </li>
  );
}

/** The list still loading, or failed to. */
function ListPending({
  state,
  onRetry,
}: {
  readonly state: Loaded<EventList>;
  readonly onRetry: () => void;
}) {
  if (state.status === "error") {
    return (
      <PaneError message={`The events couldn't be loaded: ${state.message}`} onRetry={onRetry} />
    );
  }

  return <PaneListSkeleton rows={4} square={44} />;
}

function sectionRows(list: EventList, section: Section): readonly ChannelEvent[] {
  return list[section];
}

/**
 * `/app/r/$roomId/events`: a room's calendar (the classic events page). Upcoming, past and
 * cancelled events in tabs, each row linking its event's page; "New event" opens the form at
 * `…/events/new`, and scheduling one goes on to its page.
 */
export function EventsRoute() {
  const { roomId } = useParams({ from: "/shell/r/$roomId/events" });
  const navigate = useNavigate();
  const matchRoute = useMatchRoute();
  const creating = matchRoute({ to: "/r/$roomId/events/new" }) !== false;
  // The new form's raw query: a prefilled link's `event[…]` values (see `newEventPrefill`).
  const search = useLocation({ select: (location) => location.searchStr });
  const now = useNow();
  const tabsId = useId();
  const panelId = useId();
  const [section, setSection] = useState<Section>("upcoming");
  const { state, reload, refresh } = useLoad(`${roomId}`, () => actions.events.list(roomId));
  const list = state.status === "ready" ? state.value : null;

  useEventChanges(roomId, refresh);

  const closeForm = useCloseOverlay(() => {
    void navigate({ to: "/r/$roomId/events", params: { roomId }, replace: true });
  });

  const scheduled = (detail: EventDetail, current: boolean) => {
    toast({ title: scheduledNotice(detail), tone: "success" });

    // Dismissed while it saved: stay where the dismissal went, with the new event listed.
    if (!current) {
      reload();

      return;
    }

    void navigate({
      to: "/r/$roomId/events/$eventId",
      params: { roomId, eventId: detail.event.id },
      replace: true,
    });
  };

  const tabs =
    list === null ? null : (
      <Tabs
        id={tabsId}
        panelId={panelId}
        label="Events"
        value={section}
        onValueChange={(value) => setSection(SECTIONS.find((each) => each === value) ?? "upcoming")}
        items={[
          { value: "upcoming", label: `Upcoming ${list.upcoming.length}` },
          { value: "past", label: `Past ${list.past.length}` },
          { value: "cancelled", label: `Cancelled ${list.cancelled.length}` },
        ]}
      />
    );

  const rows = list === null ? [] : sectionRows(list, section);

  return (
    <PageFrame
      title="Events"
      icon="calendar"
      meta={list === null ? null : <span className="ev-page-room">{list.roomName}</span>}
      tools={
        list?.mayCreate === true ? (
          <Link
            to="/r/$roomId/events/new"
            params={{ roomId }}
            state={overPageState()}
            className="button"
            data-variant="primary"
            data-size="sm"
          >
            <Icon name="plus" size={14} />
            New event
          </Link>
        ) : null
      }
      toolbar={tabs ?? undefined}
    >
      <div className="ev-scroll">
        <Link to="/r/$roomId" params={{ roomId }} className="ev-crumb">
          <Icon name="chevron-left" size={14} />
          Back to {list?.roomName ?? "the conversation"}
        </Link>
        {list === null ? (
          <ListPending state={state} onRetry={reload} />
        ) : (
          <div
            key={section}
            id={panelId}
            role="tabpanel"
            aria-labelledby={tabId(tabsId, section)}
            className="ev-panel enter-fade"
          >
            {rows.length === 0 ? (
              <PaneEmpty icon="calendar" title={EMPTY[section].title} text={EMPTY[section].text} />
            ) : (
              <ul className="ev-list">
                {rows.map((event) => (
                  <EventRow key={event.id} event={event} now={now} />
                ))}
              </ul>
            )}
          </div>
        )}
      </div>
      <EventFormDialog
        roomId={roomId}
        eventId={null}
        open={creating}
        search={search}
        onClose={closeForm}
        onSaved={scheduled}
      />
    </PageFrame>
  );
}
