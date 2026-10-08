import { type ReactNode, useState } from "react";
import type { AgentStep } from "../../gen/AgentStep.ts";
import type { User } from "../../gen/User.ts";
import type { WorkDetail } from "../../gen/WorkDetail.ts";
import type { WorkHistoryEntry } from "../../gen/WorkHistoryEntry.ts";
import type { WorkLink } from "../../gen/WorkLink.ts";
import type { WorkLinkKind } from "../../gen/WorkLinkKind.ts";
import type { WorkOwnerSnapshot } from "../../gen/WorkOwnerSnapshot.ts";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { formatFull } from "../../lib/time.ts";
import type { ThreadPermissions } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Accordion } from "../../ui/accordion.tsx";
import { Button } from "../../ui/button.tsx";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { Menu, MenuGroup, MenuRadioItem, MenuSeparator } from "../../ui/menu.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { BodyHtml } from "../messages/body-html.tsx";
import { isAgent } from "../people/people.ts";
import { timeAgo } from "../threads/thread-format.ts";
import { useNow } from "../threads/use-now.ts";
import { HandoffDialog } from "../work/handoff-dialog.tsx";
import {
  classicWorkUrl,
  parseTags,
  safeHttpsUrl,
  stepDuration,
  stepStatusLabel,
  tagsProblem,
  WORK_STATUS_LABEL,
  WORK_STATUSES,
} from "./board-format.ts";
import { OwnerLine, StatusChip, TagList, UserFace } from "./board-parts.tsx";

const RESULT_MAX = 20_000;

/** Runs a work write; a refusal says why in a toast. */
function save(threadId: number, body: Parameters<typeof actions.work.update>[1], failure: string) {
  return actions.work.update(threadId, body).then(
    () => true,
    (error: Error) => {
      toast({ title: failure, description: error.message, tone: "danger" });

      return false;
    },
  );
}

/** One labelled fact of the post: the label on the left, the value (or its control) beside it. */
function Fact({ label, children }: { readonly label: string; readonly children: ReactNode }) {
  return (
    <div className="post-fact">
      <dt className="post-fact-label">{label}</dt>
      <dd className="post-fact-value">{children}</dd>
    </div>
  );
}

function StatusFact({
  threadId,
  status,
  editable,
}: {
  readonly threadId: number;
  readonly status: WorkStatus;
  readonly editable: boolean;
}) {
  if (!editable) {
    return <StatusChip status={status} />;
  }

  return (
    <Menu
      label="Status"
      trigger={(props) => (
        <button
          {...props}
          type="button"
          className="post-fact-button"
          aria-label={`Status: ${WORK_STATUS_LABEL[status]}`}
        >
          <StatusChip status={status} />
          <Icon name="chevron-down" size={12} className="post-fact-chevron" />
        </button>
      )}
    >
      <MenuGroup label="Status">
        {WORK_STATUSES.map((choice) => (
          <MenuRadioItem
            key={choice}
            checked={choice === status}
            onSelect={() => {
              if (choice !== status) {
                void save(threadId, { status: choice }, "Couldn't change the status");
              }
            }}
          >
            {WORK_STATUS_LABEL[choice]}
          </MenuRadioItem>
        ))}
      </MenuGroup>
    </Menu>
  );
}

function OwnerFact({
  threadId,
  editable,
  detail,
}: {
  readonly threadId: number;
  readonly editable: boolean;
  readonly detail: WorkDetail | null;
}) {
  const work = useStore((state) => state.threads[threadId]?.work ?? null);
  const users = useStore((state) => state.users);

  if (work === null) {
    return null;
  }

  const candidates = detail?.ownerCandidates ?? [];

  if (!editable || candidates.length === 0) {
    return <OwnerLine work={work} size={20} />;
  }

  const ownerId = work.owner?.id ?? null;
  const people = candidates.filter((candidate) => !isAgent(users[candidate.userId]));
  const agents = candidates.filter((candidate) => isAgent(users[candidate.userId]));

  const assign = (userId: number | null) => {
    if (userId !== ownerId) {
      void save(threadId, { ownerId: userId }, "Couldn't change the owner");
    }
  };

  const choice = (userId: number, description: string | null) => {
    const user: User | undefined = users[userId];

    return (
      <MenuRadioItem
        key={userId}
        checked={userId === ownerId}
        {...(description === null ? {} : { description })}
        onSelect={() => assign(userId)}
      >
        <span className="post-owner-choice">
          {user === undefined ? null : <UserFace user={user} size={18} />}
          {user?.name ?? "Someone"}
        </span>
      </MenuRadioItem>
    );
  };

  return (
    <Menu
      label="Owner"
      trigger={(props) => (
        <button {...props} type="button" className="post-fact-button" aria-label="Change owner">
          <OwnerLine work={work} size={20} />
          <Icon name="chevron-down" size={12} className="post-fact-chevron" />
        </button>
      )}
    >
      <MenuRadioItem checked={ownerId === null} icon="user-x" onSelect={() => assign(null)}>
        Unassigned
      </MenuRadioItem>
      {people.length > 0 ? <MenuSeparator /> : null}
      {people.length > 0 ? (
        <MenuGroup label="Members">{people.map((c) => choice(c.userId, null))}</MenuGroup>
      ) : null}
      {agents.length > 0 ? <MenuSeparator /> : null}
      {agents.length > 0 ? (
        <MenuGroup label="Agents">
          {agents.map((c) => choice(c.userId, c.description ?? c.provider))}
        </MenuGroup>
      ) : null}
    </Menu>
  );
}

