import { useStore } from "../../store/store.ts";
import { Badge } from "../../ui/badge.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { MenuItem } from "../../ui/menu.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import { raisedHandsLabel, StageButton, useRaisedHands } from "../huddle/stage-button.tsx";
import { AvatarGroup } from "../threads/avatar-group.tsx";
import { isPaneShowing } from "./pane-selection.ts";
import { usePaneNavigation } from "./use-right-pane.ts";

/** The member stack as a button: faces and the count; it opens the Members pane. */
function MembersButton({
  ids,
  count,
  pressed,
  onClick,
}: {
  readonly ids: readonly number[];
  readonly count: number;
  readonly pressed: boolean;
  readonly onClick: () => void;
}) {
  const label = `${count} ${count === 1 ? "member" : "members"}`;

  return (
    <Tooltip content="Members" describe={false} placement="bottom">
      <button
        type="button"
        className="room-members"
        aria-label={`Members: ${label}`}
        aria-pressed={pressed}
        onClick={onClick}
      >
        <AvatarGroup userIds={ids} size={22} max={3} />
        <span className="room-members-count tabular" aria-hidden="true">
          {count}
        </span>
      </button>
    </Tooltip>
  );
}

/**
 * The room header's pane toggles, Slack's set: the member stack (Members), Threads, Pinned
 * messages (with the room's pin count) and Files. A pressed button is the pane showing; pressing
 * it again closes the pane. Direct messages have no threads and no member stack.
 */
export function PaneButtons({ roomId }: { readonly roomId: number }) {
  const { view, toggle } = usePaneNavigation(roomId);
  const detail = useStore((state) => state.rooms[roomId]?.detail ?? null);
  const kind = detail?.room.kind ?? null;

  if (detail === null || kind === null) {
    return null;
  }

  const direct = kind === "direct";
  const pins = detail.pinsCount;

  return (
    <div className="pane-buttons">
      {kind === "stage" ? (
        <StageButton
          roomId={roomId}
          pressed={isPaneShowing(view, "stage")}
          onClick={() => toggle("stage")}
        />
      ) : null}
      {direct ? null : (
        <IconButton
          icon="thread"
          label="Threads"
          tooltipPlacement="bottom"
          className="pane-button"
          aria-pressed={isPaneShowing(view, "threads")}
          onClick={() => toggle("threads")}
        />
      )}
      <span className="pane-button-wrap">
        <IconButton
          icon="pin"
          label={pins > 0 ? `Pinned messages (${pins})` : "Pinned messages"}
          tooltipPlacement="bottom"
          className="pane-button"
          aria-pressed={isPaneShowing(view, "pins")}
          onClick={() => toggle("pins")}
        />
        <Badge count={pins} tone="neutral" floating label={`${pins} pinned`} />
      </span>
      <IconButton
        icon="file"
        label="Files"
        tooltipPlacement="bottom"
        className="pane-button"
        aria-pressed={isPaneShowing(view, "files")}
        onClick={() => toggle("files")}
      />
      {direct ? null : (
        <MembersButton
          ids={detail.memberPreviewIds}
          count={detail.memberCount}
          pressed={isPaneShowing(view, "members")}
          onClick={() => toggle("members")}
        />
      )}
    </div>
  );
}

/**
 * The same panes as ⋯ menu items, for a phone's room header, which has no room for the buttons:
 * Stage (stage rooms, with the raised hands a host can act on), Members, Threads, Pinned
 * messages and Files. Each opens its pane.
 */
export function PaneMenuItems({ roomId }: { readonly roomId: number }) {
  const { toggle } = usePaneNavigation(roomId);
  const hands = useRaisedHands(roomId);
  const detail = useStore((state) => state.rooms[roomId]?.detail ?? null);
  const kind = detail?.room.kind ?? null;

  if (detail === null || kind === null) {
    return null;
  }

  const direct = kind === "direct";
  const pins = detail.pinsCount;

  return (
    <>
      {kind === "stage" ? (
        <MenuItem icon="radio" onSelect={() => toggle("stage")}>
          Stage
          {hands > 0 ? (
            <span className="pane-menu-count">
              <Badge count={hands} label={raisedHandsLabel(hands)} />
            </span>
          ) : null}
        </MenuItem>
      ) : null}
      {direct ? null : (
        <MenuItem icon="users" onSelect={() => toggle("members")}>
          Members ({detail.memberCount})
        </MenuItem>
      )}
      {direct ? null : (
        <MenuItem icon="thread" onSelect={() => toggle("threads")}>
          Threads
        </MenuItem>
      )}
      <MenuItem icon="pin" onSelect={() => toggle("pins")}>
        {pins > 0 ? `Pinned messages (${pins})` : "Pinned messages"}
      </MenuItem>
      <MenuItem icon="file" onSelect={() => toggle("files")}>
        Files
      </MenuItem>
    </>
  );
}
