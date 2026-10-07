import type { GoogleIntegration } from "../../gen/GoogleIntegration.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { classicPage, connectionSummary } from "./settings-format.ts";
import { SettingsGroup, SettingsPage, useSettings } from "./settings-parts.tsx";

/** A link to the classic page, which loads in full (connecting goes through OAuth or a token). */
export function ClassicLink({
  href,
  children,
}: {
  readonly href: string;
  readonly children: string;
}) {
  return (
    <a className="settings-classic-link" href={href}>
      {children}
      <Icon name="external-link" size={14} />
    </a>
  );
}

/** Google Calendar, as the classic panel words it. */
function calendarSummary(google: GoogleIntegration): string {
  if (!google.calendarConfigured) {
    return "Google Calendar is not configured for this workspace.";
  }

  if (google.email === null) {
    return "Not connected. Connect it to publish events you're going to and show when you're in a meeting.";
  }

  if (google.connected && google.calendar) {
    return `Connected as ${google.email}${google.drive ? ", with Drive previews" : ""}.`;
  }

  return google.connected
    ? "Calendar permission needed: reconnect to publish events."
    : "Google rejected the connection: reconnect it.";
}

/**
 * Integrations: where Google, GitHub and Fizzy stand, and the Slack import. Connecting and
 * disconnecting go through OAuth or a pasted token, so they stay on the classic page for now;
 * each card links there (with `?classic=1`, so the new UI doesn't send the person back).
 */
export function IntegrationsSection() {
  const { settings } = useSettings();
  const { integrations } = settings;
  const { google } = integrations;
  const manage = (anchor: string) => classicPage(integrations.managePath, anchor);

  return (
    <SettingsPage
      title="Integrations"
      description="Accounts linked to yours. Connecting and disconnecting open the classic profile page."
    >
      {google.signInConfigured ? (
        <SettingsGroup title="Google sign-in">
          <p>
            {google.identityEmail === null
              ? "Link your Workspace Google account so you can sign in with Google."
              : `Linked to ${google.identityEmail}. You can sign in with Google.`}
          </p>
          <ClassicLink href={manage("google-sign-in-title")}>
            {google.identityEmail === null ? "Link Google account" : "Manage"}
          </ClassicLink>
        </SettingsGroup>
      ) : null}

      <SettingsGroup title="Google Calendar">
        <p>{calendarSummary(google)}</p>
        {google.calendarConfigured ? (
          <ClassicLink href={manage("google-calendar-title")}>
            {google.email === null ? "Connect Google Calendar" : "Manage"}
          </ClassicLink>
        ) : null}
      </SettingsGroup>

      <SettingsGroup
        title="GitHub"
        description="Comment and review from PR threads as yourself, and see cards for private repositories your account can read."
      >
        <p>{connectionSummary("GitHub", integrations.github)}</p>
        <ClassicLink href={manage("github-connection-title")}>
          {integrations.github.state === "connected" ? "Manage" : "Connect GitHub"}
        </ClassicLink>
      </SettingsGroup>

      <SettingsGroup
        title="Fizzy"
        description="Preview Fizzy cards in messages with your access, and create cards from messages."
      >
        <p>{connectionSummary("Fizzy", integrations.fizzy)}</p>
        <ClassicLink href={manage("fizzy-connection-title")}>
          {integrations.fizzy.state === "connected" ? "Manage" : "Connect Fizzy"}
        </ClassicLink>
      </SettingsGroup>

      <SettingsGroup title="Slack import">
        <p>Bring your Slack direct messages, group DMs and private channels into Smartfire.</p>
        <ClassicLink href={integrations.slackImportPath}>Import from Slack</ClassicLink>
      </SettingsGroup>
    </SettingsPage>
  );
}
