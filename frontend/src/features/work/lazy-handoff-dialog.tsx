import { type ComponentProps, lazy, Suspense } from "react";
import { useOpenedOnce } from "../../ui/opened-once.ts";
import type { HandoffDialog } from "./handoff-dialog.tsx";

const LoadedDialog = lazy(() =>
  import("./handoff-dialog.tsx").then((module) => ({ default: module.HandoffDialog })),
);

/** The handoff dialog, loaded the first time it opens. */
export function LazyHandoffDialog(props: ComponentProps<typeof HandoffDialog>) {
  const opened = useOpenedOnce(props.open);

  return opened ? (
    <Suspense fallback={null}>
      <LoadedDialog {...props} />
    </Suspense>
  ) : null;
}
