import type { CreatePushSubscription } from "../gen/CreatePushSubscription.ts";
import type { PushPublicKey } from "../gen/PushPublicKey.ts";
import type { PushSubscriptionList } from "../gen/PushSubscriptionList.ts";
import { store } from "../store/store.ts";
import { settings } from "./settings.ts";

export type PushPermission = NotificationPermission | "unsupported";

export type PushEnrollmentOutcome =
  | { readonly kind: "enabled"; readonly list: PushSubscriptionList }
  | { readonly kind: "denied"; readonly permission: "denied" | "default" }
  | { readonly kind: "unsupported" }
  | { readonly kind: "failed"; readonly message: string };

type Worker = Pick<
  ServiceWorker,
  "state" | "scriptURL" | "addEventListener" | "removeEventListener"
>;

type Subscription = Pick<PushSubscription, "endpoint" | "toJSON" | "unsubscribe">;

/** The browser boundary, narrowed so tests need fake only the APIs browsers implement. */
export interface PushRegistration {
  readonly scope: string;
  readonly active: Worker | null;
  readonly installing: Worker | null;
  readonly waiting: Worker | null;
  readonly pushManager: {
    getSubscription(): Promise<Subscription | null>;
    subscribe(options: PushSubscriptionOptionsInit): Promise<Subscription>;
  };
}

export interface PushBrowser {
  readonly origin: string;
  readonly workerUrl: string;
  permission(): NotificationPermission;
  requestPermission(): Promise<NotificationPermission>;
  register(): Promise<PushRegistration>;
  currentRegistration(): Promise<PushRegistration | undefined>;
}

export interface PushEnrollmentApi {
  pushPublicKey(): Promise<PushPublicKey>;
  createPushSubscription(body: CreatePushSubscription): Promise<PushSubscriptionList>;
}

/** A null boot URL selects the classic UI's worker; an explicit click can install it too. */
export function pushBrowser(): PushBrowser | null {
  if (
    !("Notification" in window) ||
    !("PushManager" in window) ||
    !("serviceWorker" in navigator)
  ) {
    return null;
  }

  const workerUrl = store.getState().boot?.serviceWorkerUrl ?? "/service-worker.js";

  return {
    origin: window.location.origin,
    workerUrl,
    permission: () => Notification.permission,
    requestPermission: () => Notification.requestPermission(),
    register: () =>
      navigator.serviceWorker.register(workerUrl, { scope: "/", updateViaCache: "none" }),
    currentRegistration: () => navigator.serviceWorker.getRegistration(window.location.origin),
  };
}

/** Read the browser's state without registering anything or asking for permission. */
export async function inspectPush(browser = pushBrowser()): Promise<{
  readonly permission: PushPermission;
  readonly subscribed: boolean;
}> {
  if (browser === null) return { permission: "unsupported", subscribed: false };

  const permission = browser.permission();
  const registration = await browser.currentRegistration();
  const subscription = (await registration?.pushManager.getSubscription()) ?? null;

  return { permission, subscribed: subscription !== null };
}

/** Base64url VAPID public keys are passed as bytes, including on Safari. */
export function applicationServerKey(encoded: string): Uint8Array<ArrayBuffer> {
  const base64 = encoded.replaceAll("-", "+").replaceAll("_", "/");
  const binary = atob(base64.padEnd(Math.ceil(base64.length / 4) * 4, "="));
  const bytes = new Uint8Array(binary.length);

  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }

  // VAPID uses an uncompressed P-256 point: a 0x04 prefix and two 32-byte coordinates.
  if (bytes.length !== 65 || bytes[0] !== 4) throw new Error("The push public key is invalid.");

  return bytes;
}

async function selectedWorker(registration: PushRegistration, browser: PushBrowser): Promise<void> {
  const script = new URL(browser.workerUrl, browser.origin).href;

  const selected = () =>
    registration.active?.scriptURL === script && registration.active.state === "activated";

  if (registration.scope !== new URL("/", browser.origin).href) {
    throw new Error("The push worker does not have the root scope.");
  }

  if (selected()) return;

  const worker = registration.installing ?? registration.waiting ?? registration.active;

  if (worker === null || worker.scriptURL !== script) {
    throw new Error("The selected push worker is unavailable.");
  }

  await new Promise<void>((resolve, reject) => {
    const finish = (error: Error | null) => {
      clearTimeout(timeout);
      worker.removeEventListener("statechange", changed);

      if (error === null) resolve();
      else reject(error);
    };

    const changed = () => {
      if (selected()) finish(null);
      else if (worker.state === "redundant")
        finish(new Error("The push worker failed to install."));
    };

    const timeout = setTimeout(
      () => finish(new Error("The push worker did not activate.")),
      10_000,
    );

    worker.addEventListener("statechange", changed);
    changed();
  });
}

/**
 * Call directly in the click handler. The permission request runs before the first await,
 * preserving browser user activation; mounting or inspecting state never prompts.
 */
export async function enablePushNotifications(
  browser = pushBrowser(),
  api: PushEnrollmentApi = settings,
): Promise<PushEnrollmentOutcome> {
  if (browser === null) return { kind: "unsupported" };

  try {
    const permission = browser.permission();
    const granted = permission === "default" ? await browser.requestPermission() : permission;

    if (granted !== "granted") return { kind: "denied", permission: granted };

    const { publicKey } = await api.pushPublicKey();

    if (publicKey === null)
      return { kind: "failed", message: "Push notifications are not configured." };

    const key = applicationServerKey(publicKey);
    const registration = await browser.register();

    await selectedWorker(registration, browser);

    const existing = await registration.pushManager.getSubscription();

    const subscription =
      existing ??
      (await registration.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: key,
      }));

    const json = subscription.toJSON();
    const p256dhKey = json.keys?.p256dh;
    const authKey = json.keys?.auth;

    if (p256dhKey === undefined || authKey === undefined) {
      throw new Error("The browser did not provide push encryption keys.");
    }

    const list = await api.createPushSubscription({
      endpoint: subscription.endpoint,
      p256dhKey,
      authKey,
    });

    return { kind: "enabled", list };
  } catch (error) {
    return {
      kind: "failed",
      message: error instanceof Error ? error.message : "Couldn't enable push notifications.",
    };
  }
}

/** Called after the server removes a device; other browsers' subscriptions stay untouched. */
export async function unsubscribePushEndpoint(
  endpoint: string,
  browser = pushBrowser(),
): Promise<void> {
  if (browser === null) return;

  const registration = await browser.currentRegistration();
  const subscription = (await registration?.pushManager.getSubscription()) ?? null;

  if (subscription?.endpoint === endpoint) await subscription.unsubscribe();
}
