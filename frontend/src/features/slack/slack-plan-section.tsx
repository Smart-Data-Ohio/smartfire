import { Link, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useId, useState } from "react";
import type { SlackPlan } from "../../gen/SlackPlan.ts";
import type { SlackPreset } from "../../gen/SlackPreset.ts";
import { slack } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { AdministratorsOnly, adminFailure, Confirm, useAdmin } from "../admin/admin-parts.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { SettingsGroup, SettingsPage, useBusy } from "../settings/settings-parts.tsx";
import { CONFIRM, importBody } from "./slack-format.ts";
import { useRunId } from "./slack-run.tsx";
import "../admin/admin.css";
import "./slack.css";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly plan: SlackPlan };

interface Ask {
  readonly title: string;
  readonly message: string;
  readonly label: string;
  readonly danger: boolean;
  readonly run: () => void;
}

/** The plan's form: what to import, where each conversation goes, and the preset. */
function PlanForm({ plan }: { readonly plan: SlackPlan }) {
  const navigate = useNavigate();
  const id = useId();
  const everything = () => new Set(plan.conversations.map(({ conversation }) => conversation.id));
  const [checked, setChecked] = useState<ReadonlySet<string>>(everything);

  const [targets, setTargets] = useState<Readonly<Record<string, string>>>(() =>
    Object.fromEntries(
      plan.conversations.map(({ conversation, target }) => [conversation.id, target]),
    ),
  );

  const [oldest, setOldest] = useState(plan.defaultOldest);
  const [latest, setLatest] = useState("");
  const [ask, setAsk] = useState<Ask | null>(null);
  const { busy, track } = useBusy();

  const toggle = (conversation: string, on: boolean) =>
    setChecked((current) => {
      const next = new Set(current);

      if (on) next.add(conversation);
      else next.delete(conversation);

      return next;
    });

  const start = (preset: SlackPreset) =>
    void track(
      preset,
      slack
        .startImport(plan.runId, importBody(plan, checked, targets, preset, { oldest, latest }))
        .then(
          (change) => {
            toast({ title: change.notice, tone: "success" });
            void navigate({
              to: "/admin/slack/runs/$runId",
              params: { runId: `${change.run.id}` },
            });
          },
          (error: Error) => adminFailure("Couldn't start the import", error),
        ),
    );

  const confirm = (preset: SlackPreset) =>
    setAsk({
      title: preset === "test" ? "Test import" : "Full import",
      message: preset === "test" ? CONFIRM.testImport : CONFIRM.fullImport,
      label: preset === "test" ? "Test import" : "Full import",
      danger: false,
      run: () => start(preset),
    });

  return (
    <>
      <SettingsGroup title="Conversations">
        <div className="settings-actions">
          <Button size="sm" onClick={() => setChecked(everything())}>
            Select all
          </Button>
          <Button size="sm" onClick={() => setChecked(new Set())}>
            Select none
          </Button>
        </div>
        <section className="admin-audit-wrap" aria-label="Conversations to import">
          <table className="admin-audit-table">
            <thead>
              <tr>
                <th scope="col">
                  <span className="visually-hidden">Import?</span>
                </th>
                <th scope="col">Conversation</th>
                <th scope="col">Target</th>
                <th scope="col">Type</th>
                <th scope="col">Members</th>
                <th scope="col">Messages</th>
                <th scope="col">Threads</th>
              </tr>
            </thead>
            <tbody>
              {plan.conversations.map(({ conversation }) => (
                <tr key={conversation.id}>
                  <td>
                    <Checkbox
                      checked={checked.has(conversation.id)}
                      onCheckedChange={(on) => toggle(conversation.id, on)}
                      label={<span className="visually-hidden">Import {conversation.name}</span>}
                    />
                  </td>
                  <td>{conversation.name}</td>
                  <td>
                    <select
                      className="input settings-select"
                      aria-label={`Target for ${conversation.name}`}
                      aria-describedby={`${id}-targets`}
                      value={targets[conversation.id] ?? "new"}
                      onChange={(event) =>
                        setTargets((current) => ({
                          ...current,
                          [conversation.id]: event.target.value,
                        }))
                      }
                    >
                      <option value="new">New room</option>
                      {plan.rooms.map((room) => (
                        <option key={room.id} value={`${room.id}`}>
                          {room.name}
                        </option>
                      ))}
                      <option value="skip">Skip</option>
                    </select>
                  </td>
                  <td>
                    {conversation.kind}
                    {conversation.archived ? " · archived" : ""}
                  </td>
                  <td>{conversation.members}</td>
                  <td>{conversation.messages}</td>
                  <td>{conversation.threads}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
        <p id={`${id}-targets`} className="settings-hint text-faint">
          “New room” creates a room named after the conversation; pick a room to merge into it;
          “Skip” leaves the conversation out.
        </p>
      </SettingsGroup>
      <SettingsGroup
        title="Converted samples"
        description="Slack text on the left, the Markdown Smartfire will show on the right."
      >
        {plan.samples.length === 0 ? (
          <p className="text-muted">No samples in this dry run.</p>
        ) : (
          <div className="admin-audit-wrap">
            <table className="admin-audit-table slack-samples">
              <thead>
                <tr>
                  <th scope="col">Conversation</th>
                  <th scope="col">Slack text</th>
                  <th scope="col">Smartfire preview</th>
                </tr>
              </thead>
              <tbody>
                {plan.samples.map((sample, index) => (
                  // Samples have no id on the wire and never reorder.
                  <tr key={index}>
                    <td>{sample.conversation}</td>
                    <td className="slack-sample-text">{sample.slackText}</td>
                    <td dangerouslySetInnerHTML={{ __html: sample.html }} />
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </SettingsGroup>
      <SettingsGroup title="Start the import">
        <div className="settings-inline admin-icon-names">
          <TextField
            label="Oldest message (test import only)"
            type="date"
            value={oldest}
            onChange={(event) => setOldest(event.target.value)}
          />
          <TextField
            label="Latest message (test import only)"
            type="date"
            value={latest}
            onChange={(event) => setLatest(event.target.value)}
          />
        </div>
        <div className="settings-actions">
          <Button
            variant="primary"
            loading={busy("test")}
            disabled={busy()}
            onClick={() => confirm("test")}
          >
            Test import
          </Button>
          <Button loading={busy("full")} disabled={busy()} onClick={() => confirm("full")}>
            Full import
          </Button>
        </div>
        <p className="settings-hint text-faint">
          <strong>Test import</strong> imports the checked conversations' recent messages and can be
          undone. <strong>Full import</strong> imports everything in the checked conversations, with
          no date bounds.
        </p>
      </SettingsGroup>
      <Confirm ask={ask} onCancel={() => setAsk(null)} />
    </>
  );
}

/**
 * Import plan: a completed dry run's conversations to check, each one's target (a new room, a
 * room to merge into, or skip), the converted samples, and the test or full import.
 */
export function SlackPlanSection() {
  const { workspace } = useAdmin();
  const runId = useRunId();
  const [load, setLoad] = useState<Load>({ status: "loading" });

  const fetchPlan = useCallback(() => {
    slack.plan(runId).then(
      (plan) => setLoad({ status: "ready", plan }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, [runId]);

  useEffect(() => {
    if (workspace.canAdminister) fetchPlan();
  }, [fetchPlan, workspace.canAdminister]);

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  const reload = () => {
    setLoad({ status: "loading" });
    fetchPlan();
  };

  return (
    <SettingsPage
      title="Import plan"
      description={`From dry run #${runId}: choose what to import and where it goes. Unchecked conversations and “Skip” targets stay in Slack only.`}
    >
      <Link
        to="/admin/slack/runs/$runId"
        params={{ runId: `${runId}` }}
        className="settings-classic-link slack-back"
      >
        <Icon name="chevron-left" size={14} />
        Back to the dry run
      </Link>
      {load.status === "loading" ? <PaneListSkeleton rows={4} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {load.status === "ready" ? <PlanForm key={runId} plan={load.plan} /> : null}
    </SettingsPage>
  );
}
