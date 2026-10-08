import {
  type AppUpdateKind,
  reloadForUpdate,
  useAppUpdateKind,
} from "../../service-worker/update-required.ts";
import { Icon } from "../../ui/icons/icon.tsx";

const COPY: Readonly<Record<AppUpdateKind, string>> = {
  updated: "Smartfire has been updated. Reload to get the latest version.",
  failed: "Couldn't load part of Smartfire. Reload to try again.",
};

/**
 * A strip over the panes once a part of the app this tab hasn't loaded yet couldn't be fetched.
 * It names a deploy only when the server confirms a newer build; otherwise (offline, a flaky
 * request) it says plainly that something didn't load. Nothing on screen breaks or reloads by
 * itself: the person finishes what they're doing and reloads when they choose.
 */
export function UpdateBanner() {
  const kind = useAppUpdateKind();
  const open = kind !== null;

  return (
    <div
      className="update-banner"
      data-open={open}
      data-kind={kind ?? undefined}
      role="status"
      aria-hidden={!open}
    >
      <div className="update-banner-inner">
        <Icon name="refresh-cw" size={14} />
        <span>{kind === null ? "" : COPY[kind]}</span>
        <button
          type="button"
          className="update-banner-action"
          tabIndex={open ? 0 : -1}
          onClick={reloadForUpdate}
        >
          Reload
        </button>
      </div>
    </div>
  );
}
