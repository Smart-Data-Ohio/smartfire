import { type ReactNode, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { EventDetail } from "../../gen/EventDetail.ts";
import type { EventForm } from "../../gen/EventForm.ts";
import type { EventScope } from "../../gen/EventScope.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { Dialog, focusOnOpen } from "../../ui/dialog.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { TextField } from "../../ui/text-field.tsx";
import {
  browserZone,
  createBody,
  type EventDraft,
  type EventField,
  initialDraft,
  isDirty,
  newEventPrefill,
  showsRepeat,
  splitErrors,
  updateBody,
  venueGroups,
} from "./event-form-model.ts";
import { ScopeChoice } from "./scope-choice.tsx";
import type { Landing } from "./use-load.ts";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly form: EventForm };

interface EventFormDialogProps {
  readonly roomId: number;
  /** The event to edit; `null` schedules a new one. */
  readonly eventId: number | null;
  readonly open: boolean;
  /**
   * A new event's URL query: a prefilled link (the `/event` command's) fills the form from its
   * `event[…]` values.
   */
  readonly search?: string;
  readonly onClose: () => void;
  /**
   * Takes the screen's turn for a save as it starts (see `useLoad`'s `begin`), so an answer or a
   * save made while it's on its way wins over its reply. Its landing comes back with `onSaved`;
   * a failed save rereads the screen through it.
   */
  readonly begin?: () => Landing<EventDetail>;
  /**
   * The server's facts for the event it saved (the first one of a new series). `current` is false
   * when the viewer closed the dialog (or opened it again) before the save finished: the save
   * stands, but the dialog's owner mustn't close or navigate for it a second time. `landing` is
   * the turn `begin` took when the save started, if given.
   */
  readonly onSaved: (
    detail: EventDetail,
    current: boolean,
    landing: Landing<EventDetail> | null,
  ) => void;
}

/** A save on its way: what becomes of its reply, or of its failure. */
interface Saving {
  readonly saved: (detail: EventDetail) => void;
  readonly failed: () => void;
}

/** Reads the form each time the dialog opens: someone may have changed the event since. */
function useEventForm(roomId: number, eventId: number | null, open: boolean, search: string) {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [tries, setTries] = useState(0);

  // biome-ignore lint/correctness/useExhaustiveDependencies: a retry (`tries`) reads the form again
  useEffect(() => {
    if (!open) {
      return;
    }

    let live = true;

    setLoad({ status: "loading" });

    const read =
      eventId === null
        ? actions.events.newForm(roomId, newEventPrefill(search, browserZone()))
        : actions.events.editForm(roomId, eventId);

    read.then(
      (form) => {
        if (live) setLoad({ status: "ready", form });
      },
      (failure: Error) => {
        if (live) setLoad({ status: "error", message: failure.message });
      },
    );

    return () => {
      live = false;
    };
  }, [roomId, eventId, open, search, tries]);

  return { load, retry: () => setTries((count) => count + 1) };
}

/**
 * Schedule an event, or edit one: what, when (in the zone it's scheduled in), where (a voice
 * channel or stage, and/or a Google Meet link), whether it repeats, and for a series whether a
 * change covers this event or this and the following ones. The classic new and edit pages.
 */
