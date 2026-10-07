import { useCallback, useEffect, useState } from "react";
import type { HealthIssue } from "../../gen/HealthIssue.ts";
import type { IntegrationsHealth } from "../../gen/IntegrationsHealth.ts";
import { admin } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { SettingsGroup, SettingsPage } from "../settings/settings-parts.tsx";
import { type HealthFact, healthFacts } from "./admin-format.ts";
import { AdministratorsOnly, useAdmin } from "./admin-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly health: IntegrationsHealth };

/** A section's facts as the classic page's definition list. */
function Facts({ facts }: { readonly facts: readonly HealthFact[] }) {
  return (
    <dl className="admin-facts">
      {facts.map(([label, value]) => (
        <div key={label} className="admin-fact">
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

/** A list of problems under a heading, or nothing when there are none. */
function Issues({
  title,
  issues,
}: {
  readonly title: string;
  readonly issues: readonly HealthIssue[];
}) {
  if (issues.length === 0) {
    return null;
  }

  return (
    <div className="admin-issues">
      <h3 className="settings-label">{title}</h3>
      <ul className="settings-list">
        {issues.map((issue, index) => (
          <li key={`${issue.subject}-${index}`} className="settings-list-row">
            <span className="settings-list-main">
              <strong>{issue.subject}</strong>
              <span className="text-faint">{issue.detail}</span>
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/**
 * Integration health: what the classic health page reports for GitHub, Google Calendar, Fizzy,
 * webhook and agent delivery, and email to room, in its words.
 */
export function IntegrationsSection() {
  const { workspace } = useAdmin();
  const [load, setLoad] = useState<Load>({ status: "loading" });

  const fetchHealth = useCallback(() => {
    admin.integrationsHealth().then(
      (health) => setLoad({ status: "ready", health }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(() => {
    if (workspace.canAdminister) fetchHealth();
  }, [fetchHealth, workspace.canAdminister]);

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  const reload = () => {
    setLoad({ status: "loading" });
    fetchHealth();
  };

  const facts = load.status === "ready" ? healthFacts(load.health) : null;

  return (
    <SettingsPage title="Integration health">
      {load.status === "loading" ? <PaneListSkeleton rows={4} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {load.status === "ready" && facts !== null ? (
        <>
          <SettingsGroup title="GitHub">
            <Facts facts={facts.github} />
            <Issues title="Disconnected accounts" issues={load.health.github.disconnected} />
            <Issues title="Recent account errors" issues={load.health.github.lastErrors} />
            <Issues title="PR fetch errors" issues={load.health.github.fetchErrors} />
          </SettingsGroup>
          <SettingsGroup title="Google Calendar">
            <Facts facts={facts.google} />
            <Issues title="Disconnected accounts" issues={load.health.google.disconnected} />
            <Issues title="Calendar entry errors" issues={load.health.google.entryErrors} />
            <Issues title="Push channels expiring within 24 hours" issues={facts.expiring} />
          </SettingsGroup>
          <SettingsGroup title="Fizzy">
            <p>{facts.fizzy}</p>
          </SettingsGroup>
          <SettingsGroup title="Webhook and agent delivery">
            <Facts facts={facts.delivery} />
            <Issues
              title="Recent delivery errors"
              issues={load.health.agentDelivery.recentErrors}
            />
          </SettingsGroup>
          <SettingsGroup title="Email to room">
            <p>{facts.email}</p>
          </SettingsGroup>
          <div className="settings-actions">
            <Button variant="secondary" icon="rotate-ccw" onClick={reload}>
              Check again
            </Button>
          </div>
        </>
      ) : null}
    </SettingsPage>
  );
}
