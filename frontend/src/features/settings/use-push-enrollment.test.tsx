import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PushSubscriptionList } from "../../gen/PushSubscriptionList.ts";
import { settings } from "../../sync/settings.ts";
import { usePushEnrollment } from "./use-push-enrollment.ts";

const PUBLIC_KEY =
  "BGsX0fLhLEJH-Lzm5WOkQPJ3A32BLeszoPShOUXYmMKWT-NC4v4af5uO5-tKfA-eFivOM1drMV7Oy7ZAaDe_UfU=";

type Subscription = Pick<PushSubscription, "endpoint" | "toJSON" | "unsubscribe">;

class Worker extends EventTarget {
  readonly scriptURL = `${window.location.origin}/service-worker.js`;
  readonly state = "activated";
}

function browser() {
  let permission: NotificationPermission = "default";
  let held: Subscription | null = null;

  const subscription: Subscription = {
    endpoint: "https://fcm.googleapis.com/hook",
    toJSON: () => ({ keys: { p256dh: "p256", auth: "auth" } }),
    unsubscribe: async () => true,
  };

  const getSubscription = vi.fn(async () => held);

  const subscribe = vi.fn(async () => {
    held = subscription;

    return subscription;
  });

  const registration = {
    scope: `${window.location.origin}/`,
    active: new Worker(),
    installing: null,
    waiting: null,
    pushManager: { getSubscription, subscribe },
  };

  const requestPermission = vi.fn(async (): Promise<NotificationPermission> => {
    permission = "granted";

    return permission;
  });

  const register = vi.fn(async () => registration);
  const getRegistration = vi.fn(async () => registration);

  vi.stubGlobal("Notification", {
    get permission() {
      return permission;
    },
    requestPermission,
  });
  vi.stubGlobal("PushManager", class {});
  vi.stubGlobal("navigator", { serviceWorker: { register, getRegistration } });
  vi.spyOn(settings, "pushPublicKey").mockResolvedValue({ publicKey: PUBLIC_KEY });

  const list: PushSubscriptionList = {
    pushSubscriptions: [
      {
        id: 1,
        endpoint: subscription.endpoint,
        browser: "Chrome",
        version: "144",
        platform: "Linux",
      },
    ],
  };

  vi.spyOn(settings, "createPushSubscription").mockResolvedValue(list);

  return { requestPermission, register, getRegistration, getSubscription, list };
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("push enrollment state", () => {
  it("inspects on mount without prompting or registering", async () => {
    const f = browser();
    const { result } = renderHook(() => usePushEnrollment(() => undefined));

    await waitFor(() => expect(result.current.permission).toBe("default"));
    expect(result.current.subscribed).toBe(false);
    expect(result.current.busy).toBe(false);
    expect(f.requestPermission).not.toHaveBeenCalled();
    expect(f.register).not.toHaveBeenCalled();
  });

  it("requests on the click path, tracks busy, and sends the updated devices to its owner", async () => {
    const f = browser();
    const saved = vi.fn();
    const pending = Promise.withResolvers<PushSubscriptionList>();

    vi.mocked(settings.createPushSubscription).mockReturnValue(pending.promise);
    const { result } = renderHook(() => usePushEnrollment(saved));

    await waitFor(() => expect(result.current.permission).toBe("default"));

    const run = act(async () => {
      await result.current.enable();
    });

    expect(f.requestPermission).toHaveBeenCalledOnce();
    pending.resolve(f.list);
    await run;
    expect(saved).toHaveBeenCalledWith(f.list);
    expect(result.current).toMatchObject({ permission: "granted", subscribed: true, busy: false });
  });

  it("a delayed mount inspection cannot overwrite a completed enrollment", async () => {
    const f = browser();
    const inspection = Promise.withResolvers<Subscription | null>();

    f.getSubscription.mockReturnValueOnce(inspection.promise);
    const { result } = renderHook(() => usePushEnrollment(() => undefined));

    await waitFor(() => expect(f.getSubscription).toHaveBeenCalledOnce());
    await act(async () => {
      await result.current.enable();
    });
    expect(result.current.subscribed).toBe(true);
    await act(async () => {
      inspection.resolve(null);
      await inspection.promise;
    });
    expect(result.current).toMatchObject({ permission: "granted", subscribed: true, busy: false });
  });
});
