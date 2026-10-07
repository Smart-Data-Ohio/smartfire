import { type FormEvent, useId, useMemo, useState } from "react";
import type { Settings } from "../../gen/Settings.ts";
import type { UpdateNotifications } from "../../gen/UpdateNotifications.ts";
import { useStore } from "../../store/store.ts";
import { settings as settingsActions } from "../../sync/settings.ts";
import { Button } from "../../ui/button.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Toggle } from "../../ui/toggle.tsx";
import { useCandidates } from "../directs/use-candidates.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { fieldError, keywordLines, MAX_KEYWORDS } from "./settings-format.ts";
import {
  FieldError,
  fieldsOf,
  SettingsGroup,
  SettingsPage,
  toastFailure,
  useSettings,
} from "./settings-parts.tsx";

type Fields = Readonly<Record<string, readonly string[]>>;

/** People who may reach the viewer through DND: the list with Remove, and a search to add one. */
function DndExceptions() {
  const { settings, replace } = useSettings();
  const [query, setQuery] = useState("");
  const [adding, setAdding] = useState(false);
  const [busy, setBusy] = useState<number | null>(null);
  const searchId = useId();
  const users = useStore((state) => state.users);
  const viewerId = settings.profile.userId;
  const { candidates } = useCandidates(adding);
  const allowed = settings.notifications.allowedPeople;

  const matches = useMemo(() => {
    const taken = new Set(allowed.map((person) => person.userId));
    const needle = query.trim().toLowerCase();

    return (candidates ?? [])
      .filter((candidate) => !candidate.agent && candidate.userId !== viewerId)
      .filter((candidate) => !taken.has(candidate.userId))
      .map((candidate) => ({ userId: candidate.userId, name: users[candidate.userId]?.name ?? "" }))
      .filter((person) => person.name !== "" && person.name.toLowerCase().includes(needle))
      .slice(0, 6);
  }, [allowed, candidates, query, users, viewerId]);

  const change = (userId: number, name: string, allow: boolean) => {
    setBusy(userId);
    settingsActions
      .setDndAllowance(userId, allow)
      .then(
        (next: Settings) => {
          replace(next);

          if (allow) {
            setQuery("");
            setAdding(false);
          }
        },
        (error: Error) =>
          toastFailure(allow ? `Couldn't add ${name}` : `Couldn't remove ${name}`, error),
      )
      .finally(() => setBusy(null));
  };

  return (
    <div className="settings-exceptions">
      <p className="settings-label">Still notify me for</p>
      {allowed.length === 0 ? (
        <p className="text-muted">Nobody yet. Starred people get through DND.</p>
      ) : (
        <ul className="settings-list">
          {allowed.map((person) => (
            <li key={person.userId} className="settings-list-row">
              <UserAvatar userId={person.userId} size={24} decorative />
              <span className="settings-list-main">{person.name}</span>
              <Button
                variant="ghost"
                size="sm"
                loading={busy === person.userId}
                aria-label={`Remove ${person.name} from DND exceptions`}
                onClick={() => change(person.userId, person.name, false)}
              >
                Remove
              </Button>
            </li>
          ))}
        </ul>
      )}
      {adding ? (
        <div className="settings-exception-search">
          <label htmlFor={searchId} className="settings-label">
            Add someone
          </label>
          <input
            id={searchId}
            type="search"
            className="input settings-select"
            value={query}
            placeholder="Search people"
            autoComplete="off"
            // biome-ignore lint/a11y/noAutofocus: the field opens on the person's own click
            autoFocus
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                setAdding(false);
              }
            }}
          />
          {candidates === null ? (
            <p className="text-faint">Loading people…</p>
          ) : (
            <ul className="settings-list" aria-label="People to add">
              {matches.map((person) => (
                <li key={person.userId}>
                  <button
                    type="button"
                    className="settings-pick"
                    disabled={busy !== null}
                    onClick={() => change(person.userId, person.name, true)}
                  >
                    <UserAvatar userId={person.userId} size={24} decorative />
                    <span>{person.name}</span>
                  </button>
                </li>
              ))}
              {matches.length === 0 ? <li className="text-faint">No one matches.</li> : null}
            </ul>
          )}
        </div>
      ) : (
        <div className="settings-actions">
          <Button variant="secondary" size="sm" icon="user-plus" onClick={() => setAdding(true)}>
            Add someone
          </Button>
        </div>
      )}
    </div>
  );
}

/**
 * Notifications: Do not disturb (manual, quiet hours, meetings, out of office) with the people
 * who still get through, keyword alerts, and which updates land in the activity inbox. Switches
 * save as they flip; times save as they change; keywords save with their button.
 */
