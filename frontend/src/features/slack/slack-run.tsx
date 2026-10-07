import { Link, notFound, useNavigate, useParams } from "@tanstack/react-router";
import { type Ref, useCallback, useEffect, useRef, useState } from "react";
import type { SlackIssue } from "../../gen/SlackIssue.ts";
import type { SlackRun } from "../../gen/SlackRun.ts";
import type { SlackRunChange } from "../../gen/SlackRunChange.ts";
import { slack } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { toast } from "../../ui/toast-store.ts";
import { auditTime } from "../admin/admin-format.ts";
import { adminFailure, Confirm } from "../admin/admin-parts.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { SettingsGroup, SettingsPage, useBusy } from "../settings/settings-parts.tsx";
import { CONFIRM, countLines, keyed, peopleLine } from "./slack-format.ts";
import { newestOnly, usePoll } from "./slack-poll.ts";
import "../admin/admin.css";
import "./slack.css";

/** The run id in the URL (`.../runs/$runId` or `/app/settings/slack/$runId`). */
export function useRunId(): number {
  const { runId } = useParams({ strict: false });
  const id = Number(runId);

  if (!Number.isSafeInteger(id) || id <= 0) {
    throw notFound();
  }

  return id;
}

/** Where focus waits while a confirmation closes: it isn't lost there yet. */
const OVERLAY = "dialog, [role='alertdialog'], [role='menu'], [popover]";

/** How often, and how long, focus is checked while a closing confirmation still holds it. */
const FOCUS_POLL_MS = 50;

const FOCUS_LIMIT_MS = 2000;

/** A confirmation waiting to be answered. */
interface Ask {
  readonly title: string;
  readonly message: string;
  readonly label: string;
  readonly danger: boolean;
  readonly run: () => void;
}

/** One fact on the status section; `live` announces its changes. */
function Fact({
  label,
  live = false,
  children,
}: {
  readonly label: string;
  readonly live?: boolean;
  readonly children: string;
}) {
  return (
    <div className="slack-fact">
      <dt>{label}</dt>
      <dd aria-live={live ? "polite" : undefined}>{children}</dd>
    </div>
  );
}

/**
 * The classic status section: where the run stands, read again while it is active. Only the
 * status itself is announced as it changes, not every step the run works through. `focusRef`
 * takes focus when the button someone used goes away.
 */
export function RunStatus({
  run,
  focusRef,
}: {
  readonly run: SlackRun;
  readonly focusRef?: Ref<HTMLElement>;
}) {
  const counts = run.counts === null ? null : countLines(run.counts);

  return (
    <section ref={focusRef} tabIndex={-1} aria-label="Run status">
      <dl className="slack-facts">
        <Fact label="Status" live>
          {run.status}
        </Fact>
        {run.phase === null ? null : <Fact label="Phase">{run.phase}</Fact>}
        {run.current === null ? null : <Fact label="Working on">{run.current}</Fact>}
        {run.queuedBehind ? <Fact label="Queue">Queued behind another import.</Fact> : null}
        {run.people === null ? null : <Fact label="People">{peopleLine(run.people)}</Fact>}
        {counts === null ? null : (
          <>
            <Fact label="Rooms">{counts.rooms}</Fact>
            <Fact label="Messages">{counts.messages}</Fact>
            <Fact label="Extras">{counts.extras}</Fact>
          </>
        )}
        {run.apiCalls === null ? null : <Fact label="Slack API calls">{`${run.apiCalls}`}</Fact>}
        <Fact label="Issues">{`${run.issuesCount}`}</Fact>
        <Fact label="Started">{auditTime(run.startedAt ?? run.createdAt)}</Fact>
        {run.finishedAt === null ? null : <Fact label="Finished">{auditTime(run.finishedAt)}</Fact>}
      </dl>
      {run.error === null ? null : (
        <p className="settings-callout slack-error" role="alert">
          <strong>Error:</strong> {run.error}
        </p>
      )}
    </section>
  );
}

