import { useCallback, useEffect, useRef, useState } from "react";
import type { PushSubscriptionList } from "../../gen/PushSubscriptionList.ts";
import { settings as settingsActions, unsubscribePushEndpoint } from "../../sync/settings.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneEmpty, PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { deviceName } from "./settings-format.ts";
import { SettingsGroup, SettingsPage, toastFailure } from "./settings-parts.tsx";
import { type PushEnrollment, usePushEnrollment } from "./use-push-enrollment.ts";

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
      <SettingsGroup title="This browser">
        <ThisBrowser enrollment={enrollment} />
      </SettingsGroup>
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

interface BrowserPushState {
  readonly key: "unsupported" | "blocked" | "on" | "off";
  readonly icon: IconName;
  readonly title: string;
  readonly text: string;
  readonly canEnable: boolean;
}

/** What this browser can do about push, and the one control that asks for permission. */
function browserState(enrollment: PushEnrollment): BrowserPushState {
  if (enrollment.permission === "unsupported") {
    return {
      key: "unsupported",
      icon: "bell-off",
      title: "Push isn't available here",
      text: "This browser can't show push notifications. On an iPhone or iPad, add Smartfire to your Home Screen first.",
      canEnable: false,
    };
  }

  if (enrollment.permission === "denied") {
    return {
      key: "blocked",
      icon: "bell-off",
      title: "Notifications are blocked",
      text: "Allow notifications for this site in your browser's settings, then come back here.",
      canEnable: false,
    };
  }

  if (enrollment.permission === "granted" && enrollment.subscribed) {
    return {
      key: "on",
      icon: "bell-ring",
      title: "Notifications are on",
      text: "This browser gets push notifications, even when Smartfire isn't open.",
      canEnable: false,
    };
  }

  return {
    key: "off",
    icon: "bell",
    title: "Notifications are off",
    text: "Get push notifications in this browser, even when Smartfire isn't open.",
    canEnable: true,
  };
}

function ThisBrowser({ enrollment }: { readonly enrollment: PushEnrollment }) {
  const state = browserState(enrollment);

  // The permission prompt starts inside this click and nowhere else.
  const enable = () => {
    void enrollment.enable().then((outcome) => {
      if (outcome.kind === "enabled") {
        toast({ title: "Notifications are on for this browser", tone: "success" });
      } else if (outcome.kind === "denied") {
        toast({
          title: "Notifications stay off",
          description:
            outcome.permission === "denied"
              ? "This browser blocked them. You can allow them in its site settings."
              : "You can turn them on here whenever you like.",
        });
      } else if (outcome.kind === "unsupported") {
        toast({ title: "This browser can't show push notifications" });
      } else {
        toastFailure("Couldn't turn on notifications", new Error(outcome.message));
      }
    });
  };

  return (
    <div className="settings-list-row settings-push-row" data-state={state.key}>
      <span className="settings-push-icon" aria-hidden="true">
        <Icon name={state.icon} size={16} />
      </span>
      <span className="settings-list-main">
        <strong>{state.title}</strong>
        <span className="text-muted">{state.text}</span>
      </span>
      {state.canEnable ? (
        <Button variant="primary" size="sm" loading={enrollment.busy} onClick={enable}>
          Enable notifications
        </Button>
      ) : null}
    </div>
  );
}
