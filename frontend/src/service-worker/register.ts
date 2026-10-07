import { store } from "../store/store.ts";
import { PAGE_BUILD, requestsPageBuild } from "./messages.ts";
import { watchAppUpdateErrors } from "./update-required.ts";

function watchPageBuild(serviceWorker: ServiceWorkerContainer): void {
  // import.meta.url is the actual content-hashed entry this page loaded, even after a new
  // worker claims it. Reading the controller's build would incorrectly relabel an old page.
  const message = { kind: PAGE_BUILD, page: import.meta.url };
  const report = () => serviceWorker.controller?.postMessage(message);

  serviceWorker.addEventListener("controllerchange", report);
  serviceWorker.addEventListener("message", (event) => {
    if (requestsPageBuild(event)) {
      event.source?.postMessage(message);
    }
  });
  report();
}

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
  watchAppUpdateErrors();

  if (import.meta.env.DEV) {
    return;
  }

  if (navigator.serviceWorker !== undefined) {
    watchPageBuild(navigator.serviceWorker);
  }

  store.subscribe((state, previous) => {
    if (state.boot !== null && state.boot !== previous.boot) {
      void registerWorker(state.boot.serviceWorkerUrl);
    }
  });

  void registerWorker(store.getState().boot?.serviceWorkerUrl ?? null);
}