export function NotificationsSection() {
  const { settings, replace } = useSettings();
  const { notifications } = settings;
  const [busy, setBusy] = useState<string | null>(null);
  const [keywords, setKeywords] = useState(notifications.keywordAlerts.join("\n"));
  const [fields, setFields] = useState<Fields>({});

  const save = (key: string, change: Partial<UpdateNotifications>, done: string | null = null) => {
    setBusy(key);
    settingsActions
      .updateNotifications(change)
      .then(
        (next) => {
          replace(next);
          setFields({});

          if (done !== null) {
            toast({ title: done, tone: "success" });
          }
        },
        (error: Error) => {
          const named = fieldsOf(error);

          setFields(named);

          if (Object.keys(named).length === 0) {
            toastFailure("Couldn't save your notification settings", error);
          }
        },
      )
      .finally(() => setBusy(null));
  };

  const saveKeywords = (event: FormEvent) => {
    event.preventDefault();

    const lines = keywordLines(keywords);

    save("keywords", { keywordAlerts: lines }, "Keyword alerts saved");
    setKeywords(lines.join("\n"));
  };

  const keywordCount = keywordLines(keywords).length;

  return (
    <SettingsPage
      title="Notifications"
      description="When Smartfire pings you, and what it records."
    >
      <SettingsGroup
        title="Do not disturb"
        description="While DND is on, push notifications and sounds stay silent. Your inbox still records everything."
      >
        <Toggle
          checked={notifications.dndEnabled}
          disabled={busy === "dnd"}
          label="Do not disturb"
          description="Silence push and sounds until you turn this off."
          onCheckedChange={(dndEnabled) => save("dnd", { dndEnabled })}
        />
        <Toggle
          checked={notifications.quietHoursEnabled}
          disabled={busy === "quiet"}
          label="Quiet hours"
          description="Scheduled DND every day, in your time zone."
          onCheckedChange={(quietHoursEnabled) => save("quiet", { quietHoursEnabled })}
        />
        <div className="settings-inline">
          <div className="settings-field">
            <label htmlFor="settings-quiet-start" className="settings-label">
              Quiet from
            </label>
            <input
              id="settings-quiet-start"
              type="time"
              className="input settings-select"
              defaultValue={notifications.quietHoursStart ?? ""}
              disabled={busy === "quiet-start"}
              onBlur={(event) => {
                if (event.target.value !== (notifications.quietHoursStart ?? "")) {
                  save("quiet-start", { quietHoursStart: event.target.value });
                }
              }}
            />
          </div>
          <div className="settings-field">
            <label htmlFor="settings-quiet-end" className="settings-label">
              Until
            </label>
            <input
              id="settings-quiet-end"
              type="time"
              className="input settings-select"
              defaultValue={notifications.quietHoursEnd ?? ""}
              disabled={busy === "quiet-end"}
              onBlur={(event) => {
                if (event.target.value !== (notifications.quietHoursEnd ?? "")) {
                  save("quiet-end", { quietHoursEnd: event.target.value });
                }
              }}
            />
          </div>
        </div>
        <FieldError
          message={
            fieldError(fields, "quietHoursStart", "Quiet from") ??
            fieldError(fields, "quietHoursEnd", "Until")
          }
        />
        <Toggle
          checked={notifications.meetingDndEnabled}
          disabled={busy === "meeting"}
          label="Do not disturb during meetings"
          description={`Silence push and sounds while you're in a meeting. Only works while "Show when I'm in a meeting" is on; starred people still get through.`}
          onCheckedChange={(meetingDndEnabled) => save("meeting", { meetingDndEnabled })}
        />
        <Toggle
          checked={notifications.oooNotifyEnabled}
          disabled={busy === "ooo"}
          label="Keep notifying me while I'm out of office"
          description="Off means push, sounds and huddle rings stay silent while you're out of office, like DND. Starred people still get through."
          onCheckedChange={(oooNotifyEnabled) => save("ooo", { oooNotifyEnabled })}
        />
        <DndExceptions />
      </SettingsGroup>

      <SettingsGroup
        title="Keyword alerts"
        description={`One word or phrase per line, up to ${MAX_KEYWORDS}. A match in any room you belong to lands in your inbox.`}
      >
        <form className="settings-form" onSubmit={saveKeywords}>
          <label htmlFor="settings-keywords" className="settings-label">
            Keywords
          </label>
          <textarea
            id="settings-keywords"
            className="input settings-textarea"
            rows={4}
            value={keywords}
            autoComplete="off"
            placeholder={"production\ndeploy freeze"}
            onChange={(event) => setKeywords(event.target.value)}
          />
          <p
            className="settings-hint text-faint"
            data-over={keywordCount > MAX_KEYWORDS || undefined}
          >
            {keywordCount} of {MAX_KEYWORDS}
          </p>
          <FieldError message={fieldError(fields, "keywordAlerts", "Keyword alerts")} />
          <div className="settings-actions">
            <Button type="submit" loading={busy === "keywords"} disabled={busy !== null}>
              Save keywords
            </Button>
          </div>
        </form>
      </SettingsGroup>

      <SettingsGroup
        title="Activity inbox"
        description="Choose which updates land in your activity inbox. Turning one off never hides the page where the work lives."
      >
        {notifications.inbox.map((entry) => (
          <Toggle
            key={entry.key}
            checked={entry.enabled}
            disabled={busy === `inbox-${entry.key}`}
            label={entry.label}
            description={entry.description}
            onCheckedChange={(enabled) =>
              save(`inbox-${entry.key}`, { inbox: { [entry.key]: enabled } })
            }
          />
        ))}
        <FieldError message={fieldError(fields, "inbox", "Inbox")} />
      </SettingsGroup>
    </SettingsPage>
  );
}