/** The run's buttons, as the classic page offers them. */
function RunActions({
  admin,
  run,
  onChange,
}: {
  readonly admin: boolean;
  readonly run: SlackRun;
  readonly onChange: (change: SlackRunChange, started: boolean) => void;
}) {
  const [ask, setAsk] = useState<Ask | null>(null);
  const { busy, track } = useBusy();

  const act = (key: string, work: () => Promise<SlackRunChange>, failure: string) =>
    void track(
      key,
      work().then(
        (change) => onChange(change, change.run.id !== run.id),
        (error: Error) => adminFailure(failure, error),
      ),
    );

  const cancel = () =>
    setAsk({
      title: "Cancel run",
      message: admin ? CONFIRM.cancelWorkspace : CONFIRM.cancelPersonal,
      label: "Cancel run",
      danger: true,
      run: () => act("cancel", () => slack.cancel(admin, run.id), "Couldn't cancel the run"),
    });

  const undo = () =>
    setAsk({
      title: "Undo import",
      message: CONFIRM.undo,
      label: "Undo import",
      danger: true,
      run: () => act("undo", () => slack.undo(admin, run.id), "Couldn't undo the import"),
    });

  const catchUp = () =>
    setAsk({
      title: "Run catch-up import",
      message: CONFIRM.catchUp,
      label: "Run catch-up import",
      danger: false,
      run: () => act("catch-up", () => slack.catchUp(run.id), "Couldn't start the catch-up"),
    });

  const blocked = run.undoBlockedReason;

  return (
    <>
      <div className="settings-actions">
        {run.cancellable ? (
          <Button variant="danger" loading={busy("cancel")} disabled={busy()} onClick={cancel}>
            Cancel run
          </Button>
        ) : null}
        {blocked !== null || run.undoable ? (
          <Button
            variant="danger"
            loading={busy("undo")}
            disabled={blocked !== null || busy()}
            aria-describedby={blocked === null ? undefined : "slack-undo-blocked"}
            onClick={undo}
          >
            Undo import
          </Button>
        ) : null}
        {admin && run.planReady ? (
          <Link
            to="/admin/slack/runs/$runId/plan"
            params={{ runId: `${run.id}` }}
            className="button"
            data-variant="primary"
          >
            Review the plan
          </Link>
        ) : null}
        {admin && run.catchUp ? (
          <Button loading={busy("catch-up")} disabled={busy()} onClick={catchUp}>
            Run catch-up import
          </Button>
        ) : null}
      </div>
      {blocked === null ? null : (
        <p id="slack-undo-blocked" className="settings-hint text-faint">
          {blocked}
        </p>
      )}
      {admin && run.catchUp ? (
        <p className="settings-hint text-faint">
          Catch-up repeats this import's conversations and targets to pick up what changed in Slack
          since. Already-imported objects are skipped, never duplicated.
        </p>
      ) : null}
      <Confirm ask={ask} onCancel={() => setAsk(null)} />
    </>
  );
}

