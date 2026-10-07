import { useSyncExternalStore } from "react";

let required = false;

const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);

  return () => listeners.delete(listener);
}

export const appUpdateRequired = () => required;

/** A failed module load leaves the current page intact until its person chooses to reload. */
export function requireAppUpdate(): void {
  if (required) {
    return;
  }

  required = true;

  for (const listener of listeners) {
    listener();
  }
}

export function useAppUpdateRequired(): boolean {
  return useSyncExternalStore(subscribe, appUpdateRequired, appUpdateRequired);
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
