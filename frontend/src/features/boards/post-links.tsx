import { useMatchRoute, useNavigate } from "@tanstack/react-router";
import { type FormEvent, useEffect, useId, useRef, useState } from "react";
import type { CreateWorkLink } from "../../gen/CreateWorkLink.ts";
import type { WorkLink } from "../../gen/WorkLink.ts";
import type { WorkLinkEventCandidate } from "../../gen/WorkLinkEventCandidate.ts";
import type { WorkLinkKind } from "../../gen/WorkLinkKind.ts";
import { parseBoardSearch } from "../../lib/board-search.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { Tabs } from "../../ui/tabs.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";

const LINK_ICON = {
  pull_request: "git-pull-request",
  event: "calendar",
  drive_file: "file-text",
} as const satisfies Record<WorkLinkKind, IconName>;

const KINDS = [
  { value: "pull_request", label: "Pull request", icon: "git-pull-request" },
  { value: "event", label: "Event", icon: "calendar" },
  { value: "drive_file", label: "Drive file", icon: "file-text" },
] as const satisfies readonly { value: WorkLinkKind; label: string; icon: IconName }[];

/** Each kind's input, as the server names it in a `Validation` error's fields. */
const FIELD = {
  pull_request: "pullRequestUrl",
  event: "eventId",
  drive_file: "driveUrl",
} as const satisfies Record<WorkLinkKind, keyof CreateWorkLink>;

/** The classic forms' submit buttons. */
const SUBMIT = {
  pull_request: "Link pull request",
  event: "Link event",
  drive_file: "Link Drive file",
} as const satisfies Record<WorkLinkKind, string>;

function isKind(value: string): value is WorkLinkKind {
  return KINDS.some((kind) => kind.value === value);
}

/** A link goes into an `href` only when it's `https://` or a site path, as the contract says. */
function linkHref(url: string): string | null {
  return url.startsWith("https://") || (url.startsWith("/") && !url.startsWith("//")) ? url : null;
}

/** An event's start in its own time zone (the classic picker's), or the browser's if unknown. */
export function eventStart(startsAt: string, timeZone: string | null): string {
  const options: Intl.DateTimeFormatOptions = { dateStyle: "medium", timeStyle: "short" };

  try {
    return new Intl.DateTimeFormat(
      undefined,
      timeZone === null ? options : { ...options, timeZone },
    ).format(new Date(startsAt));
  } catch {
    return new Intl.DateTimeFormat(undefined, options).format(new Date(startsAt));
  }
}

/** Why a link was refused: under the field it names, or for the form as a whole. */
interface LinkProblem {
  readonly field?: string;
  readonly form?: string;
}

const NOT_LINKED = "Couldn't link that.";

/** A refused link's message for the field it names, else for the form as a whole. */
function refusal(error: Error, kind: WorkLinkKind): LinkProblem {
  if (!(error instanceof ActionError)) {
    return { form: error.message === "" ? NOT_LINKED : error.message } satisfies LinkProblem;
  }

  const own = error.fields[FIELD[kind]]?.[0];

  if (own !== undefined) {
    return { field: own } satisfies LinkProblem;
  }

  if (error.tag === "NotFound" && kind === "event") {
    return { field: "That event isn't in this room anymore." } satisfies LinkProblem;
  }

  return { form: error.message === "" ? NOT_LINKED : error.message } satisfies LinkProblem;
}

function LinkRow({
  link,
  removable,
  busy,
  onRemove,
}: {
  readonly link: WorkLink;
  readonly removable: boolean;
  readonly busy: boolean;
  readonly onRemove: () => void;
}) {
  const href = linkHref(link.url);
  const icon: IconName = LINK_ICON[link.kind] ?? "link";

  const detail =
    link.pullRequestState === null
      ? link.eventCancelled
        ? "Cancelled"
        : null
      : link.pullRequestState.replace("_", " ");

  return (
    <li className="post-link" data-kind={link.kind}>
      <Icon name={icon} size={14} />
      {href === null ? (
        <span className="post-link-label">{link.label}</span>
      ) : (
        <a
          className="post-link-label"
          href={href}
          {...(href.startsWith("https://") ? { target: "_blank", rel: "noreferrer" } : {})}
        >
          {link.label}
        </a>
      )}
      {link.title === null ? null : <span className="post-link-title">{link.title}</span>}
      {link.eventStartsAt === null ? null : (
        <span className="post-link-title">
          {eventStart(link.eventStartsAt, link.eventTimeZone)}
        </span>
      )}
      {detail === null ? null : (
        <span className="post-link-state" data-state={link.pullRequestState ?? "cancelled"}>
          {detail}
        </span>
      )}
      {removable ? (
        <IconButton
          className="post-link-remove"
          icon="x"
          size="sm"
          label={`Remove link ${link.label}`}
          disabled={busy}
          onClick={onRemove}
        />
      ) : null}
    </li>
  );
}

