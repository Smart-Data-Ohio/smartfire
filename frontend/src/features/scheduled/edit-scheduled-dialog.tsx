import { type FormEvent, useId, useState } from "react";
import type { ScheduledMessage } from "../../gen/ScheduledMessage.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { TextField } from "../../ui/text-field.tsx";
import {
  customTimeProblem,
  fromLocalInput,
  schedulePresets,
  sendAtLabel,
  toLocalInput,
} from "../composer/schedule/presets.ts";

/** What the dialog saves: only the fields that changed (`null` keeps the server's value). */
export interface ScheduledEdit {
  readonly markdownSource: string | null;
  readonly sendAt: string | null;
}

interface EditScheduledDialogProps {
  /** The message being edited; `null` closes the dialog. */
  readonly item: ScheduledMessage | null;
  readonly onClose: () => void;
  /** Resolves when it saved (the dialog closes); a rejection's message shows in the form. */
  readonly onSave: (item: ScheduledMessage, edit: ScheduledEdit) => Promise<void>;
}

/**
 * Edits a pending scheduled message's text and send time together: the Markdown in a text area,
 * the time as the composer's custom picker has it, with its presets as quick picks.
 */
export function EditScheduledDialog({ item, onClose, onSave }: EditScheduledDialogProps) {
  const [shown, setShown] = useState<ScheduledMessage | null>(item);

  // Keep the last message while the dialog plays its exit.
  if (item !== null && item !== shown) {
    setShown(item);
  }

  return (
    <Dialog
      open={item !== null}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      title="Edit scheduled message"
      size="md"
    >
      {shown === null ? null : (
        <EditForm key={shown.id} item={shown} onClose={onClose} onSave={onSave} />
      )}
    </Dialog>
  );
}

interface EditFormProps {
  readonly item: ScheduledMessage;
  readonly onClose: () => void;
  readonly onSave: (item: ScheduledMessage, edit: ScheduledEdit) => Promise<void>;
}

function EditForm({ item, onClose, onSave }: EditFormProps) {
  const textId = useId();
  const [text, setText] = useState(item.markdownSource);
  const [when, setWhen] = useState(() => toLocalInput(new Date(item.sendAt)));
  const [error, setError] = useState<string | undefined>(undefined);
  const [timeError, setTimeError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);
  const at = fromLocalInput(when);
  const now = new Date();
  const timeChanged = when !== toLocalInput(new Date(item.sendAt));
  const textChanged = text !== item.markdownSource;

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (text.trim() === "") {
      setError("The message can't be empty.");

      return;
    }

    const problem = timeChanged ? customTimeProblem(at, new Date()) : null;

    if (problem !== null || at === null) {
      setTimeError(problem ?? "Pick a date and time.");
      setAttempt((count) => count + 1);

      return;
    }

    if (!textChanged && !timeChanged) {
      onClose();

      return;
    }

    setBusy(true);
    onSave(item, {
      markdownSource: textChanged ? text : null,
      sendAt: timeChanged ? at.toISOString() : null,
    }).then(onClose, (failure: Error) => {
      setBusy(false);
      setError(failure.message);
    });
  };

  return (
    <form className="scheduled-edit" onSubmit={submit}>
      <div>
        <label htmlFor={textId} className="scheduled-edit-label">
          Message
        </label>
        <textarea
          id={textId}
          className="scheduled-edit-text"
          value={text}
          data-autofocus
          onChange={(event) => {
            setText(event.target.value);
            setError(undefined);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
              event.currentTarget.form?.requestSubmit();
            }
          }}
        />
        {error === undefined ? null : (
          <p className="scheduled-edit-error" role="alert">
            {error}
          </p>
        )}
      </div>
      <div>
        <TextField
          label="Send at"
          type="datetime-local"
          value={when}
          min={toLocalInput(now)}
          required
          error={timeError}
          attempt={attempt}
          hint={
            at === null
              ? "Pick a day and a time."
              : `Sends ${sendAtLabel(at, now).replace(/^T/, "t")}.`
          }
          onChange={(event) => {
            setWhen(event.target.value);
            setTimeError(undefined);
          }}
        />
        <fieldset className="scheduled-presets">
          <legend className="visually-hidden">Quick times</legend>
          {schedulePresets(now).map((preset) => (
            <Button
              key={preset.id}
              variant="pill"
              size="sm"
              aria-pressed={when === toLocalInput(preset.at)}
              onClick={() => {
                setWhen(toLocalInput(preset.at));
                setTimeError(undefined);
              }}
            >
              {preset.label}
            </Button>
          ))}
        </fieldset>
      </div>
      <div className="scheduled-edit-actions">
        <Button variant="secondary" onClick={onClose}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" loading={busy} loadingLabel="Saving">
          Save changes
        </Button>
      </div>
    </form>
  );
}
