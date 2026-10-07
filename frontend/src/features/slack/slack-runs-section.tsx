import { Link, useSearch } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import type { SlackRunList } from "../../gen/SlackRunList.ts";
import { slack } from "../../sync/admin.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { auditTime } from "../admin/admin-format.ts";
import { AdministratorsOnly, useAdmin } from "../admin/admin-parts.tsx";
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
          <table className="admin-audit-table">
            <thead>
              <tr>
                <th scope="col">Run</th>
                <th scope="col">Kind</th>
                <th scope="col">Mode</th>
                <th scope="col">Status</th>
                <th scope="col">Started by</th>
                <th scope="col">When</th>
              </tr>
            </thead>
            <tbody>
              {load.list.runs.map((run) => (
                <tr key={run.id}>
                  <td>
                    <Link to="/admin/slack/runs/$runId" params={{ runId: `${run.id}` }}>
                      #{run.id}
                    </Link>
                  </td>
                  <td>{run.kind}</td>
                  <td>{modeLabel(run.mode)}</td>
                  <td>{run.status}</td>
                  <td>{run.startedBy}</td>
                  <td>{auditTime(run.createdAt)}</td>
                </tr>
              ))}
            </tbody>
          </table>
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
