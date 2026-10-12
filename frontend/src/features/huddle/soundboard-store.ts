/**
 * The soundboard's client state: the catalog (fetched on first use and kept for the session, as
 * the workspace icons are; an administrator's write replaces it, and a play of a sound this tab
 * doesn't know refetches it) and this browser's soundboard preferences (volume and the opt-out,
 * kept with the other huddle preferences in engine/preferences.ts).
 */
import { useStore } from "zustand";
import { createStore } from "zustand/vanilla";
import type { SoundboardCatalog } from "../../gen/SoundboardCatalog.ts";
import type { SoundboardSound } from "../../gen/SoundboardSound.ts";
import { soundboard } from "../../sync/huddles.ts";
import {
  loadSoundboardEnabled,
  loadSoundboardVolume,
  storeSoundboardEnabled,
  storeSoundboardVolume,
} from "./engine/preferences.ts";

export type CatalogLoad =
  | { readonly status: "idle" }
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly catalog: SoundboardCatalog };

export interface SoundboardState {
  readonly load: CatalogLoad;
  /** 0–100: how loud every sound plays here, on top of its own default volume. */
  readonly volume: number;
  /** False once the person turned soundboard sounds off: they neither hear nor play them. */
  readonly enabled: boolean;
}

export const soundboardStore = createStore<SoundboardState>()(() => ({
  load: { status: "idle" },
  volume: loadSoundboardVolume(),
  enabled: loadSoundboardEnabled(),
}));

let loading: Promise<SoundboardCatalog> | null = null;

/** The catalog: the one held, or one fetch shared by every caller (`fresh` skips the held one). */
export function loadSoundboardCatalog(fresh = false): Promise<SoundboardCatalog> {
  const held = soundboardStore.getState().load;

  if (!fresh && held.status === "ready") {
    return Promise.resolve(held.catalog);
  }

  if (loading !== null) {
    return loading;
  }

  if (held.status !== "ready") {
    soundboardStore.setState({ load: { status: "loading" } });
  }

  const request = soundboard.catalog().then(
    (catalog) => {
      if (loading === request) {
        loading = null;
        soundboardStore.setState({ load: { status: "ready", catalog } });
      }

      return catalog;
    },
    (error: Error) => {
      if (loading === request) {
        loading = null;

        // A failed refetch keeps the catalog already shown.
        if (soundboardStore.getState().load.status !== "ready") {
          soundboardStore.setState({ load: { status: "error", message: error.message } });
        }
      }

      throw error;
    },
  );

  loading = request;

  return request;
}

/** The server's catalog after an administrator's write: every picker shows it at once. */
export function replaceSoundboardCatalog(catalog: SoundboardCatalog): void {
  loading = null;
  soundboardStore.setState({ load: { status: "ready", catalog } });
}

/** A held sound by id, `undefined` when the catalog isn't loaded or doesn't have it. */
export function heldSound(soundId: number): SoundboardSound | undefined {
  const { load } = soundboardStore.getState();

  return load.status === "ready"
    ? load.catalog.sounds.find((sound) => sound.id === soundId)
    : undefined;
}

export function setSoundboardVolume(volume: number): void {
  const value = Math.max(0, Math.min(100, Math.round(volume)));

  storeSoundboardVolume(value);
  soundboardStore.setState({ volume: value });
}

export function setSoundboardEnabled(enabled: boolean): void {
  storeSoundboardEnabled(enabled);
  soundboardStore.setState({ enabled });
}

/** Forgets the catalog (tests, and a sign-out's fresh start). */
export function resetSoundboard(): void {
  loading = null;
  soundboardStore.setState({
    load: { status: "idle" },
    volume: loadSoundboardVolume(),
    enabled: loadSoundboardEnabled(),
  });
}

export function useSoundboard<T>(selector: (state: SoundboardState) => T): T {
  return useStore(soundboardStore, selector);
}
