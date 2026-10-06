import { type ComponentProps, lazy, Suspense } from "react";
import { useOpenedOnce } from "../../../ui/opened-once.ts";
import type { PreviewPanel } from "./preview-panel.tsx";

const LoadedPanel = lazy(() =>
  import("./preview-panel.tsx").then((module) => ({ default: module.PreviewPanel })),
);

/** The server-rendered preview, loaded the first time it opens. */
export function LazyPreviewPanel(props: ComponentProps<typeof PreviewPanel>) {
  const opened = useOpenedOnce(props.open);

  return opened ? (
    <Suspense fallback={null}>
      <LoadedPanel {...props} />
    </Suspense>
  ) : null;
}
