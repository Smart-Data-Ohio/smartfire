import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { PushSubscriptionList } from "../../gen/PushSubscriptionList.ts";
import {
  enablePushNotifications,
  inspectPush,
  type PushEnrollmentOutcome,
  type PushPermission,
} from "../../sync/settings.ts";

export interface PushEnrollment {
  readonly permission: PushPermission;
  readonly subscribed: boolean;
  readonly busy: boolean;
  enable(): Promise<PushEnrollmentOutcome>;
  refresh(): void;
}

declare global {
  interface Window {
    /** Mock-mode enrollment driver: exercises the click path before its visible control ships. */
    __smartfirePushEnrollment?: PushEnrollment;
  }
}

/** A future notification control reads this state and calls `enable` directly from its click. */
export function usePushEnrollment(onEnabled: (list: PushSubscriptionList) => void): PushEnrollment {
  const [permission, setPermission] = useState<PushPermission>("unsupported");
  const [subscribed, setSubscribed] = useState(false);
  const [busy, setBusy] = useState(false);
  const running = useRef(false);
  const inspection = useRef(0);

  const refresh = useCallback(() => {
    const revision = ++inspection.current;

    void inspectPush().then(
      (state) => {
        if (revision !== inspection.current) return;

        setPermission(state.permission);
        setSubscribed(state.subscribed);
      },
      () => undefined,
    );
  }, []);

  useEffect(() => {
    refresh();

    return () => {
      inspection.current += 1;
    };
  }, [refresh]);

  const enable = useCallback((): Promise<PushEnrollmentOutcome> => {
    if (running.current)
      return Promise.resolve({ kind: "failed", message: "Enrollment is already in progress." });

    running.current = true;
    inspection.current += 1;
    // Start before scheduling React work: requestPermission stays on the click's synchronous path.
    const run = enablePushNotifications();

    setBusy(true);

    return run
      .then((outcome) => {
        if (outcome.kind === "enabled") {
          setPermission("granted");
          setSubscribed(true);
          onEnabled(outcome.list);
        } else {
          refresh();
        }

        return outcome;
      })
      .finally(() => {
        running.current = false;
        setBusy(false);
      });
  }, [onEnabled, refresh]);

  const enrollment = useMemo<PushEnrollment>(
    () => ({ permission, subscribed, busy, enable, refresh }),
    [permission, subscribed, busy, enable, refresh],
  );

  useEffect(() => {
    if (import.meta.env.MODE !== "mock") return;

    window.__smartfirePushEnrollment = enrollment;

    return () => {
      delete window.__smartfirePushEnrollment;
    };
  }, [enrollment]);

  return enrollment;
}
