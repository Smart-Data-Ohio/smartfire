/**
 * The room header's Stage button (stage rooms only): opens the roster pane, with the raised-hand
 * count for the hosts and administrators who can act on it.
 */
import { useStore } from "../../store/store.ts";
import { Badge } from "../../ui/badge.tsx";
import { IconButton } from "../../ui/icon-button.tsx";

export function StageButton({
  roomId,
  pressed,
  onClick,
}: {
  readonly roomId: number;
  readonly pressed: boolean;
  readonly onClick: () => void;
}) {
  const hands = useStore((state) => {
    const stage = state.stages[roomId];
    const viewerId = state.me?.user.id;
    const viewer = stage?.members.find((member) => member.userId === viewerId);
    const manages = state.me?.user.role === "administrator" || viewer?.role === "host";

    return manages
      ? (stage?.members.filter((member) => member.handRaisedAt !== null).length ?? 0)
      : 0;
  });

  return (
    <span className="pane-button-wrap">
      <IconButton
        icon="radio"
        label={hands > 0 ? `Stage (${hands} raised hands)` : "Stage"}
        tooltipPlacement="bottom"
        className="pane-button"
        aria-pressed={pressed}
        onClick={onClick}
      />
      <Badge count={hands} floating label={`${hands} raised hands`} />
    </span>
  );
}
