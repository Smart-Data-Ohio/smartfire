import { Link, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import type { SlackPersonal } from "../../gen/SlackPersonal.ts";
import { slack } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { toast } from "../../ui/toast-store.ts";
import { auditTime } from "../admin/admin-format.ts";
import { adminFailure, CardCell, CardRow, CardTable, Confirm } from "../admin/admin-parts.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { SettingsGroup, SettingsPage, useBusy } from "../settings/settings-parts.tsx";
import { CONFIRM, modeLabel, personalConnectionSentence } from "./slack-format.ts";
import { SlackRunView, useRunId } from "./slack-run.tsx";
import "../admin/admin.css";
import "./slack.css";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly page: SlackPersonal };

interface Ask {
  readonly title: string;
  readonly message: string;
  readonly label: string;
  readonly danger: boolean;
  readonly run: () => void;
}

/** Back to the integrations, where the Slack import card is. */
function IntegrationsBack() {
  return (
    <Link to="/settings/integrations" className="settings-classic-link slack-back">
      <Icon name="chevron-left" size={14} />
      Integrations
    </Link>
  );
}

/**
 * Import from Slack: the person's own Slack connection, a preview of their DMs, group DMs and
 * private channels, and their imports. Connecting goes through Slack's OAuth on the classic
 * routes.
 */
export function PersonalSlackSection() {
  const navigate = useNavigate();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [ask, setAsk] = useState<Ask | null>(null);
  const { busy, track } = useBusy();

  const fetchPage = useCallback(() => {
    slack.personal().then(
      (page) => setLoad({ status: "ready", page }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(fetchPage, [fetchPage]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchPage();
  };

  const preview = () =>
    void track(
      "preview",
      slack.startPersonal({ mode: "dry_run", dryRunId: null, conversationIds: [] }).then(
        (change) => {
          toast({ title: change.notice, tone: "success" });
          void navigate({ to: "/settings/slack/$runId", params: { runId: `${change.run.id}` } });
        },
        (error: Error) => adminFailure("Couldn't start the preview", error),
      ),
    );

  const disconnect = () =>
    setAsk({
      title: "Disconnect Slack",
      message: CONFIRM.disconnectPersonal,
      label: "Disconnect Slack",
      danger: true,
      run: () =>
        void track(
          "disconnect",
          slack.disconnect().then(
            ({ notice }) => {
              toast({ title: notice, tone: "success" });
              fetchPage();
            },
            (error: Error) => adminFailure("Couldn't disconnect Slack", error),
          ),
        ),
    });

  const page = load.status === "ready" ? load.page : null;
  const connected = page?.connection.state === "connected";

  return (
    <SettingsPage
      title="Import from Slack"
      description={
        page === null || !page.teamKnown
          ? undefined
          : "Bring your Slack history into Smartfire: your direct messages, group DMs, and the private channels you're in. Each one becomes visible in Smartfire to the other members of it. Public channels come over with the workspace import instead."
      }
    >
      <IntegrationsBack />
      {load.status === "loading" ? <PaneListSkeleton rows={3} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {page !== null && !page.teamKnown ? (
        <p className="settings-callout text-muted" role="status">
          An administrator needs to set up Slack import first.
        </p>
      ) : null}
      {page?.teamKnown === true ? (
        <>
          <SettingsGroup title="Your Slack connection">
            <p>{personalConnectionSentence(page.connection)}</p>
            <div className="settings-actions">
              <a
                className="button"
                data-variant={connected ? "secondary" : "primary"}
                href={page.connectPath}
              >
                {connected ? "Reconnect Slack" : "Connect Slack"}
              </a>
              {connected ? (
                <Button
                  variant="danger"
                  loading={busy("disconnect")}
                  disabled={busy()}
                  onClick={disconnect}
                >
                  Disconnect Slack
                </Button>
              ) : null}
            </div>
          </SettingsGroup>
          {connected ? (
            <SettingsGroup
              title="Preview"
              description="A preview reads your DMs, group DMs, and private channels and shows what an import would bring over. It writes nothing."
            >
              <div className="settings-actions">
                <Button
                  variant="primary"
                  loading={busy("preview")}
                  disabled={busy()}
                  onClick={preview}
                >
                  Start preview
                </Button>
              </div>
            </SettingsGroup>
          ) : null}
          <SettingsGroup title="Your imports">
            {page.runs.length === 0 ? (
              <p className="text-muted">No personal imports yet.</p>
            ) : (
              <section className="admin-audit-wrap" aria-label="Your imports">
                <CardTable columns={["Run", "Mode", "Status", "When"]}>
                  {page.runs.map((run) => (
                    <CardRow key={run.id}>
                      <CardCell column="Run" title>
                        <Link to="/settings/slack/$runId" params={{ runId: `${run.id}` }}>
                          #{run.id}
                        </Link>
                      </CardCell>
                      <CardCell column="Mode">{modeLabel(run.mode)}</CardCell>
                      <CardCell column="Status">{run.status}</CardCell>
                      <CardCell column="When">{auditTime(run.createdAt)}</CardCell>
                    </CardRow>
                  ))}
                </CardTable>
              </section>
            )}
          </SettingsGroup>
        </>
      ) : null}
      <Confirm ask={ask} onCancel={() => setAsk(null)} />
    </SettingsPage>
  );
}

/** One of the person's own runs. */
export function PersonalSlackRunSection() {
  const runId = useRunId();

  return <SlackRunView key={runId} admin={false} runId={runId} />;
}
