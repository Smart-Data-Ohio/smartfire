import { Link } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import type { OooPreset } from "../../gen/OooPreset.ts";
import type { Settings } from "../../gen/Settings.ts";
import type { StatusExpiry } from "../../gen/StatusExpiry.ts";
import type { UpdateStatus } from "../../gen/UpdateStatus.ts";
import { settings as settingsActions } from "../../sync/settings.ts";
import { Button } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Toggle } from "../../ui/toggle.tsx";
import {
  EXPIRY_CHOICES,
  fieldError,
  OOO_CHOICES,
  oooSummary,
  PRESENCE_CHOICES,
  statusExpiry,
} from "./settings-format.ts";
import {
  FieldError,
  fieldsOf,
  SettingsGroup,
  SettingsPage,
  SettingsSelect,
  toastFailure,
  useSettings,
} from "./settings-parts.tsx";

type Fields = Readonly<Record<string, readonly string[]>>;

/** Whether the person's Google account can read their calendar (the meeting and OOO switches). */
function calendarReady(settings: Settings): boolean {
  const { google } = settings.integrations;

  return google.calendarConfigured && google.connected && google.calendar;
}

/** Where a calendar switch can't show: why, and where to fix it. */
function CalendarHint({ on, what }: { readonly on: boolean; readonly what: string }) {
  const { settings } = useSettings();

  if (!settings.integrations.google.calendarConfigured) {
    return <p className="text-muted">Google Calendar is not configured for this workspace.</p>;
  }

  return (
    <p className="text-muted">
      {on ? `${what} is on, but Google Calendar isn't connected. ` : ""}
      <Link to="/settings/integrations" className="settings-link">
        {on ? "Reconnect Google Calendar" : "Connect Google Calendar"}
      </Link>{" "}
      {on
        ? "to resume it."
        : "first: only free/busy times and event types are read, never titles or attendees."}
    </p>
  );
}

/**
 * Status: presence, the custom status and when it clears, the meeting label, and out of office
 * (a preset or custom end, a note, and the calendar's out-of-office events). The classic status
 * form's rules apply: clearing wins, and an out-of-office end must be ahead.
 */
