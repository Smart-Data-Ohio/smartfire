import { Link } from "@tanstack/react-router";
import { Icon } from "../../ui/icons/icon.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";

/** Loads the calendar's chunk ahead of a likely click. */
function preloadEvents(): void {
  void import("./events-page.tsx");
}

/** The room header's calendar button: the room's events page (classic's "Events" link). */
export function EventsLink({ roomId }: { readonly roomId: number }) {
  return (
    <Tooltip content="Events" placement="bottom" describe={false}>
      <Link
        to="/r/$roomId/events"
        params={{ roomId }}
        className="button pane-button"
        data-variant="icon"
        data-size="md"
        aria-label="Events"
        onPointerEnter={preloadEvents}
        onFocus={preloadEvents}
      >
        <span className="button-stack">
          <span className="button-label">
            <Icon name="calendar" size={16} />
          </span>
        </span>
      </Link>
    </Tooltip>
  );
}
