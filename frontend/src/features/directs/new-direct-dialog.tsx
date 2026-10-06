import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { directs, validationMessage } from "../../sync/directs.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Kbd } from "../../ui/kbd.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PeoplePicker } from "./people-picker.tsx";
import { directIntent } from "./picker.ts";
import { useCandidates } from "./use-candidates.ts";

interface NewDirectDialogProps {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

const SUBMIT_LABEL = {
  none: "Start conversation",
  direct: "Start conversation",
  group: "Start group conversation",
} as const;

/**
 * New message (⌘⇧K, or the "+" by Direct messages): pick one person for a DM or several for a
 * group DM, then Enter. An existing conversation with exactly those people opens instead of a
 * new one (the server matches on the member set).
 */
export default function NewDirectDialog({ open, onOpenChange }: NewDirectDialogProps) {
  const navigate = useNavigate();
  const [selected, setSelected] = useState<readonly number[]>([]);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const [wasOpen, setWasOpen] = useState(open);
  const { candidates, error } = useCandidates(open);
  const intent = directIntent(selected);

  if (open !== wasOpen) {
    setWasOpen(open);

    if (open) {
      setSelected([]);
      setBusy(false);
      setProblem(null);
    }
  }

  const submit = () => {
    if (intent === "none" || busy) {
      return;
    }

    setBusy(true);
    setProblem(null);
    directs.create(selected).then(
      (row) => {
        setBusy(false);
        void navigate({ to: "/r/$roomId", params: { roomId: row.room.id } });
        onOpenChange(false);
      },
      (failure: Error) => {
        const reason = validationMessage(failure);

        setBusy(false);

        if (reason === null) {
          toast({
            title: "Couldn't start the conversation",
            description: failure.message,
            tone: "danger",
          });
        } else {
          setProblem(reason);
        }
      },
    );
  };

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="New message"
      footer={
        <>
          <span className="directs-hint" aria-hidden="true">
            <Kbd keys={["⏎"]} /> to start
          </span>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            variant="primary"
            disabled={intent === "none"}
            loading={busy}
            loadingLabel="Starting…"
            onClick={submit}
          >
            {SUBMIT_LABEL[intent]}
          </Button>
        </>
      }
    >
      <PeoplePicker
        candidates={candidates}
        error={error}
        selected={selected}
        onSelectedChange={(next) => {
          setSelected(next);
          setProblem(null);
        }}
        onSubmit={submit}
        label="To:"
        placeholder="Type a name"
      />
      {problem === null ? null : (
        <p className="picker-note picker-error" role="alert">
          {problem}
        </p>
      )}
    </Dialog>
  );
}
