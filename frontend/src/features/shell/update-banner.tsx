import { reloadForUpdate, useAppUpdateRequired } from "../../service-worker/update-required.ts";
import { Icon } from "../../ui/icons/icon.tsx";

/**
 * A strip over the panes once a part of the app this tab hasn't loaded yet is gone from the
 * server (a deploy replaced it). Nothing on screen breaks or reloads by itself: the person
 * finishes what they're doing and reloads when they choose.
 */
export function UpdateBanner() {
  const required = useAppUpdateRequired();

  return (
    <div className="update-banner" data-open={required} role="status" aria-hidden={!required}>
      <div className="update-banner-inner">
        <Icon name="refresh-cw" size={14} />
        <span>Smartfire has been updated. Reload to get the latest version.</span>
        <button
          type="button"
          className="update-banner-action"
          tabIndex={required ? 0 : -1}
          onClick={reloadForUpdate}
        >
          Reload
        </button>
      </div>
    </div>
  );
}
