import { describe, expect, it, vi } from "vitest";
import type { CreatePushSubscription } from "../gen/CreatePushSubscription.ts";
import type { PushSubscriptionList } from "../gen/PushSubscriptionList.ts";
import {
  applicationServerKey,
  enablePushNotifications,
  inspectPush,
  type PushBrowser,
  type PushEnrollmentApi,
  type PushRegistration,
  unsubscribePushEndpoint,
} from "./push-enrollment.ts";

const PUBLIC_KEY =
  "BGsX0fLhLEJH-Lzm5WOkQPJ3A32BLeszoPShOUXYmMKWT-NC4v4af5uO5-tKfA-eFivOM1drMV7Oy7ZAaDe_UfU=";

const ENDPOINT = "https://fcm.googleapis.com/fcm/send/test-browser";

const LIST: PushSubscriptionList = {
  pushSubscriptions: [
    { id: 1, endpoint: ENDPOINT, browser: "Chrome", version: "144", platform: "Linux" },
  ],
};

class Worker extends EventTarget {
  readonly scriptURL = "https://smartfire.test/app/service-worker.js";
  state: ServiceWorkerState = "activated";
}

function fixture(permission: NotificationPermission = "default") {
  const events: string[] = [];
  const unsubscribe = vi.fn(async () => true);

  const subscription = {
    endpoint: ENDPOINT,
    toJSON: () => ({ endpoint: ENDPOINT, keys: { p256dh: "p256", auth: "auth" } }),
    unsubscribe,
  };

  const subscribe = vi.fn(async (_options: PushSubscriptionOptionsInit) => {
    events.push("subscribe");

    return subscription;
  });

  const getSubscription = vi.fn(async (): Promise<typeof subscription | null> => null);
  const worker = new Worker();

  const registration: PushRegistration = {
    scope: "https://smartfire.test/",
    active: worker,
    installing: null,
    waiting: null,
    pushManager: { subscribe, getSubscription },
  };

  const requestPermission = vi.fn(async (): Promise<NotificationPermission> => {
    events.push("permission");

    return "granted";
  });

  const register = vi.fn(async () => {
    events.push("register");

    return registration;
  });

  const currentRegistration = vi.fn(
    async (): Promise<PushRegistration | undefined> => registration,
  );

  const browser: PushBrowser = {
    origin: "https://smartfire.test",
    workerUrl: "/app/service-worker.js",
    permission: () => permission,
    requestPermission,
    register,
    currentRegistration,
  };

  const pushPublicKey = vi.fn(async () => {
    events.push("key");

    return { publicKey: PUBLIC_KEY };
  });

  const createPushSubscription = vi.fn(async (_body: CreatePushSubscription) => {
    events.push("save");

    return LIST;
  });

  const api: PushEnrollmentApi = { pushPublicKey, createPushSubscription };

  return {
    browser,
    api,
    events,
    worker,
    registration,
    subscribe,
    getSubscription,
    subscription,
    unsubscribe,
    requestPermission,
    register,
    currentRegistration,
    pushPublicKey,
    createPushSubscription,
  };
}