interface AddLinkProps {
  readonly threadId: number;
  readonly busy: boolean;
  readonly onSubmit: (input: CreateWorkLink, kind: WorkLinkKind) => Promise<void>;
  readonly onCancel: () => void;
}

/** The candidates the event picker offers, fetched each time the form opens. */
type Events =
  | { readonly status: "loading" }
  | { readonly status: "ready"; readonly events: readonly WorkLinkEventCandidate[] }
  | { readonly status: "error" };

/**
 * The classic "Link" forms as one: pick the kind, then a pull request URL, one of the room's
 * upcoming events, or a Drive file URL.
 */
function AddLink({ threadId, busy, onSubmit, onCancel }: AddLinkProps) {
  const [kind, setKind] = useState<WorkLinkKind>("pull_request");
  const [pullRequestUrl, setPullRequestUrl] = useState("");
  const [driveUrl, setDriveUrl] = useState("");
  const [eventId, setEventId] = useState("");
  const [events, setEvents] = useState<Events>({ status: "loading" });
  const [problem, setProblem] = useState<LinkProblem>({});
  const [attempt, setAttempt] = useState(0);
  const tabsId = useId();
  const selectId = useId();
  const formRef = useRef<HTMLFormElement | null>(null);

  useEffect(() => {
    let current = true;

    void actions.work.linkForm(threadId).then(
      (form) => {
        if (current) {
          setEvents({ status: "ready", events: form.events });
          setEventId((chosen) => chosen || `${form.events[0]?.id ?? ""}`);
        }
      },
      () => {
        if (current) {
          setEvents({ status: "error" });
        }
      },
    );

    return () => {
      current = false;
    };
  }, [threadId]);

  useEffect(() => {
    formRef.current?.querySelector<HTMLElement>("input, select")?.focus();
  }, []);

  const choose = (value: string) => {
    if (isKind(value)) {
      setKind(value);
      setProblem({});
    }
  };

  const input = (): CreateWorkLink => {
    switch (kind) {
      case "pull_request":
        return { kind, pullRequestUrl };
      case "event":
        return { kind, eventId: eventId === "" ? null : Number(eventId) };
      case "drive_file":
        return { kind, driveUrl };
    }
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();

    if (busy) {
      return;
    }

    setProblem({});
    void onSubmit(input(), kind).catch((error: Error) => {
      setProblem(refusal(error, kind));
      setAttempt((count) => count + 1);
    });
  };

  const noEvents = kind === "event" && events.status === "ready" && events.events.length === 0;

  return (
    <form ref={formRef} className="post-link-add" aria-label="Link to this work" onSubmit={submit}>
      <Tabs id={tabsId} items={KINDS} value={kind} label="What to link" onValueChange={choose}>
        <div className="post-link-add-field">
          {kind === "pull_request" ? (
            <TextField
              label="Pull request URL"
              type="url"
              autoComplete="off"
              placeholder="https://github.com/owner/repo/pull/123"
              value={pullRequestUrl}
              disabled={busy}
              error={problem.field}
              attempt={attempt}
              onChange={(event) => setPullRequestUrl(event.target.value)}
            />
          ) : null}
          {kind === "drive_file" ? (
            <TextField
              label="Drive file URL"
              type="url"
              autoComplete="off"
              placeholder="https://drive.google.com/file/d/…"
              value={driveUrl}
              disabled={busy}
              error={problem.field}
              attempt={attempt}
              onChange={(event) => setDriveUrl(event.target.value)}
            />
          ) : null}
          {kind === "event" ? (
            <div className={`field t-input-wrap${problem.field === undefined ? "" : " is-error"}`}>
              <label className="field-label" htmlFor={selectId}>
                Event
              </label>
              {events.status === "loading" ? (
                <p className="post-link-add-note" role="status">
                  Loading events…
                </p>
              ) : null}
              {events.status === "error" ? (
                <p className="post-link-add-note">Couldn't load this room's events.</p>
              ) : null}
              {noEvents ? (
                <p className="post-link-add-note">No upcoming events in this room.</p>
              ) : null}
              {events.status === "ready" && events.events.length > 0 ? (
                <select
                  id={selectId}
                  className="input"
                  value={eventId}
                  disabled={busy}
                  aria-invalid={problem.field === undefined ? undefined : true}
                  aria-describedby={`${selectId}-error`}
                  onChange={(event) => setEventId(event.target.value)}
                >
                  {events.events.map((candidate) => (
                    <option key={candidate.id} value={candidate.id}>
                      {candidate.title} · {eventStart(candidate.startsAt, candidate.timeZone)}
                    </option>
                  ))}
                </select>
              ) : null}
              <p id={`${selectId}-error`} className="field-error t-error-msg" aria-live="polite">
                {problem.field ?? ""}
              </p>
            </div>
          ) : null}
        </div>
      </Tabs>
      {problem.form === undefined ? null : (
        <p className="post-link-add-error" role="alert">
          {problem.form}
        </p>
      )}
      <div className="post-link-add-actions">
        <Button variant="ghost" size="sm" type="button" disabled={busy} onClick={onCancel}>
          Cancel
        </Button>
        <Button
          variant="primary"
          size="sm"
          type="submit"
          loading={busy}
          disabled={busy || (kind === "event" && (events.status !== "ready" || noEvents))}
        >
          {SUBMIT[kind]}
        </Button>
      </div>
    </form>
  );
}