export function EventFormDialog({
  roomId,
  eventId,
  open,
  search = "",
  onClose,
  begin,
  onSaved,
}: EventFormDialogProps) {
  const formId = useId();
  const { load, retry } = useEventForm(roomId, eventId, open, search);
  const form = load.status === "ready" ? load.form : null;
  const editing = eventId !== null;
  const [busy, setBusy] = useState(false);
  const [dirty, setDirty] = useState(false);
  const touched = useTouchedWhile(open && form === null);
  const opening = useOpening(open ? `${roomId}/${eventId ?? "new"}` : null);

  // The opening a save belongs to: closing, the next opening (or another event) and unmounting
  // all end it, so a save that finishes after the viewer dismissed the dialog (Escape, Cancel,
  // Back) neither closes it again nor navigates.
  const active = useRef<string | null>(null);

  useEffect(() => {
    active.current = opening;

    return () => {
      active.current = null;
    };
  }, [opening]);

  const close = () => {
    active.current = null;
    onClose();
  };

  // A save takes its turn as it starts, not when it finishes: a reply that comes back after a
  // newer answer or save must lose to it.
  const start = (submittedIn: string | null) => (): Saving => {
    const landing = begin?.() ?? null;

    return {
      saved: (detail) =>
        onSaved(detail, submittedIn !== null && active.current === submittedIn, landing),
      failed: () => landing?.reread(),
    };
  };

  const footer = (
    <>
      {dirty ? (
        <span className="ev-form-dirty enter-fade" aria-live="polite">
          Unsaved changes
        </span>
      ) : null}
      <Button variant="secondary" onClick={close}>
        Cancel
      </Button>
      <Button
        type="submit"
        form={formId}
        variant="primary"
        disabled={form === null}
        loading={busy}
        loadingLabel={editing ? "Saving…" : "Scheduling…"}
      >
        {editing ? "Save changes" : "Schedule event"}
      </Button>
    </>
  );

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) close();
      }}
      title={editing ? "Edit event" : "Schedule an event"}
      description={
        form === null ? undefined : (
          <span className="ev-form-where">
            {editing ? form.values.title : `in ${form.roomName}`}
          </span>
        )
      }
      size="md"
      footer={footer}
      dirty={dirty}
    >
      {form === null ? (
        <FormPending load={load} onRetry={retry} />
      ) : (
        <EventFormBody
          key={`${form.eventId ?? "new"}`}
          formId={formId}
          form={form}
          onBusy={setBusy}
          onDirty={setDirty}
          onSubmit={start(opening)}
          touched={touched}
        />
      )}
    </Dialog>
  );
}

/**
 * Names each opening of the dialog on `target` ("room/event", or `null` while closed): a new name
 * for every opening, and for another event while open.
 */
function useOpening(target: string | null): string | null {
  const [shown, setShown] = useState<string | null>(null);
  const [count, setCount] = useState(0);

  if (target !== shown) {
    setShown(target);

    if (target !== null) {
      setCount((held) => held + 1);
    }
  }

  return target === null ? null : `${target}#${count}`;
}

/** Whether the viewer clicked or typed while `active` (the form loading) held. */
function useTouchedWhile(active: boolean) {
  const touched = useRef(false);

  useEffect(() => {
    if (!active) {
      return;
    }

    touched.current = false;

    const mark = () => {
      touched.current = true;
    };

    document.addEventListener("pointerdown", mark, true);
    document.addEventListener("keydown", mark, true);

    return () => {
      document.removeEventListener("pointerdown", mark, true);
      document.removeEventListener("keydown", mark, true);
    };
  }, [active]);

  return touched;
}

function FormPending({ load, onRetry }: { readonly load: Load; readonly onRetry: () => void }) {
  if (load.status === "error") {
    return (
      <div className="ev-form-error" role="alert">
        <p>Couldn't load the form: {load.message}</p>
        <Button variant="secondary" size="sm" icon="refresh-cw" onClick={onRetry}>
          Try again
        </Button>
      </div>
    );
  }

  return (
    <div className="ev-form-skeleton" role="status" aria-busy="true" aria-label="Loading the form">
      <Skeleton width="30%" height={12} />
      <Skeleton width="100%" height={36} radius="md" />
      <Skeleton width="30%" height={12} />
      <Skeleton width="100%" height={72} radius="md" />
      <Skeleton width="60%" height={36} radius="md" />
    </div>
  );
}

interface EventFormBodyProps {
  readonly formId: string;
  readonly form: EventForm;
  readonly onBusy: (busy: boolean) => void;
  readonly onDirty: (dirty: boolean) => void;
  /** A save starts: returns what to do with its reply. */
  readonly onSubmit: () => Saving;
  /** Whether the viewer clicked or typed while the form loaded. */
  readonly touched: { readonly current: boolean };
}

/** What a `Field`'s control carries: its id, and its error or hint as its description. */
export interface FieldControl {
  readonly id: string;
  readonly "aria-invalid": true | undefined;
  readonly "aria-describedby": string | undefined;
}

/**
 * A labelled field the form lays out itself (a select, a text area). `children` renders the
 * control from the attributes that tie the error (or hint) below to it, as `TextField` does.
 */
