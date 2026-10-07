import { useSyncExternalStore } from "react";

/**
 * Why a part of the app couldn't load: a deploy replaced this tab's build ("updated"), or the
 * load failed for some other reason, such as being offline ("failed").
 */
export type AppUpdateKind = "updated" | "failed";

let required = false;

let kind: AppUpdateKind | null = null;

let checking: Promise<void> | null = null;

const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);

  return () => listeners.delete(listener);
}

function notify(): void {
  for (const listener of listeners) {
    listener();
  }
}

export const appUpdateRequired = () => required;

export const appUpdateKind = () => kind;

/** The entry module a page names, resolved against that page's address. */
function entryScript(doc: Document, base: string): string | null {
  const src = doc
    .querySelector<HTMLScriptElement>('script[type="module"][src]')
    ?.getAttribute("src");

  return src ? new URL(src, base).href : null;
}

/** How long the newer-build probe may take, body included, before it counts as no answer. */
export const UPDATE_PROBE_TIMEOUT_MS = 8000;

/** `promise`, or `fallback` once `ms` pass without it settling. */
function within<T>(promise: Promise<T>, ms: number, fallback: T): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;

  const timeout = new Promise<T>((resolve) => {
    timer = setTimeout(() => resolve(fallback), ms);
  });

  return Promise.race([promise, timeout]).finally(() => clearTimeout(timer));
}

/**
 * Whether the server now hands out a different build than this tab runs. Being offline, a
 * failed or slow request (past `timeoutMs`, body included), or a page without an entry module
 * all count as no newer build.
 */
export async function newerBuildAvailable(
  network: typeof fetch = (...args) => fetch(...args),
  doc: Document = document,
  timeoutMs: number = UPDATE_PROBE_TIMEOUT_MS,
): Promise<boolean> {
  const current = entryScript(doc, doc.baseURI);

  if (current === null) {
    return false;
  }

  const controller = new AbortController();

  const probe = async (): Promise<boolean> => {
    const page = new URL("/app/", doc.baseURI).href;

    const response = await network(page, {
      cache: "no-store",
      credentials: "same-origin",
      headers: { Accept: "text/html" },
      signal: controller.signal,
    });

    if (!response.ok) {
      return false;
    }

    const served = entryScript(
      new DOMParser().parseFromString(await response.text(), "text/html"),
      response.url || page,
    );

    return served !== null && served !== current;
  };

  try {
    return await within(
      probe().catch(() => false),
      timeoutMs,
      false,
    );
  } finally {
    // A probe that is still waiting, on headers or on the body, stops here.
    controller.abort();
  }
}

/**
 * A failed module load leaves the current page intact until its person chooses to reload. The
 * strip opens at once with the plain "couldn't load" wording, and switches to saying Smartfire
 * was updated only once the server confirms a different build. The probe is time-bounded, and a
 * later failure re-checks, since a deploy can land after an offline blip.
 */
export function requireAppUpdate(check: () => Promise<boolean> = newerBuildAvailable): void {
  if (!required || kind === null) {
    required = true;
    kind ??= "failed";
    notify();
  }

  if (kind === "updated" || checking !== null) {
    return;
  }

  // The default probe bounds itself; this also bounds any other check, so one that never
  // settles can't block later re-checks.
  checking = within(
    check().catch(() => false),
    UPDATE_PROBE_TIMEOUT_MS + 1000,
    false,
  ).then((newer) => {
    checking = null;

    if (newer && kind !== "updated") {
      kind = "updated";
      notify();
    }
  });
}

export function useAppUpdateRequired(): boolean {
  return useSyncExternalStore(subscribe, appUpdateRequired, appUpdateRequired);
}

export function useAppUpdateKind(): AppUpdateKind | null {
  return useSyncExternalStore(subscribe, appUpdateKind, appUpdateKind);
}

export function reloadForUpdate(): void {
  window.location.reload();
}

/** Native module resource failures are distinct from errors evaluating the fetched module. */
export function isModuleResourceLoadError(cause: unknown): cause is Error {
  if (!(cause instanceof Error)) {
    return false;
  }

  if (cause.name === "TypeError") {
    return (
      /^Failed to fetch dynamically imported module(?::\s*\S+)?$/.test(cause.message) ||
      /^error loading dynamically imported module(?::\s*\S+)?$/i.test(cause.message) ||
      cause.message === "Importing a module script failed."
    );
  }

  return cause.name === "Error" && /^Unable to preload CSS for \S+$/.test(cause.message);
}

/** Optional preloads absorb a missing resource, while ordinary evaluation failures propagate. */
export function ignoreModuleResourceLoadError(cause: unknown): void {
  if (!isModuleResourceLoadError(cause)) {
    throw cause;
  }
}

export function watchAppUpdateErrors(target: EventTarget = window): () => void {
  const failed = (event: Event) => {
    if ("payload" in event && isModuleResourceLoadError(event.payload)) {
      requireAppUpdate();
    }

    // Keep Vite's rejection intact so the originating loader receives the actual failure.
    // Preventing it would resolve undefined, obscuring the cause in named-export loaders.
  };

  target.addEventListener("vite:preloadError", failed);

  return () => target.removeEventListener("vite:preloadError", failed);
}

/** Wrap only the import, so API, decoder and component errors keep their existing handling. */
export async function loadForUpdate<Module extends object>(
  load: () => Promise<Module>,
): Promise<Module> {
  try {
    const module = await load();

    if (module === undefined) {
      throw new Error("The module loader returned no module");
    }

    return module;
  } catch (error) {
    if (isModuleResourceLoadError(error)) {
      requireAppUpdate();
    }

    throw error;
  }
}
