import type { ReactElement } from "react";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { store, useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Menu, MenuGroup, MenuRadioItem, type MenuTriggerProps } from "../../ui/menu.tsx";
import { toast } from "../../ui/toast-store.ts";
import { threadTitle } from "../threads/thread-format.ts";
import { WORK_STATUS_LABEL, WORK_STATUSES } from "./board-format.ts";
import { StatusChip } from "./board-parts.tsx";

/**
 * Whether the viewer may change a post's status, as the server decides it
 * (`work_manageable_in_room`): the board's creator or an administrator, the post's creator, or
 * its owner.
 */
export function useCanMovePost(roomId: number, threadId: number): boolean {
  return useStore((state) => {
    const viewer = state.me?.user.id ?? state.boot?.user.id ?? null;
    const thread = state.threads[threadId];

    if (viewer === null || thread === undefined || thread.work === null) {
      return false;
    }

    return (
      (state.boards[roomId]?.canAdminister ?? false) ||
      thread.creatorId === viewer ||
      thread.work.owner?.id === viewer
    );
  });
}

/** Moves a post to `to` (the card moves at once), then says where it went with a way back. */
export function movePost(threadId: number, to: WorkStatus): void {
  const from = store.getState().threads[threadId]?.work?.status;

  if (from === undefined || from === to) {
    return;
  }

  actions.work.setStatus(threadId, to).then(
    () =>
      toast({
        title: `Moved to ${WORK_STATUS_LABEL[to]}`,
        action: { label: "Undo", onClick: () => movePost(threadId, from) },
      }),
    (error: Error) =>
      toast({ title: "Couldn't move the post", description: error.message, tone: "danger" }),
  );
}

interface PostMenuProps {
  readonly threadId: number;
  readonly trigger: (props: MenuTriggerProps) => ReactElement;
  /** Opened by something other than the trigger too (a long press on the card). */
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

/**
 * A post's actions on the board: "Move to" each status, the current one checked. An action sheet
 * on a touch phone, headed by the post's title and status; a dropdown from its trigger elsewhere.
 */
export function PostMenu({ threadId, trigger, open, onOpenChange }: PostMenuProps) {
  const thread = useStore((state) => state.threads[threadId]);
  const status = thread?.work?.status;

  if (thread === undefined || status === undefined) {
    return null;
  }

  return (
    <Menu
      label={`${threadTitle(thread)} actions`}
      placement="bottom-end"
      open={open}
      onOpenChange={onOpenChange}
      trigger={trigger}
      sheetHeader={
        <div className="post-sheet-head">
          <span className="post-sheet-title">{threadTitle(thread)}</span>
          <StatusChip status={status} />
        </div>
      }
    >
      <MenuGroup label="Move to">
        {WORK_STATUSES.map((choice) => (
          <MenuRadioItem
            key={choice}
            checked={choice === status}
            label={WORK_STATUS_LABEL[choice]}
            onSelect={() => movePost(threadId, choice)}
          >
            <span className="post-menu-status" data-status={choice}>
              <span className="board-status-dot" aria-hidden="true" />
              {WORK_STATUS_LABEL[choice]}
            </span>
          </MenuRadioItem>
        ))}
      </MenuGroup>
    </Menu>
  );
}
