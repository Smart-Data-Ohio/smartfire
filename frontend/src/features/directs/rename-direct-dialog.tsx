import { type FormEvent, useId, useState } from "react";
import { directs } from "../../sync/directs.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { useDirectFacts } from "./direct-facts.ts";

interface RenameDirectDialogProps {
  readonly roomId: number;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

/** The server's limit on a direct room's name. */
const MAX_NAME = 100;

/**
 * Names a group DM for everyone in it; clearing the name goes back to the members' names. A
 * refusal shakes the field with the server's reason.
 */
export default function RenameDirectDialog({
  roomId,
  open,
  onOpenChange,
}: RenameDirectDialogProps) {
  const formId = useId();
  const facts = useDirectFacts(roomId);
  const [name, setName] = useState(facts?.name ?? "");
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);
  const [wasOpen, setWasOpen] = useState(open);

  if (open !== wasOpen) {
    setWasOpen(open);

    if (open) {
      setName(facts?.name ?? "");
      setError(undefined);
      setBusy(false);
    }
  }

  const unchanged = name.trim() === (facts?.name ?? "");

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (busy) {
      return;
    }

    if (unchanged) {
      onOpenChange(false);

      return;
    }

    setBusy(true);
    directs.rename(roomId, name).then(
      (detail) => {
        setBusy(false);
        onOpenChange(false);
        toast({
          title: detail.room.name === null ? "Name cleared" : `Renamed to ${detail.displayName}`,
          tone: "success",
        });
      },
      (failure: Error) => {
        setBusy(false);
        setError(failure.message);
        setAttempt((count) => count + 1);
      },
    );
  };

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Rename conversation"
      size="sm"
      dirty={!unchanged}
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            variant="primary"
            type="submit"
            form={formId}
            loading={busy}
            loadingLabel="Saving…"
          >
            Save
          </Button>
        </>
      }
    >
      <form id={formId} onSubmit={submit} className="rename-form">
        <TextField
          label="Name"
          hint="Everyone in the conversation sees it. Leave it blank to use members' names."
          placeholder={facts?.displayName ?? ""}
          value={name}
          maxLength={MAX_NAME}
          error={error}
          attempt={attempt}
          data-autofocus
          onChange={(event) => {
            setName(event.target.value);
            setError(undefined);
          }}
        />
      </form>
    </Dialog>
  );
}