function TagsFact({
  threadId,
  tags,
  editable,
}: {
  readonly threadId: number;
  readonly tags: readonly string[];
  readonly editable: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);

  if (!editing) {
    return (
      <span className="post-tags">
        {tags.length === 0 ? <span className="post-fact-none">None</span> : <TagList tags={tags} />}
        {editable ? (
          <Button
            variant="ghost"
            size="sm"
            icon="pencil"
            className="post-fact-edit"
            aria-label="Edit tags"
            onClick={() => {
              setText(tags.join(", "));
              setError(undefined);
              setEditing(true);
            }}
          />
        ) : null}
      </span>
    );
  }

  const submit = () => {
    const next = parseTags(text);
    const problem = tagsProblem(next);

    if (problem !== undefined) {
      setError(problem);
      setAttempt((count) => count + 1);

      return;
    }

    setBusy(true);
    actions.work
      .update(threadId, { tags: next })
      .then(
        () => setEditing(false),
        (failure: Error) => {
          const messages = failure instanceof ActionError ? (failure.fields.tags ?? []) : [];

          setError(messages.length > 0 ? `Tags ${messages.join(" and ")}.` : failure.message);
          setAttempt((count) => count + 1);
        },
      )
      .finally(() => setBusy(false));
  };

  return (
    <form
      className="post-tags-form"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <TextField
        label="Tags"
        className="post-tags-input"
        placeholder="bug, api"
        hint="Up to 5, separated by commas."
        value={text}
        error={error}
        attempt={attempt}
        autoFocus
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.stopPropagation();
            setEditing(false);
          }
        }}
      />
      <div className="post-inline-actions">
        <Button variant="secondary" size="sm" onClick={() => setEditing(false)}>
          Cancel
        </Button>
        <Button variant="primary" size="sm" type="submit" loading={busy}>
          Save tags
        </Button>
      </div>
    </form>
  );
}

const LINK_ICON = {
  pull_request: "git-pull-request",
  event: "calendar",
  drive_file: "file-text",
} as const satisfies Record<WorkLinkKind, IconName>;

/** A link goes into an `href` only when it's `https://` or a site path, as the contract says. */
function linkHref(url: string): string | null {
  return url.startsWith("https://") || (url.startsWith("/") && !url.startsWith("//")) ? url : null;
}

function LinkRow({ link }: { readonly link: WorkLink }) {
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
      {detail === null ? null : (
        <span className="post-link-state" data-state={link.pullRequestState ?? "cancelled"}>
          {detail}
        </span>
      )}
    </li>
  );
}

