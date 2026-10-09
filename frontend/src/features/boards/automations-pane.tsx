import { useCallback, useEffect, useId, useRef, useState } from "react";
import type { BoardAutomations } from "../../gen/BoardAutomations.ts";
import type { BoardSlaTimerInput } from "../../gen/BoardSlaTimerInput.ts";
import type { BoardTagRule } from "../../gen/BoardTagRule.ts";
import type { UpdateBoardSlaTimers } from "../../gen/UpdateBoardSlaTimers.ts";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { useStore } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneFrame, RoomName } from "../panes/pane-frame.tsx";
import { PaneEmpty, PaneError } from "../panes/pane-states.tsx";
import { isAgent } from "../people/people.ts";
import { MAX_SLA_MINUTES, MAX_TAG_LENGTH, minutesLabel } from "./board-format.ts";
import { StatusChip, UserFace } from "./board-parts.tsx";

type Fields = Readonly<Record<string, readonly string[]>>;

const NO_FIELDS: Fields = {};

type Load =
  | { readonly status: "loading" }
  | {
      readonly status: "error";
      readonly message: string;
      /** A membership or administrator refusal, which retrying will not change. */
      readonly refusal: "forbidden" | "unavailable" | null;
    }
  | { readonly status: "ready"; readonly settings: BoardAutomations };

/** Classic answers a non-admin member with 403 and a nonmember with a redirect home. */
function refusalOf(error: Error): "forbidden" | "unavailable" | null {
  if (!(error instanceof ActionError)) return null;

  if (error.tag === "Forbidden") return "forbidden";

  if (error.tag === "NotFound") return "unavailable";

  return null;
}

/** The statuses that take a timer, as the classic form lists them. Done posts never breach. */
const TIMED = [
  { status: "planned", field: "planned", label: "Planned" },
  { status: "in_progress", field: "inProgress", label: "In progress" },
  { status: "blocked", field: "blocked", label: "Blocked" },
] as const satisfies readonly {
  readonly status: WorkStatus;
  readonly field: keyof UpdateBoardSlaTimers;
  readonly label: string;
}[];

type TimerField = (typeof TIMED)[number]["field"];

interface TimerDraft {
  readonly nudge: string;
  readonly escalate: string;
}

type TimerDrafts = Readonly<Record<TimerField, TimerDraft>>;

/**
 * The pane's one-at-a-time automation changes. `run` starts a change and shows its answer, or
 * answers `null` while another is pending (every control is disabled then, but Enter in a field
 * can still submit).
 */
interface Mutations {
  readonly pending: boolean;
  readonly run: (change: () => Promise<BoardAutomations>) => Promise<BoardAutomations> | null;
}

function timerDrafts(settings: BoardAutomations): TimerDrafts {
  const draft = (status: WorkStatus): TimerDraft => {
    const timer = settings.slaTimers.find((candidate) => candidate.status === status);

    return timer === undefined
      ? { nudge: "", escalate: "" }
      : { nudge: String(timer.nudgeAfterMinutes), escalate: String(timer.escalateAfterMinutes) };
  };

  return { planned: draft("planned"), inProgress: draft("in_progress"), blocked: draft("blocked") };
}

/**
 * The drafts once `fresh` settings arrive: a field still as last loaded takes the fresh value; one
 * the person has edited keeps their text.
 */
function reconcile(drafts: TimerDrafts, loaded: TimerDrafts, fresh: TimerDrafts): TimerDrafts {
  const field = (name: TimerField): TimerDraft => ({
    nudge: drafts[name].nudge === loaded[name].nudge ? fresh[name].nudge : drafts[name].nudge,
    escalate:
      drafts[name].escalate === loaded[name].escalate
        ? fresh[name].escalate
        : drafts[name].escalate,
  });

  return { planned: field("planned"), inProgress: field("inProgress"), blocked: field("blocked") };
}

function edited(drafts: TimerDrafts, loaded: TimerDrafts, field: TimerField): boolean {
  return (
    drafts[field].nudge.trim() !== loaded[field].nudge ||
    drafts[field].escalate.trim() !== loaded[field].escalate
  );
}

/** A minutes field's value: blank is `null`, whole minutes are numbers, anything else isn't. */
function minutesValue(text: string): number | null | "invalid" {
  const trimmed = text.trim();

  if (trimmed === "") {
    return null;
  }

  return /^\d+$/.test(trimmed) ? Number(trimmed) : "invalid";
}

function messageOf(fields: Fields, field: string): string | undefined {
  const messages = fields[field] ?? [];

  return messages.length === 0 ? undefined : messages.join(" ");
}

