import { type FormEvent, useState } from "react";
import type { UpdateCalls } from "../../gen/UpdateCalls.ts";
import { settings as settingsActions } from "../../sync/settings.ts";
import { Button } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { fieldError, VOICE_CHOICES } from "./settings-format.ts";
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

/** Calls: the microphone mode (saved as it changes) and the push-to-talk key. */
export function CallsSection() {
  const { settings, replace } = useSettings();
  const { calls } = settings;
  const [key, setKey] = useState(calls.pushToTalkKey ?? "");
  const [busy, setBusy] = useState<string | null>(null);
  const [fields, setFields] = useState<Fields>({});

  const save = (what: string, change: Partial<UpdateCalls>, done: string | null) => {
    setBusy(what);
    settingsActions
      .updateCalls(change)
      .then(
        (next) => {
          replace(next);
          setFields({});
          setKey(next.calls.pushToTalkKey ?? "");

          if (done !== null) {
            toast({ title: done, tone: "success" });
          }
        },
        (error: Error) => {
          const named = fieldsOf(error);

          setFields(named);

          if (Object.keys(named).length === 0) {
            toastFailure("Couldn't save your call settings", error);
          }
        },
      )
      .finally(() => setBusy(null));
  };

  const saveKey = (event: FormEvent) => {
    event.preventDefault();
    save("key", { pushToTalkKey: key }, "Push-to-talk key saved");
  };

  return (
    <SettingsPage title="Calls" description="How your microphone behaves in huddles.">
      <SettingsGroup
        title="Microphone"
        description="With voice activity your microphone is live whenever you are unmuted. With push-to-talk it opens only while you hold the key, never while you are typing."
      >
        <SettingsSelect
          label="Microphone mode"
          value={calls.voiceMode}
          choices={VOICE_CHOICES}
          disabled={busy === "mode"}
          onChange={(voiceMode) => save("mode", { voiceMode }, null)}
        />
        <FieldError message={fieldError(fields, "voiceMode", "Microphone mode")} />
        <form className="settings-form" onSubmit={saveKey} noValidate>
          <TextField
            label="Push-to-talk key"
            value={key}
            maxLength={20}
            autoComplete="off"
            placeholder="`"
            error={fieldError(fields, "pushToTalkKey", "Push-to-talk key")}
            onChange={(event) => setKey(event.target.value)}
          />
          <div className="settings-actions">
            <Button
              type="submit"
              loading={busy === "key"}
              disabled={busy !== null || key === (calls.pushToTalkKey ?? "")}
            >
              Save key
            </Button>
          </div>
        </form>
      </SettingsGroup>
    </SettingsPage>
  );
}
