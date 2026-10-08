import type { WorkerBuild } from "./runtime.ts";

type WorkerRequest =
  | { readonly requestMethod: "GET"; readonly requestMode: "navigate" }
  | { readonly requestMethod: "GET"; readonly urlPattern: string };

export interface NetworkRoute {
  readonly condition: { readonly not: { readonly or: readonly WorkerRequest[] } };
  readonly source: "network";
}

interface RoutingInstallEvent extends Event {
  addRoutes?(route: NetworkRoute): Promise<void>;
}

/** Pass-through requests reach the network without waking the worker during a script swap. */
export function networkRoute(origin: string, build: WorkerBuild): NetworkRoute {
  const classicAssets = new URL("/assets/", origin).href;
  const offline = new URL(build.offline, origin).href;
  const spaAssets = new URL(".", new URL(build.page, origin)).href;
  const patterns = new Set([`${classicAssets}*`, `${spaAssets}*`, offline]);

  // The build's precache lives under assets/ plus offline.html. Preserve any future entries
  // outside those paths without adding one condition for every emitted chunk.
  for (const path of build.precache) {
    const url = new URL(path, origin).href;

    if (!url.startsWith(classicAssets) && !url.startsWith(spaAssets) && url !== offline) {
      patterns.add(url);
    }
  }

  const handled: WorkerRequest[] = [{ requestMethod: "GET", requestMode: "navigate" }];

  for (const urlPattern of patterns) {
    handled.push({ requestMethod: "GET", urlPattern });
  }

  return { condition: { not: { or: handled } }, source: "network" };
}

/** addRoutes must be called synchronously during install, before its first awaited work. */
export async function installNetworkRoute(
  event: RoutingInstallEvent,
  origin: string,
  build: WorkerBuild,
): Promise<void> {
  if (event.addRoutes === undefined) {
    return;
  }

  try {
    await event.addRoutes(networkRoute(origin, build));
  } catch {
    // Older implementations expose addRoutes but reject not/or; keep the fetch handler.
  }
}
