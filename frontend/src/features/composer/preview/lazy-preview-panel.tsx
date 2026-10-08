import { type ComponentProps, Suspense } from "react";
import { lazyForUpdate as lazy } from "../../../service-worker/lazy.ts";
import { useOpenedOnce } from "../../../ui/opened-once.ts";
import type { PreviewPanel } from "./preview-panel.tsx";

const LoadedPanel = lazy(async () => {
  const module = await import("./preview-panel.tsx");

  return { default: module.PreviewPanel };
});

/** The server-rendered preview, loaded the first time it opens. */
export function LazyPreviewPanel(props: ComponentProps<typeof PreviewPanel>) {
  const opened = useOpenedOnce(props.open);

  return opened ? (
    <Suspense fallback={null}>
      <LoadedPanel {...props} />
    </Suspense>
  ) : null;
}
