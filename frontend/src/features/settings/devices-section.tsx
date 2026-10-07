import { useCallback, useEffect, useState } from "react";
import type { PushSubscriptionList } from "../../gen/PushSubscriptionList.ts";
import { settings as settingsActions } from "../../sync/settings.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { PaneEmpty, PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { deviceName } from "./settings-format.ts";
import { SettingsGroup, SettingsPage, toastFailure } from "./settings-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly list: PushSubscriptionList };

/** Push devices: each browser that gets push notifications, and a way to stop one. */
export function DevicesSection() {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [busy, setBusy] = useState<number | null>(null);

  const fetchList = useCallback(() => {
    settingsActions.pushSubscriptions().then(
      (list) => setLoad({ status: "ready", list }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(fetchList, [fetchList]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchList();
  };

  const remove = (id: number) => {
    setBusy(id);
    settingsActions
      .removePushSubscription(id)
      .then(
        (list) => setLoad({ status: "ready", list }),
        (error: Error) => toastFailure("Couldn't remove that device", error),
      )
      .finally(() => setBusy(null));
  };

  return (
    <SettingsPage
      title="Push devices"
      description="Browsers and phones that get push notifications for your account."
    >
      {load.status === "loading" ? <PaneListSkeleton rows={2} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {load.status === "ready" && load.list.pushSubscriptions.length === 0 ? (
        <PaneEmpty
          icon="bell-off"
          title="No devices yet"
          text="Allow notifications in a browser and it shows up here."
        />
      ) : null}
      {load.status === "ready" && load.list.pushSubscriptions.length > 0 ? (
        <SettingsGroup title="Subscribed">
          <ul className="settings-list">
            {load.list.pushSubscriptions.map((subscription) => (
              <li key={subscription.id} className="settings-list-row">
                <span className="settings-list-main">
                  <strong>{deviceName(subscription)}</strong>
                  <span className="settings-endpoint text-faint">{subscription.endpoint}</span>
                </span>
                <IconButton
                  icon="trash"
                  size="sm"
                  label={`Remove ${deviceName(subscription)}`}
                  disabled={busy !== null}
                  onClick={() => remove(subscription.id)}
                />
              </li>
            ))}
          </ul>
        </SettingsGroup>
      ) : null}
    </SettingsPage>
  );
}
