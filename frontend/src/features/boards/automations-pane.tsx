import { useEffect, useId, useState } from "react";
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
import { isAgent } from "../people/people.ts";
import { PaneFrame, RoomName } from "../panes/pane-frame.tsx";
import { PaneError } from "../panes/pane-states.tsx";
import { MAX_SLA_MINUTES, MAX_TAG_LENGTH, minutesLabel } from "./board-format.ts";
import { StatusChip, UserFace } from "./board-parts.tsx";

type Fields = Readonly<Record<string, readonly string[]>>;

const NO_FIELDS: Fields = {};

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly settings: BoardAutomations };

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

function timerDrafts(settings: BoardAutomations): TimerDrafts {
  const draft = (status: WorkStatus): TimerDraft => {
    const timer = settings.slaTimers.find((candidate) => candidate.status === status);

    return timer === undefined
      ? { nudge: "", escalate: "" }
      : { nudge: String(timer.nudgeAfterMinutes), escalate: String(timer.escalateAfterMinutes) };
  };

  return { planned: draft("planned"), inProgress: draft("in_progress"), blocked: draft("blocked") };
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
  onRemove,
}: {
  readonly rule: BoardTagRule;
  readonly removing: boolean;
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
        disabled={removing}
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
  onChange,
}: {
  readonly candidates: readonly number[];
  readonly value: string;
  readonly error: string | undefined;
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
  onSaved,
}: {
  readonly roomId: number;
  readonly settings: BoardAutomations;
  readonly onSaved: (settings: BoardAutomations) => void;
}) {
  const titleId = useId();
  const [tag, setTag] = useState("");
  const [assigneeId, setAssigneeId] = useState("");
  const [fields, setFields] = useState<Fields>(NO_FIELDS);
  const [attempt, setAttempt] = useState(0);
  const [saving, setSaving] = useState(false);
  const [removing, setRemoving] = useState<number | null>(null);

  const add = () => {
    setSaving(true);
    actions.boards
      .addTagRule(roomId, {
        tag,
        assigneeId: assigneeId === "" ? null : Number(assigneeId),
      })
      .then(
        (next) => {
          setTag("");
          setAssigneeId("");
          setFields(NO_FIELDS);
          onSaved(next);
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
    setRemoving(rule.id);
    actions.boards
      .removeTagRule(roomId, rule.id)
      .then(
        (next) => {
          onSaved(next);
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
            error={messageOf(fields, "tag")}
            attempt={attempt}
            onChange={(event) => setTag(event.target.value)}
          />
          <AssigneeSelect
            candidates={settings.candidates}
            value={assigneeId}
            error={messageOf(fields, "assigneeId")}
            onChange={setAssigneeId}
          />
        </div>
        <div className="automation-actions">
          <Button type="submit" variant="primary" size="sm" icon="plus" loading={saving}>
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
  onChange,
}: {
  readonly label: string;
  readonly status: WorkStatus;
  readonly draft: TimerDraft;
  readonly error: string | undefined;
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

/** "SLA timers": nudge and escalation minutes for each unfinished status, saved together. */
function SlaTimers({
  roomId,
  settings,
  onSaved,
}: {
  readonly roomId: number;
  readonly settings: BoardAutomations;
  readonly onSaved: (settings: BoardAutomations) => void;
}) {
  const titleId = useId();
  const [saved, setSaved] = useState(() => timerDrafts(settings));
  const [drafts, setDrafts] = useState(saved);
  const [fields, setFields] = useState<Fields>(NO_FIELDS);
  const [alert, setAlert] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const dirty = TIMED.some(
    ({ field }) =>
      drafts[field].nudge.trim() !== saved[field].nudge ||
      drafts[field].escalate.trim() !== saved[field].escalate,
  );

  const save = () => {
    const local = localTimerProblems(drafts);

    if (Object.keys(local).length > 0) {
      setFields(local);
      setAlert(null);

      return;
    }

    setSaving(true);
    actions.boards
      .saveSlaTimers(roomId, {
        planned: timerInput(drafts.planned),
        inProgress: timerInput(drafts.inProgress),
        blocked: timerInput(drafts.blocked),
      })
      .then(
        (next) => {
          const fresh = timerDrafts(next);

          setSaved(fresh);
          setDrafts(fresh);
          setFields(NO_FIELDS);
          setAlert(null);
          onSaved(next);
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
              disabled={saving}
              onClick={() => {
                setDrafts(saved);
                setFields(NO_FIELDS);
                setAlert(null);
              }}
            >
              Reset
            </Button>
          ) : null}
          <Button type="submit" variant="primary" size="sm" loading={saving} disabled={!dirty}>
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
 * change saves at once and answers the settings as they now stand.
 */
export function BoardAutomationsPane({ roomId }: { readonly roomId: number }) {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [generation, setGeneration] = useState(0);

  // biome-ignore lint/correctness/useExhaustiveDependencies: generation is the Retry trigger
  useEffect(() => {
    let current = true;

    setLoad({ status: "loading" });
    actions.boards.automations(roomId).then(
      (settings) => {
        if (current) {
          setLoad({ status: "ready", settings });
        }
      },
      (error: Error) => {
        if (current) {
          setLoad({ status: "error", message: error.message });
        }
      },
    );

    return () => {
      current = false;
    };
  }, [roomId, generation]);

  const saved = (settings: BoardAutomations) => setLoad({ status: "ready", settings });

  return (
    <PaneFrame title="Automations" subtitle={<RoomName roomId={roomId} />}>
      {load.status === "error" ? (
        <PaneError
          message={`The board's automations couldn't be loaded. ${load.message}`}
          onRetry={() => setGeneration((count) => count + 1)}
        />
      ) : (
        <SkeletonReveal loading={load.status === "loading"} skeleton={<AutomationsSkeleton />}>
          {load.status === "ready" ? (
            <div className="automations">
              <p className="automation-intro">
                Tag rules and SLA timers run for every post on this board. Only the board's creator
                and administrators can change them.
              </p>
              <TagRules roomId={roomId} settings={load.settings} onSaved={saved} />
              <SlaTimers roomId={roomId} settings={load.settings} onSaved={saved} />
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
