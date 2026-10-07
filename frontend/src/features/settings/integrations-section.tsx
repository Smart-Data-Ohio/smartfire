import { Link } from "@tanstack/react-router";
import { type FormEvent, type ReactNode, useState } from "react";
import type { Connection } from "../../gen/Connection.ts";
import type { GoogleIntegration } from "../../gen/GoogleIntegration.ts";
import type { IntegrationChange } from "../../gen/IntegrationChange.ts";
import { postClassicForm } from "../../lib/classic-form.ts";
import { ActionError } from "../../sync/run.ts";
import { settings as settingsActions, type TokenService } from "../../sync/settings.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { needsSudo, SUDO_PAGE } from "../admin/admin-parts.tsx";
import { connectionSummary } from "./settings-format.ts";
import {
  SettingsGroup,
  SettingsPage,
  toastFailure,
  useBusy,
  useSettings,
} from "./settings-parts.tsx";

/** A service the profile can disconnect. */
type Service = TokenService | "google";

/** The service's name, as the classic panels and notices put it. */
const SERVICE_NAME = { github: "GitHub", fizzy: "Fizzy", google: "Google Calendar" } as const;

/** What each disconnect does, as the classic panels' `turbo_confirm` warns. */
const DISCONNECT_WARNING = {
  github: "You will no longer be able to comment or review from PR threads.",
  fizzy:
    "Card previews will stop working and you will no longer be able to create cards from messages.",
  google: "Your published event entries will be removed.",
} as const;

/**
 * A failed write: a lapsed password confirmation goes to the classic page that asks for it (it
 * comes back here; a pasted token isn't kept, so it's pasted again), anything else is a toast.
 */
function failed(title: string, error: Error): void {
  if (needsSudo(error)) {
    window.location.assign(SUDO_PAGE);

    return;
  }

  toastFailure(title, error);
}

/** `POST /google/connect`, a full page load to Google; `drive` asks for Drive previews too. */
function connectGoogle(drive: boolean): void {
  postClassicForm("/google/connect", drive ? [["features[]", "drive"]] : []);
}

/** Pastes a personal access token for GitHub or Fizzy, as the classic panel's form does. */
function TokenForm({
  service,
  linked,
  placeholder,
  hint,
  onConnected,
}: {
  readonly service: TokenService;
  readonly linked: boolean;
  readonly placeholder: string;
  readonly hint?: string;
  readonly onConnected: (change: IntegrationChange) => void;
}) {
  const [token, setToken] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [saving, setSaving] = useState(false);
  const name = SERVICE_NAME[service];
  const label = `${linked ? "Reconnect" : "Connect"} ${name}`;

  const submit = (event: FormEvent) => {
    event.preventDefault();

    if (saving) {
      return;
    }

    setSaving(true);
    settingsActions
      .connect(service, token)
      .then(
        (change) => {
          setToken("");
          setError(undefined);
          onConnected(change);
        },
        (failure: Error) => {
          if (failure instanceof ActionError && failure.tag === "Validation") {
            setError(failure.message);
            setAttempt((count) => count + 1);
          } else {
            failed(`Couldn't connect ${name}`, failure);
          }
        },
      )
      .finally(() => setSaving(false));
  };

  return (
    <form className="settings-token" onSubmit={submit} noValidate>
      <TextField
        label={`${name} personal access token`}
        type="password"
        value={token}
        placeholder={placeholder}
        autoComplete="off"
        spellCheck={false}
        {...(hint === undefined ? {} : { hint })}
        error={error}
        attempt={attempt}
        onChange={(event) => setToken(event.target.value)}
      />
      <Button type="submit" variant="primary" size="lg" loading={saving} loadingLabel="Checking…">
        {label}
      </Button>
    </form>
  );
}

/** A disconnect button; it asks the classic question before anything changes. */
function Disconnect({
  service,
  label,
  busy,
  onAsk,
}: {
  readonly service: Service;
  readonly label: string;
  readonly busy: boolean;
  readonly onAsk: (service: Service) => void;
}) {
  return (
    <Button variant="danger" loading={busy} onClick={() => onAsk(service)}>
      {label}
    </Button>
  );
}