export function Field({
  id,
  label,
  hint,
  error,
  children,
}: {
  readonly id: string;
  readonly label: string;
  readonly hint?: string | undefined;
  readonly error?: string | undefined;
  readonly children: (control: FieldControl) => ReactNode;
}) {
  const control: FieldControl = {
    id,
    "aria-invalid": error === undefined ? undefined : true,
    "aria-describedby":
      error === undefined ? (hint === undefined ? undefined : `${id}-hint`) : `${id}-error`,
  };

  return (
    <div className={`field${error === undefined ? "" : " is-error"}`}>
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      {children(control)}
      {error === undefined ? (
        hint === undefined ? null : (
          <p id={`${id}-hint`} className="field-hint">
            {hint}
          </p>
        )
      ) : (
        <p id={`${id}-error`} className="field-error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

function EventFormBody({ formId, form, onBusy, onDirty, onSubmit, touched }: EventFormBodyProps) {
  const ids = {
    description: useId(),
    venue: useId(),
    rule: useId(),
  };

  const editing = form.eventId !== null;
  const [initial] = useState(() => initialDraft(form, new Date()));
  const [draft, setDraft] = useState<EventDraft>(initial);
  const [errors, setErrors] = useState<Partial<Record<EventField, string>>>({});
  const [summary, setSummary] = useState<readonly string[]>([]);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);
  const titleRef = useRef<HTMLInputElement | null>(null);
  // A new event is scheduled in the browser's zone; an edit keeps the zone it was scheduled in.
  const timeZone = editing ? form.values.timeZone : browserZone();
  const repeat = showsRepeat(form);
  const repeats = draft.recurrenceRule !== "";
  const groups = venueGroups(form.venues);

  const keptVenue =
    draft.venueRoomId !== "" && !form.venues.some((v) => `${v.roomId}` === draft.venueRoomId);

  useEffect(() => onDirty(isDirty(initial, draft)), [initial, draft, onDirty]);

  useEffect(() => () => onDirty(false), [onDirty]);

  // The dialog took focus (its Close button) while the form loaded; the title gets it now, unless
  // the viewer has already moved on to something that's still there.
  // biome-ignore lint/correctness/useExhaustiveDependencies: once, when the loaded form mounts
  useLayoutEffect(() => {
    const input = titleRef.current;
    const dialog = input?.closest("dialog") ?? null;
    const active = document.activeElement;

    const lost =
      active === null ||
      active === document.body ||
      active === dialog ||
      (dialog !== null && !dialog.contains(active));

    if (input !== null && (lost || !touched.current)) {
      focusOnOpen(input);
    }
  }, []);

  const edit = (patch: Partial<EventDraft>) => {
    setDraft((current) => ({ ...current, ...patch }));

    const touched = Object.keys(patch);

    setErrors((current) =>
      Object.fromEntries(Object.entries(current).filter(([key]) => !touched.includes(key))),
    );
  };

  const fail = (failure: Error) => {
    setBusy(false);
    onBusy(false);

    if (failure instanceof ActionError && failure.tag === "Validation") {
      const split = splitErrors(failure.fields);

      setErrors(split.byField);
      setSummary(
        split.other.length > 0 || Object.keys(split.byField).length > 0
          ? split.other
          : [failure.message],
      );
      setAttempt((count) => count + 1);

      return;
    }

    setSummary([failure.message]);
  };

  const submit = () => {
    if (busy) {
      return;
    }

    if (draft.title.trim() === "") {
      setErrors({ title: "Give the event a title." });
      setAttempt((count) => count + 1);
      titleRef.current?.focus();

      return;
    }

    setBusy(true);
    onBusy(true);
    setSummary([]);

    const saving = onSubmit();

    const saved =
      form.eventId === null
        ? actions.events.create(form.roomId, createBody(draft, timeZone))
        : actions.events.update(form.roomId, form.eventId, updateBody(form, draft));

    saved.then(
      (detail) => {
        setBusy(false);
        onBusy(false);
        saving.saved(detail);
      },
      (failure: Error) => {
        saving.failed();
        fail(failure);
      },
    );
  };

  return (
    <form
      id={formId}
      className="ev-form"
      noValidate
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      {summary.length === 0 ? null : (
        <div className="ev-form-summary" role="alert">
          <Icon name="circle-alert" size={16} />
          <ul>
            {summary.map((message) => (
              <li key={message}>{message}</li>
            ))}
          </ul>
        </div>
      )}
      <TextField
        ref={titleRef}
        label="Title"
        value={draft.title}
        placeholder="Team check-in"
        autoComplete="off"
        maxLength={form.limits.titleMaxLength}
        required
        data-autofocus
        error={errors.title}
        attempt={attempt}
        onChange={(event) => edit({ title: event.target.value })}
      />
      <div className="ev-form-times">
        <TextField
          label="Starts"
          type="datetime-local"
          value={draft.startsAt}
          required
          error={errors.startsAt}
          attempt={attempt}
          onChange={(event) => edit({ startsAt: event.target.value })}
        />
        <TextField
          label="Ends (optional)"
          type="datetime-local"
          value={draft.endsAt}
          min={draft.startsAt === "" ? undefined : draft.startsAt}
          error={errors.endsAt}
          attempt={attempt}
          onChange={(event) => edit({ endsAt: event.target.value })}
        />
      </div>
      <p className="ev-form-zone">
        <Icon name="globe" size={14} />
        <span>
          {editing ? (
            <>
              Times are in <strong>{timeZone}</strong>, the zone this event was scheduled in.
            </>
          ) : (
            <>
              Times are in your time zone, <strong>{timeZone}</strong>.
            </>
          )}{" "}
          Attendees see them in their own.
        </span>
      </p>
      {errors.timeZone === undefined ? null : (
        <p className="field-error" role="alert">
          {errors.timeZone}
        </p>
      )}
      {form.series && editing ? (
        <ScopeChoice
          legend="Apply changes to"
          value={draft.scope}
          options={form.scopeOptions}
          onChange={(scope: EventScope) => edit({ scope })}
        />
      ) : null}
      {repeat ? (
        <div className="ev-form-repeat">
          <Field id={ids.rule} label="Repeats" error={errors.recurrenceRule}>
            {(control) => (
              <select
                {...control}
                className="input ev-select"
                value={draft.recurrenceRule}
                onChange={(event) => edit({ recurrenceRule: event.target.value })}
              >
                {form.repeatOptions.map((option) => (
                  <option key={option.value ?? "none"} value={option.value ?? ""}>
                    {option.label}
                  </option>
                ))}
              </select>
            )}
          </Field>
          {repeats ? (
            <TextField
              label="Until"
              type="date"
              value={draft.recurrenceUntil}
              min={draft.startsAt.slice(0, 10) || undefined}
              required
              error={errors.recurrenceUntil}
              attempt={attempt}
              onChange={(event) => edit({ recurrenceUntil: event.target.value })}
            />
          ) : null}
        </div>
      ) : null}
      {repeat && repeats ? (
        <p className="field-hint ev-form-hint">
          {editing
            ? "Changing how this series repeats rebuilds its future occurrences, keeping any where someone answered differently. It applies to this and the following events."
            : `One occurrence per repeat, up to ${form.limits.maxOccurrences}. Members get a single invitation for the whole series.`}
        </p>
      ) : null}
      <Field id={ids.venue} label="Where" error={errors.venueRoomId}>
        {(control) => (
          <select
            {...control}
            className="input ev-select"
            value={draft.venueRoomId}
            onChange={(event) => edit({ venueRoomId: event.target.value })}
          >
            <option value="">No channel</option>
            {keptVenue ? <option value={draft.venueRoomId}>The current channel</option> : null}
            {groups.map((group) => (
              <optgroup key={group.label} label={group.label}>
                {group.options.map((venue) => (
                  <option key={venue.roomId} value={`${venue.roomId}`}>
                    {venue.name}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
        )}
      </Field>
      {form.values.meetLink === null ? (
        form.meetAvailable ? (
          <div className="ev-form-meet">
            <Checkbox
              checked={draft.meetLinkRequested}
              onCheckedChange={(meetLinkRequested) => edit({ meetLinkRequested })}
              label="Add a Google Meet link"
            />
            <p className="field-hint">
              Needs the organizer's connected Google account. The link appears on the event once
              Google creates it.
            </p>
          </div>
        ) : null
      ) : (
        <p className="ev-form-meet-link">
          <Icon name="video" size={14} />
          <a href={form.values.meetLink} target="_blank" rel="noopener noreferrer">
            {form.values.meetLink}
          </a>
        </p>
      )}
      <Field id={ids.description} label="Description (optional)" error={errors.description}>
        {(control) => (
          <textarea
            {...control}
            className="input ev-description-input"
            value={draft.description}
            rows={4}
            placeholder="Agenda, links, what to bring"
            onChange={(event) => edit({ description: event.target.value })}
            onKeyDown={(event) => {
              if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
                event.currentTarget.form?.requestSubmit();
              }
            }}
          />
        )}
      </Field>
    </form>
  );
}
