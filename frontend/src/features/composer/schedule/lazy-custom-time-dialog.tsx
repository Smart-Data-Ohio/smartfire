import { type ComponentProps, Suspense } from "react";
import { lazyForUpdate as lazy } from "../../../service-worker/lazy.ts";
import { useOpenedOnce } from "../../../ui/opened-once.ts";
import type { CustomTimeDialog } from "./custom-time-dialog.tsx";

const LoadedDialog = lazy(async () => {
  const module = await import("./custom-time-dialog.tsx");

  return { default: module.CustomTimeDialog };
});

/** The custom-time picker, loaded the first time it opens. */
export function LazyCustomTimeDialog(props: ComponentProps<typeof CustomTimeDialog>) {
  const opened = useOpenedOnce(props.open);

  return opened ? (
    <Suspense fallback={null}>
      <LoadedDialog {...props} />
    </Suspense>
  ) : null;
}