export function StatusSection() {
  const { settings, replace } = useSettings();
  const { status } = settings;
  const [emoji, setEmoji] = useState(status.customStatusEmoji ?? "");
  const [text, setText] = useState(status.customStatusText ?? "");
  const [expiry, setExpiry] = useState<StatusExpiry>("never");
  const [preset, setPreset] = useState<OooPreset | "">("");
  const [custom, setCustom] = useState("");
  const [note, setNote] = useState(status.oooNote ?? "");
  const [fields, setFields] = useState<Fields>({});
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState<string | null>(null);
  const ready = calendarReady(settings);

  const save = (key: string, change: Partial<UpdateStatus>, done: string | null) => {
    setBusy(key);
    settingsActions
      .updateStatus(change)
      .then(
        (next) => {
          replace(next);
          setFields({});
          setEmoji(next.status.customStatusEmoji ?? "");
          setText(next.status.customStatusText ?? "");
          setNote(next.status.oooNote ?? "");
          setPreset("");
          setCustom("");

          if (done !== null) {
            toast({ title: done, tone: "success" });
          }
        },
        (error: Error) => {
          const named = fieldsOf(error);

          setFields(named);
          setAttempt((count) => count + 1);

          if (Object.keys(named).length === 0) {
            toastFailure("Couldn't update your status", error);
          }
        },
      )
      .finally(() => setBusy(null));
  };

  const saveCustom = (event: FormEvent) => {
    event.preventDefault();
    save(
      "custom",
      { customStatusEmoji: emoji, customStatusText: text, customStatusExpiresIn: expiry },
      "Status saved",
    );
  };

  const saveOoo = (event: FormEvent) => {
    event.preventDefault();
    save(
      "ooo",
      {
        oooPreset: preset === "" ? null : preset,
        oooUntilCustom: preset === "custom" ? custom : null,
        oooNote: note,
      },
      "Out of office saved",
    );
  };

  const summary = oooSummary(status);
  const expires = statusExpiry(status);

  return (
    <SettingsPage
      title="Status"
      description="Automatic shows you online while you're here and idle after 10 quiet minutes. Your custom status shows beside your name."
    >
      <SettingsGroup title="Presence">
        <SettingsSelect
          label="Presence"
          value={status.presenceSetting}
          choices={PRESENCE_CHOICES}
          disabled={busy === "presence"}
          onChange={(presenceSetting) => save("presence", { presenceSetting }, null)}
        />
        <FieldError message={fieldError(fields, "presenceSetting", "Presence")} />
      </SettingsGroup>

      <SettingsGroup title="Custom status" description={expires ?? undefined}>
        <form className="settings-form" onSubmit={saveCustom} noValidate>
          <div className="settings-inline">
            <TextField
              label="Emoji"
              className="settings-emoji"
              value={emoji}
              maxLength={8}
              placeholder="😀"
              autoComplete="off"
              onChange={(event) => setEmoji(event.target.value)}
            />
            <TextField
              label="Status text"
              value={text}
              maxLength={100}
              placeholder="Working from the train…"
              autoComplete="off"
              error={
                fieldError(fields, "customStatusText", "Custom status") ??
                fieldError(fields, "customStatusEmoji", "Custom status")
              }
              attempt={attempt}
              onChange={(event) => setText(event.target.value)}
            />
          </div>
          <SettingsSelect
            label="Clear after"
            value={expiry}
            choices={EXPIRY_CHOICES}
            onChange={setExpiry}
          />
          <div className="settings-actions">
            <Button type="submit" loading={busy === "custom"} disabled={busy !== null}>
              Save status
            </Button>
            <Button
              variant="secondary"
              loading={busy === "clear-custom"}
              disabled={
                busy !== null ||
                (status.customStatusEmoji === null && status.customStatusText === null)
              }
              onClick={() => save("clear-custom", { clearCustomStatus: true }, "Status cleared")}
            >
              Clear status
            </Button>
          </div>
        </form>
      </SettingsGroup>

      <SettingsGroup
        title="Meetings"
        description='Show "In a meeting" beside your name while your Google Calendar says you are busy. A custom status or DND shows instead.'
      >
        {ready ? (
          <>
            <Toggle
              checked={status.meetingStatusEnabled}
              disabled={busy === "meeting"}
              label="Show when I'm in a meeting"
              description={`Connected as ${settings.integrations.google.email ?? "your Google account"}.`}
              onCheckedChange={(meetingStatusEnabled) =>
                save("meeting", { meetingStatusEnabled }, null)
              }
            />
            {status.calendarError === null ? null : (
              <p className="text-muted">{status.calendarError}</p>
            )}
          </>
        ) : (
          <CalendarHint on={status.meetingStatusEnabled} what="Meeting status" />
        )}
      </SettingsGroup>

      <SettingsGroup
        title="Out of office"
        description='Show "Out of office" beside your name until the end you pick. It clears itself when the end passes, and wins over a custom status, DND and the meeting label.'
      >
        {summary === null ? null : <p className="settings-callout">{summary}</p>}
        <form className="settings-form" onSubmit={saveOoo} noValidate>
          <SettingsSelect
            label="Out until"
            value={preset}
            choices={OOO_CHOICES}
            onChange={setPreset}
          />
          {preset === "custom" ? (
            <div className="settings-field">
              <label htmlFor="settings-ooo-custom" className="settings-label">
                Custom end
              </label>
              <input
                id="settings-ooo-custom"
                type="datetime-local"
                className="input settings-select"
                value={custom}
                onChange={(event) => setCustom(event.target.value)}
              />
              <p className="settings-hint text-faint">In your time zone.</p>
            </div>
          ) : null}
          <FieldError message={fieldError(fields, "oooUntil", "Out of office")} />
          <TextField
            label="Note"
            value={note}
            maxLength={140}
            placeholder="Back soon, slow to reply…"
            autoComplete="off"
            error={fieldError(fields, "oooNote", "Note")}
            attempt={attempt}
            onChange={(event) => setNote(event.target.value)}
          />
          <div className="settings-actions">
            <Button type="submit" loading={busy === "ooo"} disabled={busy !== null}>
              Save out of office
            </Button>
            <Button
              variant="secondary"
              loading={busy === "clear-ooo"}
              disabled={busy !== null || status.oooUntil === null}
              onClick={() => save("clear-ooo", { clearOoo: true }, "Out of office cleared")}
            >
              Clear out of office
            </Button>
          </div>
        </form>
        {ready ? (
          <Toggle
            checked={status.oooCalendarEnabled}
            disabled={busy === "ooo-calendar"}
            label="Use my Google Calendar out-of-office"
            description="Out-of-office events mark you out for their span. Only the event type and times are read."
            onCheckedChange={(oooCalendarEnabled) =>
              save("ooo-calendar", { oooCalendarEnabled }, null)
            }
          />
        ) : (
          <CalendarHint on={status.oooCalendarEnabled} what="Calendar out-of-office" />
        )}
      </SettingsGroup>
    </SettingsPage>
  );
}