/** A completed preview's conversations, to check and import (the personal page's plan). */
function PreviewPlan({ run }: { readonly run: SlackRun }) {
  const navigate = useNavigate();

  const [checked, setChecked] = useState<ReadonlySet<string>>(
    () => new Set(run.conversations.map((each) => each.id)),
  );

  const [ask, setAsk] = useState<Ask | null>(null);
  const { busy, track } = useBusy();

  const toggle = (id: string, on: boolean) =>
    setChecked((current) => {
      const next = new Set(current);

      if (on) next.add(id);
      else next.delete(id);

      return next;
    });

  const start = () =>
    void track(
      "import",
      slack
        .startPersonal({
          mode: "import",
          dryRunId: run.id,
          conversationIds: run.conversations.flatMap((each) =>
            checked.has(each.id) ? [each.id] : [],
          ),
        })
        .then(
          (change) => {
            toast({ title: change.notice, tone: "success" });
            void navigate({ to: "/settings/slack/$runId", params: { runId: `${change.run.id}` } });
          },
          (error: Error) => adminFailure("Couldn't start the import", error),
        ),
    );

  return (
    <SettingsGroup title="Your plan">
      <div className="settings-actions">
        <Button
          size="sm"
          onClick={() => setChecked(new Set(run.conversations.map((each) => each.id)))}
        >
          Select all
        </Button>
        <Button size="sm" onClick={() => setChecked(new Set())}>
          Select none
        </Button>
      </div>
      <section className="admin-audit-wrap" aria-label="Conversations in the preview">
        <table className="admin-audit-table">
          <thead>
            <tr>
              <th scope="col">
                <span className="visually-hidden">Import?</span>
              </th>
              <th scope="col">Conversation</th>
              <th scope="col">Type</th>
              <th scope="col">Members</th>
              <th scope="col">Messages</th>
              <th scope="col">Threads</th>
            </tr>
          </thead>
          <tbody>
            {run.conversations.map((each) => (
              <tr key={each.id}>
                <td>
                  <Checkbox
                    checked={checked.has(each.id)}
                    onCheckedChange={(on) => toggle(each.id, on)}
                    label={<span className="visually-hidden">Import {each.name}</span>}
                  />
                </td>
                <td>{each.name}</td>
                <td>{each.kind}</td>
                <td>{each.members}</td>
                <td>{each.messages}</td>
                <td>{each.threads}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
      <div className="settings-actions">
        <Button
          variant="primary"
          loading={busy("import")}
          disabled={busy() || checked.size === 0}
          onClick={() =>
            setAsk({
              title: "Import checked",
              message: CONFIRM.importChecked,
              label: "Import checked",
              danger: false,
              run: start,
            })
          }
        >
          Import checked
        </Button>
      </div>
      <Confirm ask={ask} onCancel={() => setAsk(null)} />
    </SettingsGroup>
  );
}

/** The run's issues, a page at a time (the administrator's page only). */
export function Issues({
  count,
  issues,
  more,
  onMore,
}: {
  readonly count: number;
  readonly issues: readonly SlackIssue[];
  readonly more: boolean;
  readonly onMore: () => Promise<void>;
}) {
  const { busy, track } = useBusy();
  const list = useRef<HTMLUListElement>(null);
  // Where the older issues start, while they load: once the last page is in and its button
  // gone, focus goes to the first of them.
  const [landing, setLanding] = useState<number | null>(null);

  useEffect(() => {
    if (landing === null || issues.length <= landing) {
      return;
    }

    if (!more) {
      list.current?.querySelectorAll<HTMLElement>(":scope > li")[landing]?.focus();
    }

    setLanding(null);
  }, [landing, more, issues.length]);

  const older = () => {
    setLanding(issues.length);
    void track("more", onMore());
  };

  return (
    <SettingsGroup title={`Issues (${count})`}>
      {issues.length === 0 ? (
        <p className="text-muted">No issues recorded.</p>
      ) : (
        <ul className="slack-issues" ref={list}>
          {keyed(
            issues,
            (issue) => `${issue.level}\u0000${issue.slackRef}\u0000${issue.message}`,
          ).map(({ key, row: issue }) => (
            <li key={key} tabIndex={-1}>
              <strong>{issue.level}</strong>
              {issue.slackRef === null ? null : (
                <>
                  {" "}
                  <code className="admin-code">{issue.slackRef}</code>
                </>
              )}{" "}
              — {issue.message}
            </li>
          ))}
        </ul>
      )}
      {more ? (
        <div className="settings-actions">
          <Button loading={busy("more")} disabled={busy()} onClick={older}>
            Older issues
          </Button>
        </div>
      ) : null}
    </SettingsGroup>
  );
}

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | {
      readonly status: "ready";
      readonly run: SlackRun;
      readonly issues: readonly SlackIssue[];
      readonly nextPage: number | null;
    };

/**
 * One run, as its classic page shows it: the status (read again every few seconds while the run
 * is active), the cancel, undo, plan and catch-up buttons, and for an administrator the issues;
 * a person's completed preview lists its conversations to import.
 */
export function SlackRunView({
  admin,
  runId,
}: {
  readonly admin: boolean;
  readonly runId: number;
}) {
  const navigate = useNavigate();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const status = useRef<HTMLElement>(null);
  const [focusAsked, setFocusAsked] = useState(0);
  // A read that started before an undo must not put back the status the undo replaced.
  const [tickets] = useState(newestOnly);

  const fetchRun = useCallback(() => {
    const latest = tickets.take();

    const work = admin
      ? slack.runPage(runId)
      : slack.status(false, runId).then((run) => ({ run, issues: [], nextPage: null }));

    work.then(
      (page) => {
        if (latest()) setLoad({ status: "ready", ...page });
      },
      (error: Error) => {
        if (latest()) setLoad({ status: "error", message: error.message });
      },
    );
  }, [admin, runId, tickets]);

  useEffect(fetchRun, [fetchRun]);

  const replace = useCallback(
    (run: SlackRun) =>
      setLoad((current) => (current.status === "ready" ? { ...current, run } : current)),
    [],
  );

  const read = useCallback(
    (id: number) => {
      const latest = tickets.take();

      return slack.status(admin, id).then((fresh) => {
        if (!latest()) return null;

        replace(fresh);

        return fresh;
      });
    },
    [admin, replace, tickets],
  );

  // When the button someone used is gone after their write, focus goes to the status: once the
  // confirmation has let go of focus and it has fallen to the page. Focus someone moved elsewhere
  // stays put.
  useEffect(() => {
    if (focusAsked === 0) {
      return;
    }

    const land = (): boolean => {
      const active = document.activeElement;

      if (active !== null && active !== document.body) {
        return active.closest(OVERLAY) === null;
      }

      status.current?.focus();

      return true;
    };

    if (land()) {
      return;
    }

    let waited = 0;

    const timer = window.setInterval(() => {
      waited += FOCUS_POLL_MS;

      if (land() || waited >= FOCUS_LIMIT_MS) {
        window.clearInterval(timer);
      }
    }, FOCUS_POLL_MS);

    return () => window.clearInterval(timer);
  }, [focusAsked]);

  const run = load.status === "ready" ? load.run : null;

  // Once the run settles its issues are final: read the page again for them.
  const trouble = usePoll(run, read, fetchRun);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchRun();
  };

  const changed = (change: SlackRunChange, started: boolean) => {
    toast({ title: change.notice, tone: "success" });

    if (started) {
      void navigate(
        admin
          ? { to: "/admin/slack/runs/$runId", params: { runId: `${change.run.id}` } }
          : { to: "/settings/slack/$runId", params: { runId: `${change.run.id}` } },
      );

      return;
    }

    tickets.take();
    replace(change.run);
    setFocusAsked((count) => count + 1);
  };

  const more = async () => {
    if (load.status !== "ready" || load.nextPage === null) {
      return;
    }

    try {
      const page = await slack.runPage(runId, load.nextPage);

      setLoad((current) =>
        current.status === "ready"
          ? { ...current, issues: [...current.issues, ...page.issues], nextPage: page.nextPage }
          : current,
      );
    } catch (error) {
      adminFailure(
        "Couldn't load older issues",
        error instanceof Error ? error : new Error(`${error}`),
      );
    }
  };

  return (
    <SettingsPage
      title={run?.title ?? "Import run"}
      description={
        run === null ? undefined : `Started by ${run.startedBy} ${auditTime(run.createdAt)}.`
      }
    >
      <Link
        to={admin ? "/admin/slack/runs" : "/settings/slack"}
        className="settings-classic-link slack-back"
      >
        <Icon name="chevron-left" size={14} />
        {admin ? "All import runs" : "Your imports"}
      </Link>
      {load.status === "loading" ? <PaneListSkeleton rows={4} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {load.status === "ready" ? (
        <>
          <SettingsGroup title="Progress">
            <RunStatus run={load.run} focusRef={status} />
            {trouble === null ? null : (
              <p className="settings-hint text-faint" role="status">
                {trouble === "stopped"
                  ? "Can't refresh this run any more. Reload the page to see where it stands."
                  : "Can't refresh right now. Trying again…"}
              </p>
            )}
            <RunActions admin={admin} run={load.run} onChange={changed} />
          </SettingsGroup>
          {!admin && load.run.conversations.length > 0 ? <PreviewPlan run={load.run} /> : null}
          {admin ? (
            <Issues
              count={load.run.issuesCount}
              issues={load.issues}
              more={load.nextPage !== null}
              onMore={more}
            />
          ) : null}
        </>
      ) : null}
    </SettingsPage>
  );
}
