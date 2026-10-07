import { useCallback, useEffect, useRef, useState } from "react";
import type { PushSubscriptionList } from "../../gen/PushSubscriptionList.ts";
import { settings as settingsActions, unsubscribePushEndpoint } from "../../sync/settings.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneEmpty, PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { deviceName } from "./settings-format.ts";
import { SettingsGroup, SettingsPage, toastFailure } from "./settings-parts.tsx";
import { usePushEnrollment } from "./use-push-enrollment.ts";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly list: PushSubscriptionList };

/**
 * Push devices: each browser that gets push notifications, a test notification to check one, and
 * a way to stop one.
 */
export function DevicesSection() {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [busy, setBusy] = useState<number | null>(null);
  const listRevision = useRef(0);

  const fetchList = useCallback(() => {
    const revision = ++listRevision.current;

    settingsActions.pushSubscriptions().then(
      (list) => {
        if (revision !== listRevision.current) return;

        setLoad({ status: "ready", list });
      },
      (error: Error) => {
        if (revision !== listRevision.current) return;

        setLoad({ status: "error", message: error.message });
      },
    );
  }, []);

  useEffect(fetchList, [fetchList]);

  const replaceList = useCallback((list: PushSubscriptionList) => {
    listRevision.current += 1;

    setLoad({ status: "ready", list });
  }, []);

  const enrollment = usePushEnrollment(replaceList);
  // LEAD-UI: the future Enable control here calls enrollment.enable directly from its click;
  // permission, subscribed and busy are available without any automatic permission prompt.

  const reload = () => {
    setLoad({ status: "loading" });
    fetchList();
  };

  // Each device's test runs on its own: one finishing never clears another still on its way.
  const [testing, setTesting] = useState<ReadonlySet<number>>(new Set());

  const test = (id: number) => {
    setTesting((current) => new Set(current).add(id));
    settingsActions
      .testPush(id)
      .then(
        () => toast({ title: "Test notification sent", tone: "success" }),
        (error: Error) => toastFailure("Couldn't send a test", error),
      )
      .finally(() =>
        setTesting((current) => {
          const next = new Set(current);

          next.delete(id);

          return next;
        }),
      );
  };

  const remove = (id: number) => {
    const endpoint =
      load.status === "ready"
        ? load.list.pushSubscriptions.find((subscription) => subscription.id === id)?.endpoint
        : undefined;

    setBusy(id);
    settingsActions
      .removePushSubscription(id)
      .then(
        (list) => {
          replaceList(list);

          if (
            endpoint !== undefined &&
            !list.pushSubscriptions.some((subscription) => subscription.endpoint === endpoint)
          ) {
            void unsubscribePushEndpoint(endpoint)
              .finally(enrollment.refresh)
              .catch(() => undefined);
          }
        },
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
                <Button
                  variant="secondary"
                  size="sm"
                  loading={testing.has(subscription.id)}
                  disabled={busy !== null}
                  onClick={() => test(subscription.id)}
                >
                  Send a test
                </Button>
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