/** One rule: "#bug → Maya", with the remove button. */
function TagRuleRow({
  rule,
  removing,
  disabled,
  onRemove,
}: {
  readonly rule: BoardTagRule;
  readonly removing: boolean;
  readonly disabled: boolean;
  readonly onRemove: () => void;
}) {
  const assignee = useStore((state) => state.users[rule.assigneeId]);

  return (
    <li className="automation-rule enter-fade" data-removing={removing || undefined}>
      <span className="board-tag">{rule.tag}</span>
      <span className="automation-rule-arrow" aria-hidden="true">
        →
      </span>
      <span className="automation-rule-assignee">
        {assignee === undefined ? null : <UserFace user={assignee} size={20} />}
        <span className="automation-rule-name">{assignee?.name ?? "Former member"}</span>
        {assignee !== undefined && isAgent(assignee) ? (
          <span className="message-agent-tag">agent</span>
        ) : null}
      </span>
      <IconButton
        icon="trash"
        label={`Remove auto-assign rule for ${rule.tag}`}
        className="automation-rule-remove"
        disabled={disabled}
        onClick={onRemove}
      />
    </li>
  );
}

/** "Assign to": the board's active members, people then agents, after "Choose a member". */
function AssigneeSelect({
  candidates,
  value,
  error,
  disabled,
  onChange,
}: {
  readonly candidates: readonly number[];
  readonly value: string;
  readonly error: string | undefined;
  readonly disabled: boolean;
  readonly onChange: (value: string) => void;
}) {
  const id = useId();
  const users = useStore((state) => state.users);
  const people = candidates.filter((userId) => !isAgent(users[userId]));
  const agents = candidates.filter((userId) => isAgent(users[userId]));
  const name = (userId: number) => users[userId]?.name ?? "Someone";

  return (
    <div className={`field t-input-wrap${error === undefined ? "" : " is-error"}`}>
      <label className="field-label" htmlFor={id}>
        Assign to
      </label>
      <select
        id={id}
        className="input board-select"
        value={value}
        disabled={disabled}
        aria-invalid={error === undefined ? undefined : true}
        aria-describedby={error === undefined ? undefined : `${id}-error`}
        onChange={(event) => onChange(event.target.value)}
      >
        <option value="">Choose a member</option>
        {people.length > 0 ? (
          <optgroup label="Members">
            {people.map((userId) => (
              <option key={userId} value={userId}>
                {name(userId)}
              </option>
            ))}
          </optgroup>
        ) : null}
        {agents.length > 0 ? (
          <optgroup label="Agents">
            {agents.map((userId) => (
              <option key={userId} value={userId}>
                {name(userId)}
              </option>
            ))}
          </optgroup>
        ) : null}
      </select>
      <p id={`${id}-error`} className="field-error t-error-msg" aria-live="polite">
        {error ?? ""}
      </p>
    </div>
  );
}

