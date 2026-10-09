import { Link, useSearch } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import type { SlackRunList } from "../../gen/SlackRunList.ts";
import { slack } from "../../sync/admin.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { auditTime } from "../admin/admin-format.ts";
import {
  AdministratorsOnly,
  CardCell,
  CardRow,
  CardTable,
  useAdmin,
} from "../admin/admin-parts.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { SettingsPage } from "../settings/settings-parts.tsx";
import { modeLabel } from "./slack-format.ts";
import { SlackRunView, useRunId } from "./slack-run.tsx";
import "../admin/admin.css";
import "./slack.css";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly list: SlackRunList };

/** Import runs: every workspace and personal run, newest first, as the classic list shows them. */
export function SlackRunsSection() {
  const { workspace } = useAdmin();
  const [load, setLoad] = useState<Load>({ status: "loading" });

  const fetchRuns = useCallback(() => {
    slack.runs().then(
      (list) => setLoad({ status: "ready", list }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(() => {
    if (workspace.canAdminister) fetchRuns();
  }, [fetchRuns, workspace.canAdminister]);

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  const reload = () => {
    setLoad({ status: "loading" });
    fetchRuns();
  };

  return (
    <SettingsPage title="Import runs">
      <Link to="/admin/slack" className="settings-classic-link slack-back">
        <Icon name="chevron-left" size={14} />
        Slack import
      </Link>
      {load.status === "loading" ? <PaneListSkeleton rows={4} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {load.status === "ready" && load.list.runs.length === 0 ? (
        <p className="text-muted">
          No runs yet. <Link to="/admin/slack">Start a dry run</Link>.
        </p>
      ) : null}
      {load.status === "ready" && load.list.runs.length > 0 ? (
        <section className="admin-audit-wrap" aria-label="Import runs">
          <CardTable columns={["Run", "Kind", "Mode", "Status", "Started by", "When"]}>
            {load.list.runs.map((run) => (
              <CardRow key={run.id}>
                <CardCell column="Run" title>
                  <Link to="/admin/slack/runs/$runId" params={{ runId: `${run.id}` }}>
                    #{run.id}
                  </Link>
                </CardCell>
                <CardCell column="Kind">{run.kind}</CardCell>
                <CardCell column="Mode">{modeLabel(run.mode)}</CardCell>
                <CardCell column="Status">{run.status}</CardCell>
                <CardCell column="Started by">{run.startedBy}</CardCell>
                <CardCell column="When">{auditTime(run.createdAt)}</CardCell>
              </CardRow>
            ))}
          </CardTable>
        </section>
      ) : null}
    </SettingsPage>
  );
}

/** One run, for an administrator: any run, with its issues. */
export function SlackRunSection() {
  const { workspace } = useAdmin();
  const runId = useRunId();
  const { page } = useSearch({ strict: false });

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  return <SlackRunView key={runId} admin runId={runId} throughPage={page ?? 1} />;
}