/** Google Calendar, as the classic panel words and offers it. */
function GoogleCalendar({
  google,
  disconnect,
}: {
  readonly google: GoogleIntegration;
  readonly disconnect: ReactNode;
}) {
  if (!google.calendarConfigured) {
    return <p>Google Calendar is not configured for this workspace.</p>;
  }

  if (google.email === null) {
    return (
      <>
        <p>Publish the events you're going to, or might go to, on your Google Calendar.</p>
        <div className="settings-actions">
          <Button variant="primary" onClick={() => connectGoogle(false)}>
            Connect Google Calendar
          </Button>
        </div>
      </>
    );
  }

  if (google.connected && google.calendar) {
    return (
      <>
        <p>Connected as {google.email}.</p>
        {google.drive ? <p>Drive previews enabled.</p> : null}
        <div className="settings-actions">
          {google.drive ? null : (
            <Button onClick={() => connectGoogle(true)}>Enable Drive previews</Button>
          )}
          {disconnect}
        </div>
      </>
    );
  }

  return (
    <>
      <p>
        {google.connected
          ? "Calendar permission needed: reconnect to publish events."
          : "Google rejected the connection: reconnect it."}
      </p>
      <div className="settings-actions">
        <Button variant="primary" onClick={() => connectGoogle(google.drive)}>
          Connect Google Calendar
        </Button>
        {google.connected ? disconnect : null}
      </div>
    </>
  );
}

/** What GitHub is for, before it's connected or once it is (a rejected one says so instead). */
function githubPurpose(connection: Connection): ReactNode {
  switch (connection.state) {
    case "connected":
      return (
        <p>
          Comments and reviews you post from PR threads act as <strong>@{connection.name}</strong>{" "}
          on GitHub. Your linked account also unlocks cards for private repositories it can read.
        </p>
      );

    case "missing":
      return (
        <p>
          Connect GitHub to comment and review from PR threads as yourself. Linking also shows you
          cards for private repositories your account can read.
        </p>
      );

    case "rejected":
      return null;
  }
}

/** What Fizzy is for, before it's connected or once it is (a rejected one says so instead). */
function fizzyPurpose(connection: Connection): ReactNode {
  switch (connection.state) {
    case "connected":
      return (
        <p>
          Card links in messages preview with <strong>your</strong> Fizzy access, and cards you
          create from messages are created as <strong>{connection.name}</strong> on Fizzy.
        </p>
      );

    case "missing":
      return (
        <p>
          Paste a personal access token to preview Fizzy cards in messages and create cards from
          them. Generate one in Fizzy under your profile → API → Personal access tokens; card
          creation needs <strong>Read + Write</strong>. The token is checked with Fizzy before it is
          stored and is never shown again.
        </p>
      );

    case "rejected":
      return null;
  }
}

const GITHUB_TOKEN_HINT =
  "Needs Pull requests: Read and write, Issues: Read and write, and Metadata: Read. It is checked with GitHub before it is stored and is never shown again.";

/**
 * Integrations: Google sign-in and Calendar, GitHub and Fizzy, and the Slack import. Tokens are
 * pasted and connections dropped here, through the same code as the classic profile. Signing in
 * with Google or GitHub is a full page load to the provider and back, as on the classic page.
 */
