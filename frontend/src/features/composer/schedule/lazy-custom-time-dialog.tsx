import { type ComponentProps, lazy, Suspense } from "react";
import { useOpenedOnce } from "../../../ui/opened-once.ts";
import type { CustomTimeDialog } from "./custom-time-dialog.tsx";

const LoadedDialog = lazy(() =>
  import("./custom-time-dialog.tsx").then((module) => ({ default: module.CustomTimeDialog })),
);

/** The custom-time picker, loaded the first time it opens. */
export function LazyCustomTimeDialog(props: ComponentProps<typeof CustomTimeDialog>) {
  const opened = useOpenedOnce(props.open);

  return opened ? (
    <Suspense fallback={null}>
      <LoadedDialog {...props} />
    </Suspense>
  ) : null;
}