/** "Auto-assign by tag": the rules, each removable, and the form that adds one. */
function TagRules({
  roomId,
  settings,
  mutations,
}: {
  readonly roomId: number;
  readonly settings: BoardAutomations;
  readonly mutations: Mutations;
}) {
  const titleId = useId();
  const [tag, setTag] = useState("");
  const [assigneeId, setAssigneeId] = useState("");
  const [fields, setFields] = useState<Fields>(NO_FIELDS);
  const [attempt, setAttempt] = useState(0);
  const [saving, setSaving] = useState(false);
  const [removing, setRemoving] = useState<number | null>(null);

  const add = () => {
    const input = { tag, assigneeId: assigneeId === "" ? null : Number(assigneeId) };
    const request = mutations.run(() => actions.boards.addTagRule(roomId, input));

    if (request === null) {
      return;
    }

    setSaving(true);
    request
      .then(
        () => {
          // The fields were disabled while the rule saved, so these are still what was sent.
          setTag("");
          setAssigneeId("");
          setFields(NO_FIELDS);
          toast({ title: "Auto-assign rule added." });
        },
        (error: Error) => {
          const named = error instanceof ActionError ? error.fields : NO_FIELDS;

          setFields(named);
          setAttempt((count) => count + 1);

          if (Object.keys(named).length === 0) {
            toast({ title: "Couldn't add the rule", description: error.message, tone: "danger" });
          }
        },
      )
      .finally(() => setSaving(false));
  };

  const remove = (rule: BoardTagRule) => {
    const request = mutations.run(() => actions.boards.removeTagRule(roomId, rule.id));

    if (request === null) {
      return;
    }

    setRemoving(rule.id);
    request
      .then(
        () => {
          toast({ title: "Auto-assign rule removed." });
        },
        (error: Error) => {
          toast({ title: "Couldn't remove the rule", description: error.message, tone: "danger" });
        },
      )
      .finally(() => setRemoving(null));
  };

  return (
    <section className="automation-section" aria-labelledby={titleId}>
      <h3 id={titleId} className="post-section-title">
        Auto-assign by tag
      </h3>
      <p className="automation-text">
        When a post gains one of these tags while it has no owner, the post is assigned
        automatically. Rules never override an existing assignment.
      </p>
      {settings.tagRules.length === 0 ? (
        <p className="automation-none">No tag rules yet.</p>
      ) : (
        <ul className="automation-rules" aria-labelledby={titleId}>
          {settings.tagRules.map((rule) => (
            <TagRuleRow
              key={rule.id}
              rule={rule}
              removing={removing === rule.id}
              disabled={mutations.pending}
              onRemove={() => remove(rule)}
            />
          ))}
        </ul>
      )}
      <form
        className="automation-form"
        aria-label="Add an auto-assign rule"
        noValidate
        onSubmit={(event) => {
          event.preventDefault();
          add();
        }}
      >
        <div className="automation-form-row">
          <TextField
            label="Tag"
            placeholder="bug"
            autoComplete="off"
            maxLength={MAX_TAG_LENGTH}
            value={tag}
            disabled={mutations.pending}
            error={messageOf(fields, "tag")}
            attempt={attempt}
            onChange={(event) => setTag(event.target.value)}
          />
          <AssigneeSelect
            candidates={settings.candidates}
            value={assigneeId}
            error={messageOf(fields, "assigneeId")}
            disabled={mutations.pending}
            onChange={setAssigneeId}
          />
        </div>
        <div className="automation-actions">
          <Button
            type="submit"
            variant="primary"
            size="sm"
            icon="plus"
            loading={saving}
            disabled={mutations.pending && !saving}
          >
            Add rule
          </Button>
        </div>
      </form>
    </section>
  );
}

/** One status's row: its chip and the two minutes fields, each with what it reads as. */
function TimerRow({
  label,
  status,
  draft,
  error,
  disabled,
  onChange,
}: {
  readonly label: string;
  readonly status: WorkStatus;
  readonly draft: TimerDraft;
  readonly error: string | undefined;
  readonly disabled: boolean;
  readonly onChange: (draft: TimerDraft) => void;
}) {
  const errorId = useId();
  const described = error === undefined ? undefined : errorId;

  const input = (value: string, name: string, change: (value: string) => void) => {
    const minutes = minutesValue(value);

    return (
      <div className="automation-minutes">
        <input
          type="number"
          inputMode="numeric"
          className={`input automation-minutes-input${error === undefined ? "" : " is-error"}`}
          min={1}
          max={MAX_SLA_MINUTES}
          step={1}
          placeholder="Off"
          value={value}
          disabled={disabled}
          aria-label={`${label} ${name} minutes`}
          aria-describedby={described}
          aria-invalid={error === undefined ? undefined : true}
          onChange={(event) => change(event.target.value)}
        />
        <span className="automation-minutes-hint" aria-hidden="true">
          {minutes !== null && minutes !== "invalid" && minutes > 0 ? minutesLabel(minutes) : " "}
        </span>
      </div>
    );
  };

  return (
    <>
      <tr className="automation-timer" data-invalid={error !== undefined || undefined}>
        <th scope="row" className="automation-timer-status">
          <StatusChip status={status} />
        </th>
        <td>{input(draft.nudge, "nudge", (nudge) => onChange({ ...draft, nudge }))}</td>
        <td>
          {input(draft.escalate, "escalation", (escalate) => onChange({ ...draft, escalate }))}
        </td>
      </tr>
      {error === undefined ? null : (
        <tr className="automation-timer-problem">
          <td />
          <td id={errorId} colSpan={2} className="automation-timer-error t-error-msg">
            {error}
          </td>
        </tr>
      )}
    </>
  );
}

/** Each row's local problem: whole minutes only, as the number field's `step` promises. */
function localTimerProblems(drafts: TimerDrafts): Fields {
  const problems: [string, string[]][] = [];

  for (const { field } of TIMED) {
    const messages: string[] = [];

    if (minutesValue(drafts[field].nudge) === "invalid") {
      messages.push("Nudge after minutes must be a whole number");
    }

    if (minutesValue(drafts[field].escalate) === "invalid") {
      messages.push("Escalate after minutes must be a whole number");
    }

    if (messages.length > 0) {
      problems.push([field, messages]);
    }
  }

  return Object.fromEntries(problems);
}

