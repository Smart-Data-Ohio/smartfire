import { Link, useNavigate } from "@tanstack/react-router";
import { type FormEvent, useCallback, useEffect, useRef, useState } from "react";
import type { SlackSetup } from "../../gen/SlackSetup.ts";
import { slack } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import {
  AdministratorsOnly,
  adminFailure,
  Confirm,
  copy,
  keepDraft,
  takeDraft,
  useAdmin,
} from "../admin/admin-parts.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { FieldError, SettingsGroup, SettingsPage, useBusy } from "../settings/settings-parts.tsx";
import { activeRunSentence, adminConnectionSentence, CONFIRM, optional } from "./slack-format.ts";
import "../admin/admin.css";
import "./slack.css";
import { leftToConfirm, stoppedAtConfirmation } from "../auth/confirmation.ts";

/** Where the Client ID waits while the classic page confirms the password (never the secret). */
const DRAFT_KEY = "smartfire.draft.admin-slack-client-id";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly setup: SlackSetup };

interface Ask {
  readonly title: string;
  readonly message: string;
  readonly label: string;
  readonly danger: boolean;
  readonly run: () => void;
}

/** Step 1: the manifest to create the Slack app from. */
function Manifest({ manifest }: { readonly manifest: string }) {
  const shown = useRef<HTMLElement>(null);

  return (
    <SettingsGroup
      title="1. Create the Slack app"
      description="Create an internal app from this manifest, then copy its Client ID and Client Secret below."
    >
      <ol className="slack-steps">
        <li>
          Open{" "}
          <a href="https://api.slack.com/apps" target="_blank" rel="noopener">
            api.slack.com/apps
          </a>{" "}
          and choose <strong>Create New App → From a manifest</strong>.
        </li>
        <li>Pick the Slack workspace, paste the manifest, and create the app.</li>
        <li>
          Open <strong>Basic Information</strong> and copy the <strong>Client ID</strong> and{" "}
          <strong>Client Secret</strong>.
        </li>
      </ol>
      <p className="settings-label">
        App manifest <span className="text-faint">(user scopes only — no bot user, no events)</span>
      </p>
      <section aria-label="App manifest">
        <pre
          className="slack-manifest"
          // biome-ignore lint/a11y/noNoninteractiveTabindex: the manifest scrolls; focus lets a keyboard scroll it
          tabIndex={0}
        >
          <code ref={shown}>{manifest}</code>
        </pre>
      </section>
      <div className="settings-actions">
        <Button icon="copy" onClick={() => copy(manifest, "Manifest", shown.current)}>
          Copy manifest
        </Button>
      </div>
    </SettingsGroup>
  );
}

/** Step 2: the Client ID and the write-only Client Secret. */
function Credentials({
  setup,
  onSaved,
}: {
  readonly setup: SlackSetup;
  readonly onSaved: (setup: SlackSetup) => void;
}) {
  const [clientId, setClientId] = useState(setup.clientId ?? "");
  const [secret, setSecret] = useState("");
  const [error, setError] = useState<string | undefined>();
  const { busy, track } = useBusy();
  // Whether the kept draft was taken: once, even when Strict Mode runs the effect twice.
  const taken = useRef(false);

  useEffect(() => {
    if (taken.current) {
      return;
    }

    taken.current = true;

    const kept = takeDraft(DRAFT_KEY);

    if (kept !== null) {
      setClientId(kept);
      toast({
        title: "Your Client ID is back",
        description: "Paste the secret again and save.",
      });
    }
  }, []);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setError(undefined);
    void track(
      "save",
      slack.saveCredentials({ clientId, clientSecret: optional(secret) }).then(
        (change) => {
          setSecret("");
          onSaved(change.setup);
          toast({ title: change.notice, tone: "success" });
        },
        (failure: Error) => {
          if (leftToConfirm(failure)) {
            keepDraft(DRAFT_KEY, clientId);
          }

          if (stoppedAtConfirmation(failure)) {
            return;
          }

          setError(failure.message);
        },
      ),
    );
  };

  return (
    <SettingsGroup
      title="2. Save credentials"
      description={
        setup.configured
          ? `Credentials saved${setup.configuredBy === null ? "" : ` by ${setup.configuredBy}`}. The secret is write-only: paste a new one to replace it.`
          : "Paste the Client ID and Client Secret from the app's Basic Information page."
      }
    >
      <form className="settings-form" onSubmit={submit}>
        <FieldError message={error} />
        <TextField
          label="Client ID"
          value={clientId}
          required
          autoComplete="off"
          onChange={(event) => setClientId(event.target.value)}
        />
        <TextField
          label="Client Secret (never shown again)"
          type="password"
          value={secret}
          autoComplete="new-password"
          placeholder={setup.configured ? "Saved — paste a new secret to replace it" : "xoxp-…"}
          onChange={(event) => setSecret(event.target.value)}
        />
        <div className="settings-actions">
          <Button type="submit" variant="primary" loading={busy("save")} disabled={busy("save")}>
            {setup.configured ? "Save new credentials" : "Save credentials"}
          </Button>
        </div>
      </form>
    </SettingsGroup>
  );
}

