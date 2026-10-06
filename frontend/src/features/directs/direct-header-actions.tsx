import { lazy, Suspense, useState } from "react";
import { useStore } from "../../store/store.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { useDirectFacts } from "./direct-facts.ts";

const AddPeopleDialog = lazy(() => import("./add-people-dialog.tsx"));

const RenameDirectDialog = lazy(() => import("./rename-direct-dialog.tsx"));

type HeaderDialog = "add" | "rename";

/**
 * A DM's header actions: "Add people" (a one-to-one becomes a new group DM, a group grows in
 * place) and, for a group DM, "Rename". Each opens its dialog, loaded on first use. A
 * note-to-self has neither.
 */
export function DirectHeaderActions({ roomId }: { readonly roomId: number }) {
  const facts = useDirectFacts(roomId);
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  const [open, setOpen] = useState<HeaderDialog | null>(null);
  const [seen, setSeen] = useState<ReadonlySet<HeaderDialog>>(new Set());

  if (open !== null && !seen.has(open)) {
    setSeen(new Set([...seen, open]));
  }

  if (facts === null) {
    return null;
  }

  const noteToSelf = facts.memberIds.length === 1 && facts.memberIds[0] === viewerId;
  const groupCapable = facts.memberIds.length > 1 || facts.name !== null;

  if (noteToSelf) {
    return null;
  }

  const close = (dialog: HeaderDialog) => (next: boolean) => {
    if (!next) {
      setOpen((current) => (current === dialog ? null : current));
    }
  };

  return (
    <>
      <IconButton
        icon="user-plus"
        label="Add people"
        tooltipPlacement="bottom"
        onClick={() => setOpen("add")}
      />
      {groupCapable ? (
        <IconButton
          icon="pencil"
          label="Rename conversation"
          tooltipPlacement="bottom"
          onClick={() => setOpen("rename")}
        />
      ) : null}
      <Suspense fallback={null}>
        {seen.has("add") ? (
          <AddPeopleDialog roomId={roomId} open={open === "add"} onOpenChange={close("add")} />
        ) : null}
      </Suspense>
      <Suspense fallback={null}>
        {seen.has("rename") && groupCapable ? (
          <RenameDirectDialog
            roomId={roomId}
            open={open === "rename"}
            onOpenChange={close("rename")}
          />
        ) : null}
      </Suspense>
    </>
  );
}