function timerInput(draft: TimerDraft): BoardSlaTimerInput {
  const nudge = minutesValue(draft.nudge);
  const escalate = minutesValue(draft.escalate);

  return {
    nudgeAfterMinutes: nudge === "invalid" ? null : nudge,
    escalateAfterMinutes: escalate === "invalid" ? null : escalate,
  };
}

/**
 * "SLA timers": nudge and escalation minutes for each unfinished status. Settings that arrive
 * meanwhile (someone else's save, this pane's other changes) fill the fields the person hasn't
 * touched, and a save sends only the rows they changed.
 */
function SlaTimers({
  roomId,
  settings,
  mutations,
}: {
  readonly roomId: number;
  readonly settings: BoardAutomations;
  readonly mutations: Mutations;
}) {
  const titleId = useId();
  const [shown, setShown] = useState(settings);
  const [loaded, setLoaded] = useState(() => timerDrafts(settings));
  const [drafts, setDrafts] = useState(loaded);
  const [fields, setFields] = useState<Fields>(NO_FIELDS);
  const [alert, setAlert] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  if (settings !== shown) {
    const fresh = timerDrafts(settings);

    setShown(settings);
    setLoaded(fresh);
    setDrafts(reconcile(drafts, loaded, fresh));
  }

  const dirty = TIMED.some(({ field }) => edited(drafts, loaded, field));

  const save = () => {
    const local = localTimerProblems(drafts);

    if (Object.keys(local).length > 0) {
      setFields(local);
      setAlert(null);

      return;
    }

    // Only the rows the person changed: a timer saved elsewhere meanwhile stays as it is.
    const input: UpdateBoardSlaTimers = {};

    for (const { field } of TIMED) {
      if (edited(drafts, loaded, field)) {
        input[field] = timerInput(drafts[field]);
      }
    }

    const request = mutations.run(() => actions.boards.saveSlaTimers(roomId, input));

    if (request === null) {
      return;
    }

    setSaving(true);
    request
      .then(
        (next) => {
          // The fields were disabled while the timers saved: nothing typed since is lost.
          setDrafts(timerDrafts(next));
          setFields(NO_FIELDS);
          setAlert(null);
          toast({ title: "SLA timers saved." });
        },
        (error: Error) => {
          if (error instanceof ActionError && error.tag === "Validation") {
            setFields(error.fields);
            setAlert(error.message);

            return;
          }

          toast({ title: "Couldn't save the timers", description: error.message, tone: "danger" });
        },
      )
      .finally(() => setSaving(false));
  };

  return (
    <section className="automation-section" aria-labelledby={titleId}>
      <h3 id={titleId} className="post-section-title">
        SLA timers
      </h3>
      <p className="automation-text">
        A post sitting in a status past the nudge time notifies its owner; past the escalation time
        it escalates to the board's creator. Each fires once per status change. Leave both blank to
        disable a status.
      </p>
      <form
        className="automation-form"
        aria-label="SLA timers"
        noValidate
        onSubmit={(event) => {
          event.preventDefault();
          save();
        }}
      >
        {alert === null ? null : (
          <p className="automation-alert" role="alert">
            {alert}
          </p>
        )}
        <table className="automation-timers" aria-labelledby={titleId}>
          <thead>
            <tr className="automation-timer automation-timer-head">
              <th scope="col">Status</th>
              <th scope="col">Nudge after (minutes)</th>
              <th scope="col">Escalate after (minutes)</th>
            </tr>
          </thead>
          <tbody>
            {TIMED.map(({ status, field, label }) => (
              <TimerRow
                key={status}
                label={label}
                status={status}
                draft={drafts[field]}
                error={messageOf(fields, field)}
                disabled={mutations.pending}
                onChange={(draft) => setDrafts((previous) => ({ ...previous, [field]: draft }))}
              />
            ))}
          </tbody>
        </table>
        <div className="automation-actions">
          {dirty ? (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={mutations.pending}
              onClick={() => {
                setDrafts(loaded);
                setFields(NO_FIELDS);
                setAlert(null);
              }}
            >
              Reset
            </Button>
          ) : null}
          <Button
            type="submit"
            variant="primary"
            size="sm"
            loading={saving}
            disabled={!dirty || (mutations.pending && !saving)}
          >
            Save SLA timers
          </Button>
        </div>
      </form>
    </section>
  );
}

