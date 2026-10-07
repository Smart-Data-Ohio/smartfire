import { useEffect } from "react";
import {
  needsFetch,
  type Preview,
  type PreviewKind,
  type PreviewValues,
} from "../../store/cards.ts";
import { useStore } from "../../store/store.ts";

/**
 * A per-viewer preview from the store, fetched (through `load`) when the card mounts and the
 * store has none, or only a stale one. A failed fetch stays failed until the card's Retry;
 * `message.cards` drops the preview, so a mounted card fetches it again.
 */
export function usePreview<Kind extends PreviewKind>(
  kind: Kind,
  key: string,
  load: () => Promise<void>,
): Preview<PreviewValues[Kind]> | undefined {
  const preview: Preview<PreviewValues[Kind]> | undefined = useStore(
    (state) => state.cards.previews[kind][key],
  );

  const stale = needsFetch(preview, Date.now());

  useEffect(() => {
    if (stale) {
      // A failure lands in the store as the preview's error; nothing to do with it here.
      load().catch(() => undefined);
    }
  }, [stale, load]);

  return preview;
}