/**
 * What the work links to (classic `work_threads/links`): pull requests, the room's events and
 * Drive files. Whoever can see the work may add and remove them, one change at a time. The form
 * is open at `/r/:roomId/t/:threadId/links`, where the classic links page's URL lands.
 */
export function PostLinks({
  threadId,
  roomId,
  links,
  editable,
}: {
  readonly threadId: number;
  readonly roomId: number;
  readonly links: readonly WorkLink[];
  readonly editable: boolean;
}) {
  const navigate = useNavigate();
  const adding = useMatchRoute()({ to: "/r/$roomId/t/$threadId/links" }) !== false && editable;
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  const headingId = useId();
  const linkButtonRef = useRef<HTMLButtonElement | null>(null);

  const setAdding = (open: boolean) => {
    void navigate({
      to: open ? "/r/$roomId/t/$threadId/links" : "/r/$roomId/t/$threadId",
      params: { roomId, threadId },
      search: parseBoardSearch,
      replace: true,
    });
  };

  /** Runs one change; another can't start until it answers. */
  const run = async <A,>(change: () => Promise<A>): Promise<A | undefined> => {
    if (busyRef.current) {
      return undefined;
    }

    busyRef.current = true;
    setBusy(true);

    try {
      return await change();
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  };

  const add = async (input: CreateWorkLink) => {
    const saved = await run(() => actions.work.addLink(threadId, input));

    if (saved !== undefined) {
      setAdding(false);
      linkButtonRef.current?.focus();
    }
  };

  const remove = (link: WorkLink) => {
    void run(() => actions.work.removeLink(threadId, link.id)).catch((error: Error) => {
      const title = `Couldn't remove ${link.label}`;
      const description = error.message;

      toast(
        description === "" ? { title, tone: "danger" } : { title, description, tone: "danger" },
      );
    });
  };

  if (!editable && links.length === 0) {
    return null;
  }

  return (
    <section className="post-links" aria-labelledby={headingId}>
      <header className="post-section-head">
        <h3 id={headingId} className="post-section-title">
          <Icon name="link" size={14} />
          Linked
        </h3>
        {editable && !adding ? (
          <Button
            ref={linkButtonRef}
            variant="ghost"
            size="sm"
            icon="plus"
            disabled={busy}
            onClick={() => setAdding(true)}
          >
            Link
          </Button>
        ) : null}
      </header>
      {links.length === 0 ? (
        <p className="post-result-none">Nothing linked yet.</p>
      ) : (
        <ul className="post-link-list">
          {links.map((link) => (
            <LinkRow
              key={link.id}
              link={link}
              removable={editable}
              busy={busy}
              onRemove={() => remove(link)}
            />
          ))}
        </ul>
      )}
      {adding ? (
        <AddLink
          threadId={threadId}
          busy={busy}
          onSubmit={add}
          onCancel={() => {
            setAdding(false);
          }}
        />
      ) : null}
    </section>
  );
}