function AutomationsSkeleton() {
  return (
    <div className="automations">
      {[0, 1].map((section) => (
        <div key={section} className="automation-section">
          <Skeleton width={120} height={12} />
          <Skeleton width="90%" height={12} />
          <Skeleton width="70%" height={12} />
          <Skeleton width="100%" height={32} />
          <Skeleton width="100%" height={32} />
        </div>
      ))}
    </div>
  );
}

/**
 * The board's automations in the right pane (classic `rooms/boards/automations#show`): the
 * auto-assign-by-tag rules and the SLA timers, for the board's creator and administrators. Every
 * change saves at once and answers the settings as they now stand. Changes go one at a time, and
 * `board.automations.changed` (someone else's change) refetches once none is pending; an answer
 * older than the one shown is dropped.
 */
export function BoardAutomationsPane({ roomId }: { readonly roomId: number }) {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [generation, setGeneration] = useState(0);
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const issued = useRef(0);
  const shown = useRef(0);
  const refetchWanted = useRef(false);
  const signal = useStore((state) => state.boardAutomationsChanged[roomId] ?? 0);
  const signalSeen = useRef(signal);

  /** Shows `settings` unless an answer to a later request is already showing. */
  const show = useCallback((ticket: number, settings: BoardAutomations) => {
    if (ticket > shown.current) {
      shown.current = ticket;
      setLoad({ status: "ready", settings });
    }
  }, []);

  const refetch = useCallback(() => {
    if (busy.current) {
      refetchWanted.current = true;

      return;
    }

    issued.current += 1;

    const ticket = issued.current;

    actions.boards.automations(roomId).then(
      (settings) => show(ticket, settings),
      // The settings shown stay; the next change or signal tries again.
      () => undefined,
    );
  }, [roomId, show]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: generation is the Retry trigger
  useEffect(() => {
    let current = true;

    issued.current += 1;

    const ticket = issued.current;

    setLoad({ status: "loading" });
    actions.boards.automations(roomId).then(
      (settings) => {
        if (current) {
          show(ticket, settings);
        }
      },
      (error: Error) => {
        if (current && ticket > shown.current) {
          setLoad({ status: "error", message: error.message, refusal: refusalOf(error) });
        }
      },
    );

    return () => {
      current = false;
    };
  }, [roomId, generation, show]);

  useEffect(() => {
    if (signal !== signalSeen.current) {
      signalSeen.current = signal;
      refetch();
    }
  }, [signal, refetch]);

  const run = (change: () => Promise<BoardAutomations>): Promise<BoardAutomations> | null => {
    if (busy.current) {
      return null;
    }

    busy.current = true;
    setPending(true);
    issued.current += 1;

    const ticket = issued.current;

    return change()
      .then((settings) => {
        show(ticket, settings);

        return settings;
      })
      .finally(() => {
        busy.current = false;
        setPending(false);

        if (refetchWanted.current) {
          refetchWanted.current = false;
          refetch();
        }
      });
  };

  const mutations: Mutations = { pending, run };

  return (
    <PaneFrame title="Automations" subtitle={<RoomName roomId={roomId} />}>
      {load.status === "error" ? (
        load.refusal === "forbidden" ? (
          <PaneEmpty
            icon="lock"
            title="Automations are limited"
            text="Only the person who made this board and administrators can open them."
          />
        ) : load.refusal === "unavailable" ? (
          <PaneEmpty
            icon="ban"
            title="This board isn't available"
            text="You aren't in it, or it isn't a board."
          />
        ) : (
          <PaneError
            message={`The board's automations couldn't be loaded. ${load.message}`}
            onRetry={() => setGeneration((count) => count + 1)}
          />
        )
      ) : (
        <SkeletonReveal loading={load.status === "loading"} skeleton={<AutomationsSkeleton />}>
          {load.status === "ready" ? (
            <div className="automations">
              <p className="automation-intro">
                Tag rules and SLA timers run for every post on this board. Only the board's creator
                and administrators can change them.
              </p>
              <TagRules roomId={roomId} settings={load.settings} mutations={mutations} />
              <SlaTimers roomId={roomId} settings={load.settings} mutations={mutations} />
              <section className="automation-section" aria-labelledby="automation-digest-title">
                <h3 id="automation-digest-title" className="post-section-title">
                  Stale-work digest
                </h3>
                <p className="automation-text">
                  Every day the board posts one quiet digest of the posts sitting past their nudge
                  time. No configuration needed.
                </p>
              </section>
            </div>
          ) : null}
        </SkeletonReveal>
      )}
    </PaneFrame>
  );
}
