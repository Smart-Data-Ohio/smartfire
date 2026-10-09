import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useStore } from "../../store/store.ts";
import { directs, validationMessage } from "../../sync/directs.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { toast } from "../../ui/toast-store.ts";
import { useDirectFacts } from "./direct-facts.ts";
import { PeoplePicker } from "./people-picker.tsx";
import { addPlan, MAX_OTHERS } from "./picker.ts";
import { useCandidates } from "./use-candidates.ts";

interface AddPeopleDialogProps {
  readonly roomId: number;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

/**
 * Add people to a DM. A group DM grows in place (everyone sees a system note); a one-to-one
 * can't, so picking people there starts a new group conversation with all of you, and opens it.
 */
export default function AddPeopleDialog({ roomId, open, onOpenChange }: AddPeopleDialogProps) {
  const navigate = useNavigate();
  const facts = useDirectFacts(roomId);
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  const [selected, setSelected] = useState<readonly number[]>([]);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const [wasOpen, setWasOpen] = useState(open);
  const { candidates, error } = useCandidates(open);
  const memberIds = facts?.memberIds ?? [];
  const plan = addPlan(memberIds, facts?.name != null, selected);
  const excluded = new Set(viewerId === null ? memberIds : [...memberIds, viewerId]);

  if (open !== wasOpen) {
    setWasOpen(open);

    if (open) {
      setSelected([]);
      setBusy(false);
      setProblem(null);
    }
  }

  const fail = (failure: Error) => {
    const reason = validationMessage(failure);

    setBusy(false);

    if (reason === null) {
      toast({ title: "Couldn't add people", description: failure.message, tone: "danger" });
    } else {
      setProblem(reason);
    }
  };

  const changeSelected = (next: readonly number[]) => {
    setSelected(next);
    setProblem(null);
  };

  const submit = () => {
    if (selected.length === 0 || busy) {
      return;
    }

    setBusy(true);
    setProblem(null);

    if (plan.kind === "add") {
      directs.addMembers(roomId, plan.userIds).then(() => {
        setBusy(false);
        onOpenChange(false);
        toast({
          title: selected.length === 1 ? "Added 1 person" : `Added ${selected.length} people`,
          tone: "success",
        });
      }, fail);

      return;
    }

    directs.create(plan.userIds).then((row) => {
      setBusy(false);
      void navigate({ to: "/r/$roomId", params: { roomId: row.room.id } });
      onOpenChange(false);
    }, fail);
  };

  const description =
    plan.kind === "new-group"
      ? "This starts a new group conversation; the current one stays as it is."
      : "They'll see the conversation's history.";

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={`Add people to ${facts?.displayName ?? "this conversation"}`}
      dirty={selected.length > 0}
      description={description}
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            variant="primary"
            disabled={selected.length === 0}
            loading={busy}
            loadingLabel="Adding…"
            onClick={submit}
          >
            {plan.kind === "new-group" ? "Start group conversation" : "Add"}
          </Button>
        </>
      }
    >
      <PeoplePicker
        candidates={candidates}
        error={error}
        selected={selected}
        onSelectedChange={changeSelected}
        excluded={excluded}
        fixed={plan.kind === "new-group" ? memberIds : []}
        limit={Math.max(MAX_OTHERS - memberIds.length, 0)}
        onSubmit={submit}
        label="Add:"
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