describe("explicit browser push enrollment", () => {
  it("requests permission synchronously, then registers the selected root worker and saves typed keys", async () => {
    const f = fixture();
    const run = enablePushNotifications(f.browser, f.api);

    expect(f.events).toEqual(["permission"]);
    expect(await run).toEqual({ kind: "enabled", list: LIST, endpoint: ENDPOINT });
    expect(f.events).toEqual(["permission", "key", "register", "subscribe", "save"]);
    const options = f.subscribe.mock.calls[0]?.[0];

    expect(options?.userVisibleOnly).toBe(true);
    expect(options?.applicationServerKey).toEqual(applicationServerKey(PUBLIC_KEY));
    expect(f.createPushSubscription).toHaveBeenCalledWith({
      endpoint: ENDPOINT,
      p256dhKey: "p256",
      authKey: "auth",
    });
  });

  it("reuses a subscription and granted permission without asking or subscribing again", async () => {
    const f = fixture("granted");

    f.getSubscription.mockResolvedValue(f.subscription);
    expect(await enablePushNotifications(f.browser, f.api)).toEqual({
      kind: "enabled",
      list: LIST,
      endpoint: ENDPOINT,
    });
    expect(f.requestPermission).not.toHaveBeenCalled();
    expect(f.subscribe).not.toHaveBeenCalled();
    expect(f.createPushSubscription).toHaveBeenCalledOnce();
  });

  it.each(["denied", "default"] as const)(
    "returns %s without enrolling when the permission prompt is refused",
    async (permission) => {
      const f = fixture();

      f.requestPermission.mockResolvedValue(permission);
      expect(await enablePushNotifications(f.browser, f.api)).toEqual({
        kind: "denied",
        permission,
      });
      expect(f.pushPublicKey).not.toHaveBeenCalled();
      expect(f.register).not.toHaveBeenCalled();
    },
  );

  it("does not ask again when the browser already denied notifications", async () => {
    const f = fixture("denied");

    expect(await enablePushNotifications(f.browser, f.api)).toEqual({
      kind: "denied",
      permission: "denied",
    });
    expect(f.requestPermission).not.toHaveBeenCalled();
    expect(f.createPushSubscription).not.toHaveBeenCalled();
  });

  it("reports unsupported browsers and unconfigured push without attempting a subscription", async () => {
    const f = fixture("granted");

    expect(await enablePushNotifications(null, f.api)).toEqual({ kind: "unsupported" });

    const unconfigured: PushEnrollmentApi = {
      ...f.api,
      pushPublicKey: async () => ({ publicKey: null }),
    };

    expect(await enablePushNotifications(f.browser, unconfigured)).toEqual({
      kind: "failed",
      message: "Push notifications are not configured.",
    });
    expect(f.register).not.toHaveBeenCalled();
  });

  it("reports registration, subscription and server failures as failed outcomes", async () => {
    for (const stage of ["register", "subscribe", "save"] as const) {
      const f = fixture("granted");
      const error = new Error(`${stage} failed`);

      if (stage === "register") f.register.mockRejectedValue(error);

      if (stage === "subscribe") f.subscribe.mockRejectedValue(error);

      if (stage === "save") f.createPushSubscription.mockRejectedValue(error);
      expect(await enablePushNotifications(f.browser, f.api)).toEqual({
        kind: "failed",
        message: error.message,
      });
    }
  });

  it("waits for the selected script to activate before subscribing", async () => {
    const f = fixture("granted");

    f.worker.state = "installing";
    let active: Worker | null = null;

    const registration: PushRegistration = {
      ...f.registration,
      get active() {
        return active;
      },
      installing: f.worker,
    };

    f.register.mockResolvedValue(registration);
    const run = enablePushNotifications(f.browser, f.api);

    await vi.waitFor(() => expect(f.register).toHaveBeenCalledOnce());
    expect(f.subscribe).not.toHaveBeenCalled();
    // The browser's registration object changes active when the installer reaches activated.
    active = f.worker;
    f.worker.state = "activated";
    f.worker.dispatchEvent(new Event("statechange"));
    expect((await run).kind).toBe("enabled");
  });

  it("refuses a registration with another scope or active script", async () => {
    for (const wrong of ["scope", "script"] as const) {
      const f = fixture("granted");

      const browser =
        wrong === "script" ? { ...f.browser, workerUrl: "/service-worker.js" } : f.browser;

      if (wrong === "scope")
        f.register.mockResolvedValue({ ...f.registration, scope: "https://smartfire.test/app/" });
      expect((await enablePushNotifications(browser, f.api)).kind).toBe("failed");
      expect(f.subscribe).not.toHaveBeenCalled();
    }
  });

  it("inspection neither prompts nor registers, including when no root worker exists", async () => {
    const f = fixture();

    f.currentRegistration.mockResolvedValue(undefined);
    expect(await inspectPush(f.browser)).toEqual({
      permission: "default",
      subscribed: false,
      endpoint: null,
    });
    expect(f.requestPermission).not.toHaveBeenCalled();
    expect(f.register).not.toHaveBeenCalled();
    expect(await inspectPush(null)).toEqual({
      permission: "unsupported",
      subscribed: false,
      endpoint: null,
    });
  });

  it("inspection reports this browser's endpoint so Settings can match it against saved devices", async () => {
    const f = fixture("granted");

    f.getSubscription.mockResolvedValue(f.subscription);
    expect(await inspectPush(f.browser)).toEqual({
      permission: "granted",
      subscribed: true,
      endpoint: ENDPOINT,
    });
    expect(f.subscribe).not.toHaveBeenCalled();
  });

  it("unsubscribes only the local endpoint after removing a device", async () => {
    const f = fixture();

    f.getSubscription.mockResolvedValue(f.subscription);
    await unsubscribePushEndpoint("https://fcm.googleapis.com/other-device", f.browser);
    expect(f.unsubscribe).not.toHaveBeenCalled();
    await unsubscribePushEndpoint(ENDPOINT, f.browser);
    expect(f.unsubscribe).toHaveBeenCalledOnce();
    expect(f.requestPermission).not.toHaveBeenCalled();
  });

  it("decodes padded and unpadded base64url keys and rejects malformed keys", () => {
    const padded = applicationServerKey(PUBLIC_KEY);

    expect(applicationServerKey(PUBLIC_KEY.replace(/=+$/, ""))).toEqual(padded);
    expect(padded).toHaveLength(65);
    expect(padded[0]).toBe(4);
    expect(() => applicationServerKey("invalid")).toThrow();
    expect(() => applicationServerKey("AQ==")).toThrow();
  });
});
