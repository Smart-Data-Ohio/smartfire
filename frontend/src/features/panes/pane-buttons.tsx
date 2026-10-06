import { useStore } from "../../store/store.ts";
import { Badge } from "../../ui/badge.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
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