export function IntegrationsSection() {
  const { settings, replace } = useSettings();
  const { integrations } = settings;
  const { google, github, fizzy } = integrations;
  const { busy, track } = useBusy();

  // The service stays put while the question closes, so its words don't blank mid-fade.
  const [ask, setAsk] = useState<{ readonly service: Service; readonly open: boolean }>({
    service: "github",
    open: false,
  });

  const githubLine = connectionSummary("GitHub", github);
  const fizzyLine = connectionSummary("Fizzy", fizzy);

  const landed = (change: IntegrationChange, service: Service) => {
    replace({ ...settings, integrations: change.integrations });
    toast({ title: change.notice, tone: "success" });

    // A GitHub link sets (and an unlink frees) the profile's GitHub username: reload the page so
    // the profile section shows it.
    if (service === "github") {
      void settingsActions.load().then(replace, () => undefined);
    }
  };

  const disconnect = (service: Service) => {
    setAsk({ service, open: false });
    void track(
      service,
      settingsActions.disconnect(service).then(
        (change) => landed(change, service),
        (error: Error) => failed(`Couldn't disconnect ${SERVICE_NAME[service]}`, error),
      ),
    );
  };

  const disconnectButton = (service: Service, label: string) => (
    <Disconnect
      service={service}
      label={label}
      busy={busy(service)}
      onAsk={(asked) => setAsk({ service: asked, open: true })}
    />
  );

  return (
    <SettingsPage
      title="Integrations"
      description="Accounts linked to yours, and the Slack import."
    >
      {google.signInConfigured ? (
        <SettingsGroup title="Google sign-in">
          {google.identityEmail === null ? (
            <>
              <p>
                Link your Workspace Google account so you can sign in with Google. You will confirm
                it with Google.
              </p>
              <div className="settings-actions">
                <Button onClick={() => postClassicForm("/user/profile/google_sign_in_link")}>
                  Link Google sign-in
                </Button>
              </div>
            </>
          ) : (
            <p>Linked to {google.identityEmail}. You can sign in with Google.</p>
          )}
        </SettingsGroup>
      ) : null}

      <SettingsGroup title="Google Calendar">
        <GoogleCalendar google={google} disconnect={disconnectButton("google", "Disconnect")} />
      </SettingsGroup>

      <SettingsGroup title="GitHub">
        {githubLine === null ? null : <p>{githubLine}</p>}
        {githubPurpose(github)}
        {github.state === "connected" ? (
          <div className="settings-actions">{disconnectButton("github", "Disconnect GitHub")}</div>
        ) : integrations.githubAppConfigured ? (
          <>
            <div className="settings-actions">
              <a className="button" data-variant="primary" href="/github/app/connect">
                {github.state === "missing" ? "Connect with GitHub" : "Reconnect with GitHub"}
              </a>
            </div>
            <details className="settings-details">
              <summary>Or paste a personal access token instead</summary>
              <TokenForm
                service="github"
                linked={github.state !== "missing"}
                placeholder="github_pat_…"
                hint={GITHUB_TOKEN_HINT}
                onConnected={(change) => landed(change, "github")}
              />
            </details>
          </>
        ) : (
          <TokenForm
            service="github"
            linked={github.state !== "missing"}
            placeholder="github_pat_…"
            hint={GITHUB_TOKEN_HINT}
            onConnected={(change) => landed(change, "github")}
          />
        )}
      </SettingsGroup>

      <SettingsGroup title="Fizzy">
        {fizzyLine === null ? null : <p>{fizzyLine}</p>}
        {fizzyPurpose(fizzy)}
        {fizzy.state === "connected" ? (
          <div className="settings-actions">{disconnectButton("fizzy", "Disconnect Fizzy")}</div>
        ) : (
          <TokenForm
            service="fizzy"
            linked={fizzy.state !== "missing"}
            placeholder="Fizzy token…"
            onConnected={(change) => landed(change, "fizzy")}
          />
        )}
      </SettingsGroup>

      <SettingsGroup title="Slack import">
        <p>Bring your Slack direct messages, group DMs and private channels into Smartfire.</p>
        <Link to="/settings/slack" className="settings-classic-link">
          Import from Slack
          <Icon name="chevron-right" size={14} />
        </Link>
      </SettingsGroup>

      <Dialog
        open={ask.open}
        onOpenChange={(open) => setAsk({ ...ask, open })}
        role="alertdialog"
        size="sm"
        title={`Disconnect ${SERVICE_NAME[ask.service]}?`}
        description={DISCONNECT_WARNING[ask.service]}
        footer={
          <>
            <Button
              variant="secondary"
              onClick={() => setAsk({ ...ask, open: false })}
              data-autofocus
            >
              Cancel
            </Button>
            <Button variant="danger" onClick={() => disconnect(ask.service)}>
              Disconnect
            </Button>
          </>
        }
      />
    </SettingsPage>
  );
}