function ResultSection({
  threadId,
  detail,
  editable,
}: {
  readonly threadId: number;
  readonly detail: WorkDetail | null;
  readonly editable: boolean;
}) {
  const updatedAt = useStore((state) => state.threads[threadId]?.work?.resultUpdatedAt ?? null);

  const editor = useStore((state) =>
    detail?.resultUpdatedById == null ? undefined : state.users[detail.resultUpdatedById],
  );

  const now = useNow();
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const html = detail?.resultHtml ?? null;

  const submit = () => {
    setBusy(true);
    void save(
      threadId,
      { resultMarkdown: text.trim() === "" ? null : text },
      "Couldn't save the result",
    )
      .then((saved) => {
        if (saved) {
          setEditing(false);
        }
      })
      .finally(() => setBusy(false));
  };

  return (
    <section className="post-result" aria-labelledby={`post-result-${threadId}`}>
      <header className="post-section-head">
        <h3 id={`post-result-${threadId}`} className="post-section-title">
          <Icon name="circle-check" size={14} />
          Result
        </h3>
        {editable && !editing ? (
          <Button
            variant="ghost"
            size="sm"
            icon="pencil"
            onClick={() => {
              setText(detail?.resultMarkdown ?? "");
              setEditing(true);
            }}
          >
            Edit result
          </Button>
        ) : null}
      </header>
      {editing ? (
        <form
          className="post-result-form"
          onSubmit={(event) => {
            event.preventDefault();
            submit();
          }}
        >
          <textarea
            className="input post-result-input"
            aria-label="Result in Markdown"
            placeholder="Record the outcome in Markdown…"
            maxLength={RESULT_MAX}
            rows={6}
            value={text}
            // biome-ignore lint/a11y/noAutofocus: the editor replaces the button that opened it
            autoFocus
            onChange={(event) => setText(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
                event.preventDefault();
                submit();
              } else if (event.key === "Escape") {
                event.stopPropagation();
                setEditing(false);
              }
            }}
          />
          <div className="post-inline-actions">
            <Button variant="secondary" size="sm" onClick={() => setEditing(false)}>
              Cancel
            </Button>
            <Button variant="primary" size="sm" type="submit" loading={busy}>
              Save result
            </Button>
          </div>
        </form>
      ) : html === null ? (
        <p className="post-result-none">No result recorded yet.</p>
      ) : (
        <>
          <BodyHtml html={html} className="post-result-body message-body" />
          {updatedAt === null ? null : (
            <p className="post-result-meta" title={formatFull(updatedAt)}>
              Updated {timeAgo(updatedAt, now)}
              {detail?.resultUpdatedById == null ? null : ` by ${editor?.name ?? "someone"}`}
            </p>
          )}
        </>
      )}
    </section>
  );
}

function statusName(status: WorkStatus | null): string {
  return status === null ? "Ordinary thread" : (WORK_STATUS_LABEL[status] ?? status);
}

function ownerName(owner: WorkOwnerSnapshot | null): string {
  return owner?.name ?? "Unassigned";
}

/** One history line, worded as the classic post words it. */
function historyText(entry: WorkHistoryEntry): string {
  const owners = `owner ${ownerName(entry.fromOwner)} → ${ownerName(entry.toOwner)}`;

  if (entry.kind === "result") {
    return "updated the result";
  }

  if (entry.kind === "handoff") {
    const handoff = entry.handoff;

    const package_ =
      handoff === null
        ? ""
        : ` · ${handoff.summary} (${handoff.linkCount} ${handoff.linkCount === 1 ? "link" : "links"}, ${handoff.questionCount} open ${handoff.questionCount === 1 ? "question" : "questions"})`;

    return `handed off ${owners}${package_}`;
  }

  const parts: string[] = [];

  if (entry.fromStatus !== entry.toStatus) {
    parts.push(`status ${statusName(entry.fromStatus)} → ${statusName(entry.toStatus)}`);
  }

  if ((entry.fromOwner?.userId ?? null) !== (entry.toOwner?.userId ?? null)) {
    parts.push(owners);
  }

  if (entry.note !== null) {
    parts.push(`Note: ${entry.note}`);
  }

  return parts.join(" · ");
}

function History({ history }: { readonly history: readonly WorkHistoryEntry[] }) {
  const users = useStore((state) => state.users);
  const now = useNow();

  if (history.length === 0) {
    return null;
  }

  return (
    <div className="post-history">
      <Accordion title={`Work history · ${history.length}`}>
        <ol className="post-history-list">
          {history.map((entry) => {
            const actor = entry.actorId === null ? undefined : users[entry.actorId];

            return (
              <li key={entry.id} className="post-history-entry" data-kind={entry.kind}>
                <span className="post-history-text">
                  <strong>{actor?.name ?? "Former member"}</strong>
                  {entry.kind === "update" ? " · " : " "}
                  {historyText(entry)}
                </span>
                <time
                  className="post-history-time"
                  dateTime={entry.createdAt}
                  title={formatFull(entry.createdAt)}
                >
                  {timeAgo(entry.createdAt, now)}
                </time>
              </li>
            );
          })}
        </ol>
      </Accordion>
    </div>
  );
}

