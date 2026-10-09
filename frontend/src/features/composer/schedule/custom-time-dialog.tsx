import { type FormEvent, useEffect, useState } from "react";
import { Button } from "../../../ui/button.tsx";
import { Dialog } from "../../../ui/dialog.tsx";
import { TextField } from "../../../ui/text-field.tsx";
import {
  customTimeProblem,
  fromLocalInput,
  schedulePresets,
  sendAtLabel,
  toLocalInput,
} from "./presets.ts";

interface CustomTimeDialogProps {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** "Schedule message" or "Reschedule message". */
  readonly title: string;
  /** Where the picker starts; tomorrow morning when absent. */
  readonly initial?: Date | null;
  readonly confirmLabel: string;
  /** Resolves when it worked (the dialog closes); a rejection's message shows in the field. */
  readonly onConfirm: (at: Date) => Promise<void>;
}

/** A date-and-time picker for a scheduled send (the custom preset, and rescheduling). */
export function CustomTimeDialog(props: CustomTimeDialogProps) {
  // Mount the form fresh for each opening, so it starts from `initial` every time.
  const [opened, setOpened] = useState(0);
  const [wasOpen, setWasOpen] = useState(props.open);
  const [dirty, setDirty] = useState(false);

  if (props.open !== wasOpen) {
    setWasOpen(props.open);

    if (props.open) {
      setOpened((count) => count + 1);
    }
  }

  return (
    <Dialog
      open={props.open}
      onOpenChange={props.onOpenChange}
      title={props.title}
      size="sm"
      dirty={dirty}
    >
      <CustomTimeForm key={opened} {...props} onDirty={setDirty} />
    </Dialog>
  );
}

interface CustomTimeFormProps extends CustomTimeDialogProps {
  /** Whether the time differs from the one the picker started at. */
  readonly onDirty: (dirty: boolean) => void;
}

function CustomTimeForm({
  initial,
  confirmLabel,
  onConfirm,
  onOpenChange,
  onDirty,
}: CustomTimeFormProps) {
  const [start] = useState(() =>
    toLocalInput(initial ?? schedulePresets(new Date())[1]?.at ?? new Date()),
  );

  const [value, setValue] = useState(start);
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);
  const at = fromLocalInput(value);
  const now = new Date();
  const dirty = value !== start;

  useEffect(() => onDirty(dirty), [dirty, onDirty]);

  useEffect(() => () => onDirty(false), [onDirty]);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const problem = customTimeProblem(at, new Date());

    if (problem !== null || at === null) {
      setError(problem ?? "Pick a date and time.");
      setAttempt((count) => count + 1);

      return;
    }

    setBusy(true);
    onConfirm(at).then(
      () => onOpenChange(false),
      (failure: Error) => {
        setBusy(false);
        setError(failure.message);
        setAttempt((count) => count + 1);
      },
    );
  };

  return (
    <form className="schedule-form" onSubmit={submit}>
      <TextField
        label="Send at"
        type="datetime-local"
        value={value}
        min={toLocalInput(now)}
        required
        data-autofocus
        error={error}
        attempt={attempt}
        hint={
          at === null
            ? "Pick a day and a time."
            : `Sends ${sendAtLabel(at, now).replace(/^T/, "t")}.`
        }
        onChange={(event) => {
          setValue(event.target.value);
          setError(undefined);
        }}
      />
      <div className="schedule-form-actions" data-dialog-actions>
        <Button variant="secondary" onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" loading={busy} loadingLabel="Scheduling">
          {confirmLabel}
        </Button>
      </div>
    </form>
  );
}
