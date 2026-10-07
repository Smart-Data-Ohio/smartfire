import { store } from "../store/store.ts";

/** Startup quietly swaps the script in the root registration and asks for the latest build. */
export async function registerWorker(
  url: string | null,
  serviceWorker: ServiceWorkerContainer | null = navigator.serviceWorker ?? null,
): Promise<void> {
  if (url === null || serviceWorker === null) {
    return;
  }

  try {
    const registration = await serviceWorker.register(url, { scope: "/", updateViaCache: "none" });

    await registration.update();
  } catch {
    // Installation can fail while offline; ordinary network navigation still works.
  }
}

/** Boot may arrive inline or over the dev API. Registration follows the decoded boot in both. */
export function watchWorkerRegistration(): void {
  if (import.meta.env.DEV) {
    return;
  }

  store.subscribe((state, previous) => {
    if (state.boot !== null && state.boot !== previous.boot) {
      void registerWorker(state.boot.serviceWorkerUrl);
    }
  });

  void registerWorker(store.getState().boot?.serviceWorkerUrl ?? null);
}
