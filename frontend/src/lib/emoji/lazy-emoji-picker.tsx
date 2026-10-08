import { Suspense } from "react";
import { lazyForUpdate as lazy } from "../../service-worker/lazy.ts";
import {
  ignoreModuleResourceLoadError,
  loadForUpdate,
} from "../../service-worker/update-required.ts";
import { Spinner } from "../../ui/button.tsx";
import type { EmojiPickerProps } from "./emoji-picker.tsx";
import "./lazy-emoji-picker.css";

/** The picker and its catalogue load as their own chunks the first time a picker opens. */
const EmojiPicker = lazy(() => import("./emoji-picker.tsx"));

/** Holds the picker's size while its chunk arrives, so the popover doesn't jump. */
function PickerFallback() {
  return (
    <div className="emoji-picker-fallback" role="status">
      <Spinner label="Loading emoji" />
    </div>
  );
}

/** The emoji picker behind `React.lazy`: drop it into a Popover. */
export function LazyEmojiPicker(props: EmojiPickerProps) {
  return (
    <Suspense fallback={<PickerFallback />}>
      <EmojiPicker {...props} />
    </Suspense>
  );
}

/** Starts fetching the picker's chunks early (on hover of an "add reaction" button). */
export function preloadEmojiPicker(): void {
  void loadForUpdate(() => import("./emoji-picker.tsx")).catch(ignoreModuleResourceLoadError);
  void loadForUpdate(() => import("./data.ts"))
    .then((module) => module.loadEmojiData())
    .catch(ignoreModuleResourceLoadError);
}