function Steps({ steps }: { readonly steps: readonly AgentStep[] }) {
  if (steps.length === 0) {
    return null;
  }

  return (
    <div className="post-history post-steps">
      <Accordion title={`Steps (${steps.length})`}>
        <ol className="post-step-list">
          {steps.map((step) => (
            <li key={step.id} className="post-step" data-status={step.status}>
              <div className="post-step-head">
                <strong className="post-step-name">{step.name}</strong>
                <span className="post-step-status">{stepStatusLabel(step.status)}</span>
                {step.durationMs === null ? null : (
                  <span className="post-step-duration">{stepDuration(step.durationMs)}</span>
                )}
              </div>
              <StepSummary label="In:" text={step.inputSummary} />
              <StepSummary label="Out:" text={step.outputSummary} />
            </li>
          ))}
        </ol>
      </Accordion>
    </div>
  );
}

function StepSummary({ label, text }: { readonly label: string; readonly text: string | null }) {
  if (text === null || text.trim() === "") {
    return null;
  }

  return (
    <p className="post-step-io">
      <span className="post-step-io-label">{label}</span> {text}
    </p>
  );
}

/** Link management stays classic; handoffs use the shared work dialog. */
function PostWorkActions({
  threadId,
  detail,
}: {
  readonly threadId: number;
  readonly detail: WorkDetail | null;
}) {
  const [handingOff, setHandingOff] = useState(false);
  const name = useStore((state) => state.threads[threadId]?.name ?? "Post");

  return (
    <div className="post-classic">
      <a className="post-classic-link" href={classicWorkUrl(threadId, "links")}>
        Manage links
        <Icon name="arrow-up-right" size={12} />
      </a>
      {detail === null || detail.handoffReceivers.length === 0 ? null : (
        <>
          <Button variant="secondary" size="sm" icon="send" onClick={() => setHandingOff(true)}>
            Hand off to an agent
          </Button>
          <HandoffDialog
            threadId={threadId}
            threadName={name}
            work={detail}
            open={handingOff}
            onOpenChange={setHandingOff}
          />
        </>
      )}
    </div>
  );
}

/**
 * A board post's work, on top of its discussion in the right pane: status, owner and tags (each
 * a control for whoever may change it), the agent's run, what's linked, the pinned result with
 * its editor, the agent's steps, the work history, and link management and handoff controls.
 */
export function PostWork({ threadId }: { readonly threadId: number }) {
  const work = useStore((state) => state.threads[threadId]?.work ?? null);
  const detail = useStore((state) => state.threadPanes[threadId]?.work ?? null);

  const permissions: ThreadPermissions | null = useStore(
    (state) => state.threadPanes[threadId]?.permissions ?? null,
  );

  if (work === null) {
    return null;
  }

  const run = safeHttpsUrl(work.runUrl);

  return (
    <div className="post-work">
      <dl className="post-facts">
        <Fact label="Status">
          <StatusFact
            threadId={threadId}
            status={work.status}
            editable={permissions?.canUpdateWorkStatus === true}
          />
        </Fact>
        <Fact label="Owner">
          <OwnerFact
            threadId={threadId}
            editable={permissions?.canAssignWork === true}
            detail={detail}
          />
        </Fact>
        <Fact label="Tags">
          <TagsFact
            threadId={threadId}
            tags={work.tags}
            editable={permissions?.canManageWork === true}
          />
        </Fact>
        {run === null ? null : (
          <Fact label="Run">
            <a className="post-run" href={run} target="_blank" rel="noreferrer">
              Open run
              <Icon name="arrow-up-right" size={12} />
            </a>
          </Fact>
        )}
      </dl>
      {work.links.length > 0 ? (
        <section className="post-links" aria-label="Linked">
          <h3 className="post-section-title">
            <Icon name="link" size={14} />
            Linked
          </h3>
          <ul className="post-link-list">
            {work.links.map((link) => (
              <LinkRow key={link.id} link={link} />
            ))}
          </ul>
        </section>
      ) : null}
      <ResultSection
        threadId={threadId}
        detail={detail}
        editable={permissions?.canManageWork === true}
      />
      <Steps steps={detail?.steps ?? []} />
      <History history={detail?.history ?? []} />
      {permissions?.canManageWork === true ? (
        <PostWorkActions threadId={threadId} detail={detail} />
      ) : null}
    </div>
  );
}
