import { type ReactNode, Suspense, useState } from "react";
import { lazyForUpdate as lazy } from "../../service-worker/lazy.ts";
import { useStore } from "../../store/store.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import type { IconName } from "../../ui/icons/icon.tsx";
import { useDirectFacts } from "./direct-facts.ts";

const AddPeopleDialog = lazy(() => import("./add-people-dialog.tsx"));

const RenameDirectDialog = lazy(() => import("./rename-direct-dialog.tsx"));

type HeaderDialog = "add" | "rename";

export interface DirectAction {
  readonly key: HeaderDialog;
  readonly icon: IconName;
  readonly label: string;
  readonly onSelect: () => void;
}

interface DirectActions {
  /** What the DM offers: header buttons on wide screens, menu items or rows on phones. */
  readonly actions: readonly DirectAction[];
  /** The dialogs the actions open; render them where they outlive a closing menu. */
  readonly dialogs: ReactNode;
}

/**
 * A DM's people actions: "Add people" (a one-to-one becomes a new group DM, a group grows in
 * place) and, for a group DM, "Rename". Each opens its dialog, loaded on first use. A
 * note-to-self has neither.
 */
export function useDirectActions(roomId: number): DirectActions {
  const facts = useDirectFacts(roomId);
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  const [open, setOpen] = useState<HeaderDialog | null>(null);
  const [seen, setSeen] = useState<ReadonlySet<HeaderDialog>>(new Set());

  if (open !== null && !seen.has(open)) {
    setSeen(new Set([...seen, open]));
  }

  if (facts === null) {
    return { actions: [], dialogs: null };
  }

  const noteToSelf = facts.memberIds.length === 1 && facts.memberIds[0] === viewerId;
  const groupCapable = facts.memberIds.length > 1 || facts.name !== null;

  if (noteToSelf) {
    return { actions: [], dialogs: null };
  }

  const close = (dialog: HeaderDialog) => (next: boolean) => {
    if (!next) {
      setOpen((current) => (current === dialog ? null : current));
    }
  };

  const add: DirectAction = {
    key: "add",
    icon: "user-plus",
    label: "Add people",
    onSelect: () => setOpen("add"),
  };

  const rename: DirectAction = {
    key: "rename",
    icon: "pencil",
    label: "Rename conversation",
    onSelect: () => setOpen("rename"),
  };

  return {
    actions: groupCapable ? [add, rename] : [add],
    dialogs: (
      <>
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
    ),
  };
}

/** A DM's header actions on wide screens: one icon button per action. */
export function DirectHeaderActions({ roomId }: { readonly roomId: number }) {
  const { actions, dialogs } = useDirectActions(roomId);

  return (
    <>
      {actions.map((action) => (
        <IconButton
          key={action.key}
          icon={action.icon}
          label={action.label}
          tooltipPlacement="bottom"
          onClick={action.onSelect}
        />
      ))}
      {dialogs}
    </>
  );
}
