/**
 * The GIF feature's client state that outlives one picker: whether this server has GIFs at all
 * (`GET /api/v1/gifs/config`, fetched once and shared, as the sticker catalog is), plus the
 * picker's masonry layout — which column each GIF falls in, and where the keyboard goes next.
 * Search results themselves stay in the picker; they change with every keystroke.
 */
import { useStore } from "zustand";
import { createStore } from "zustand/vanilla";
import type { GifConfig } from "../../gen/GifConfig.ts";
import type { GifItem } from "../../gen/GifItem.ts";
import { gifActions } from "../../sync/gif-actions.ts";

/** GIFs per masonry column set, in a popover and in a phone's sheet alike. */
export const GIF_COLUMNS = 2;

export type GifConfigLoad =
  | { readonly status: "idle" }
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly enabled: boolean };

export const gifStore = createStore<{ readonly load: GifConfigLoad }>()(() => ({
  load: { status: "idle" },
}));

let loading: Promise<GifConfig> | null = null;

/** The server's GIF config: the one held, or one fetch shared by every caller. */
export function loadGifConfig(): Promise<GifConfig> {
  const held = gifStore.getState().load;

  if (held.status === "ready") {
    return Promise.resolve({ enabled: held.enabled });
  }

  if (loading !== null) {
    return loading;
  }

  gifStore.setState({ load: { status: "loading" } });

  const request = gifActions.config().then(
    (config) => {
      if (loading === request) {
        loading = null;
        gifStore.setState({ load: { status: "ready", enabled: config.enabled } });
      }

      return config;
    },
    (error: Error) => {
      if (loading === request) {
        loading = null;
        gifStore.setState({ load: { status: "error", message: error.message } });
      }

      throw error;
    },
  );

  loading = request;

  return request;
}

/** Forgets the config (tests, and a sign-out's fresh start). */
export function resetGifConfig(): void {
  loading = null;
  gifStore.setState({ load: { status: "idle" } });
}

export function useGifConfig(): GifConfigLoad {
  return useStore(gifStore, (state) => state.load);
}

/** Whether the GIF button shows: only once the server says GIFs are on. */
export function useGifEnabled(): boolean {
  return useStore(gifStore, (state) => state.load.status === "ready" && state.load.enabled);
}

/** How much column a GIF takes for its width (`height / width`); a square when KLIPY didn't say. */
function gifWeight(item: GifItem): number {
  if (item.width <= 0 || item.height <= 0) {
    return 1;
  }

  return Math.min(2.5, Math.max(0.4, item.height / item.width));
}

/** The CSS `aspect-ratio` for a GIF's cell, so it holds its shape before the image loads. */
export function gifAspectRatio(item: GifItem): string {
  if (item.width <= 0 || item.height <= 0) {
    return "1 / 1";
  }

  return `${item.width} / ${item.height}`;
}

/**
 * Deals the GIFs into `columns` masonry columns, each new one into the shortest column so far
 * (by summed height-for-width): the columns end level whatever shapes arrive, and the order in
 * each column is the order the GIFs came in.
 */
export function distributeGifs(
  items: readonly GifItem[],
  columns = GIF_COLUMNS,
): readonly (readonly GifItem[])[] {
  const dealt: GifItem[][] = Array.from({ length: columns }, () => []);
  const weights: number[] = Array.from({ length: columns }, () => 0);

  for (const item of items) {
    let shortest = 0;

    for (let column = 1; column < columns; column += 1) {
      if ((weights[column] ?? 0) < (weights[shortest] ?? 0)) {
        shortest = column;
      }
    }

    dealt[shortest]?.push(item);
    weights[shortest] = (weights[shortest] ?? 0) + gifWeight(item);
  }

  return dealt;
}

/**
 * Where the active GIF goes for an arrow key: down or up its own column, or across to the GIF at
 * the same depth in the next column (the last one when that column is shorter). The active GIF
 * itself when the move would leave the grid, `null` for any other key (or an unknown slug).
 */
export function moveGifFocus(
  columns: readonly (readonly GifItem[])[],
  slug: string,
  key: string,
): string | null {
  let column = -1;
  let row = -1;

  for (let index = 0; index < columns.length; index += 1) {
    const found = columns[index]?.findIndex((item) => item.slug === slug) ?? -1;

    if (found !== -1) {
      column = index;
      row = found;
    }
  }

  if (column === -1) {
    return null;
  }

  const own = columns[column] ?? [];

  switch (key) {
    case "ArrowDown":
      return own[Math.min(own.length - 1, row + 1)]?.slug ?? null;
    case "ArrowUp":
      return own[Math.max(0, row - 1)]?.slug ?? null;
    case "ArrowRight":
    case "ArrowLeft": {
      const next = columns[column + (key === "ArrowRight" ? 1 : -1)];

      if (next === undefined || next.length === 0) {
        return slug;
      }

      return next[Math.min(next.length - 1, row)]?.slug ?? null;
    }

    default:
      return null;
  }
}