/** Step 4's form: a workspace dry run. */
function DryRun() {
  const navigate = useNavigate();
  const [includePrivate, setIncludePrivate] = useState(true);
  const [oldest, setOldest] = useState("");
  const [latest, setLatest] = useState("");
  const { busy, track } = useBusy();

  const submit = (event: FormEvent) => {
    event.preventDefault();
    void track(
      "dry-run",
      slack.dryRun({ includePrivate, oldest: optional(oldest), latest: optional(latest) }).then(
        (change) => {
          toast({ title: change.notice, tone: "success" });
          void navigate({
            to: "/admin/slack/runs/$runId",
            params: { runId: `${change.run.id}` },
          });
        },
        (error: Error) => adminFailure("Couldn't start the dry run", error),
      ),
    );
  };

  return (
    <form className="settings-form" onSubmit={submit}>
      <Checkbox
        checked={includePrivate}
        onCheckedChange={setIncludePrivate}
        label="Include private channels I'm in"
      />
      <div className="settings-inline admin-icon-names">
        <TextField
          label="Oldest message (optional)"
          type="date"
          value={oldest}
          onChange={(event) => setOldest(event.target.value)}
        />
        <TextField
          label="Latest message (optional)"
          type="date"
          value={latest}
          onChange={(event) => setLatest(event.target.value)}
        />
      </div>
      <div className="settings-actions">
        <Button type="submit" variant="primary" loading={busy("dry-run")} disabled={busy()}>
          Start dry run
        </Button>
      </div>
    </form>
  );
}

/**
 * Slack import: the classic setup page's four steps (the app manifest, the credentials, the
 * administrator's Slack connection, the dry run) and removing the credentials. Connecting goes
 * through Slack's OAuth on the classic routes and comes back to the classic page.
 */
export function SlackSetupSection() {
  const { workspace } = useAdmin();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [ask, setAsk] = useState<Ask | null>(null);
  const { busy, track } = useBusy();

  const fetchSetup = useCallback(() => {
    slack.setup().then(
      (setup) => setLoad({ status: "ready", setup }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(() => {
    if (workspace.canAdminister) fetchSetup();
  }, [fetchSetup, workspace.canAdminister]);

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  const reload = () => {
    setLoad({ status: "loading" });
    fetchSetup();
  };

  const replace = (setup: SlackSetup) => setLoad({ status: "ready", setup });

  const disconnect = () =>
    setAsk({
      title: "Disconnect Slack",
      message: CONFIRM.disconnectAdmin,
      label: "Disconnect Slack",
      danger: true,
      run: () =>
        void track(
          "disconnect",
          slack.disconnect().then(
            ({ notice }) => {
              toast({ title: notice, tone: "success" });
              fetchSetup();
            },
            (error: Error) => adminFailure("Couldn't disconnect Slack", error),
          ),
        ),
    });

  const remove = () =>
    setAsk({
      title: "Remove Slack credentials",
      message: CONFIRM.removeCredentials,
      label: "Remove Slack credentials",
      danger: true,
      run: () =>
        void track(
          "remove",
          slack.removeCredentials().then(
            (change) => {
              replace(change.setup);
              toast({ title: change.notice, tone: "success" });
            },
            (error: Error) => adminFailure("Couldn't remove the credentials", error),
          ),
        ),
    });

  const setup = load.status === "ready" ? load.setup : null;

  return (
    <SettingsPage
      title="Slack import"
      description="Move this workspace from Slack Pro to Smartfire: create an internal Slack app, connect it, dry-run the migration, then import."
    >
      {load.status === "loading" ? <PaneListSkeleton rows={4} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {setup === null ? null : (
        <>
          <Manifest manifest={setup.manifest} />
          <Credentials setup={setup} onSaved={replace} />
          <SettingsGroup title="3. Connect your Slack account">
            <p>{adminConnectionSentence(setup.connection, setup.teamName)}</p>
            <div className="settings-actions">
              <a
                className="button"
                data-variant={setup.connection.state === "connected" ? "secondary" : "primary"}
                href={setup.connectPath}
              >
                {setup.connection.state === "connected" ? "Reconnect Slack" : "Connect Slack"}
              </a>
              {setup.connection.state === "connected" ? (
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
          <SettingsGroup title="4. Dry run and import">
            {setup.activeRun !== null ? (
              <p>
                {activeRunSentence(setup.activeRun)}{" "}
                <Link to="/admin/slack/runs/$runId" params={{ runId: `${setup.activeRun.id}` }}>
                  View the run
                </Link>
                .
              </p>
            ) : setup.teamKnown && setup.connection.state === "connected" ? (
              <>
                <p>
                  Dry runs write nothing: review the plan, then run a scoped test import (undoable)
                  before the full import.
                </p>
                <DryRun />
              </>
            ) : (
              <p>
                Finish steps 2 and 3 first: the dry run needs saved credentials and your connected
                Slack account.
              </p>
            )}
            <div className="settings-actions">
              <Link to="/admin/slack/runs" className="button" data-variant="secondary">
                All import runs
              </Link>
            </div>
          </SettingsGroup>
          {setup.configured ? (
            <SettingsGroup
              title="Remove Slack credentials"
              description="Deletes the app credentials and every member's Slack connection. Run history stays. Blocked while a run is active."
            >
              <div className="settings-actions">
                <Button
                  variant="danger"
                  loading={busy("remove")}
                  disabled={busy()}
                  onClick={remove}
                >
                  Remove Slack credentials
                </Button>
              </div>
            </SettingsGroup>
          ) : null}
        </>
      )}
      <Confirm ask={ask} onCancel={() => setAsk(null)} />
    </SettingsPage>
  );
}
