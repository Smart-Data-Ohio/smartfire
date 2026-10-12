/**
 * The sticker catalog on the client: fetched on first use and kept for the session, as the
 * soundboard's is. An administrator's write replaces it, and opening the picker refetches it in
 * the background so another administrator's changes show.
 */
import { useStore } from "zustand";
import { createStore } from "zustand/vanilla";
import type { Sticker } from "../../gen/Sticker.ts";
import type { StickerCatalog } from "../../gen/StickerCatalog.ts";
import { composerActions } from "../../sync/composer-actions.ts";

export type StickerLoad =
  | { readonly status: "idle" }
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly catalog: StickerCatalog };

export const stickerStore = createStore<{ readonly load: StickerLoad }>()(() => ({
  load: { status: "idle" },
}));

let loading: Promise<StickerCatalog> | null = null;

/** The catalog: the one held, or one fetch shared by every caller (`fresh` skips the held one). */
export function loadStickerCatalog(fresh = false): Promise<StickerCatalog> {
  const held = stickerStore.getState().load;

  if (!fresh && held.status === "ready") {
    return Promise.resolve(held.catalog);
  }

  if (loading !== null) {
    return loading;
  }

  if (held.status !== "ready") {
    stickerStore.setState({ load: { status: "loading" } });
  }

  const request = composerActions.stickerCatalog().then(
    (catalog) => {
      if (loading === request) {
        loading = null;
        stickerStore.setState({ load: { status: "ready", catalog } });
      }

      return catalog;
    },
    (error: Error) => {
      if (loading === request) {
        loading = null;

        // A failed refetch keeps the catalog already shown.
        if (stickerStore.getState().load.status !== "ready") {
          stickerStore.setState({ load: { status: "error", message: error.message } });
        }
      }

      throw error;
    },
  );

  loading = request;

  return request;
}

/** The server's catalog after an administrator's write: every picker shows it at once. */
export function replaceStickerCatalog(catalog: StickerCatalog): void {
  loading = null;
  stickerStore.setState({ load: { status: "ready", catalog } });
}

/** Forgets the catalog (tests, and a sign-out's fresh start). */
export function resetStickers(): void {
  loading = null;
  stickerStore.setState({ load: { status: "idle" } });
}

export function useStickerLoad(): StickerLoad {
  return useStore(stickerStore, (state) => state.load);
}

/** The held stickers; empty until the catalog loads. */
export function useHeldStickers(): readonly Sticker[] {
  return useStore(stickerStore, (state) =>
    state.load.status === "ready" ? state.load.catalog.stickers : NONE,
  );
}

const NONE: readonly Sticker[] = [];

/**
 * The stickers matching `query`: each of its words must appear in the sticker's name or tags
 * (tags may hold a related emoji, matched as typed), case aside. A blank query matches them all.
 */
export function searchStickers(list: readonly Sticker[], query: string): readonly Sticker[] {
  const terms = query.toLocaleLowerCase().split(/\s+/).filter(Boolean);

  if (terms.length === 0) {
    return list;
  }

  return list.filter((sticker) => {
    const text = `${sticker.name} ${sticker.tags}`.toLocaleLowerCase();

    return terms.every((term) => text.includes(term));
  });
}
