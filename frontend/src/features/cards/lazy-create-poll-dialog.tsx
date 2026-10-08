import { type ComponentProps, Suspense } from "react";
import { lazyForUpdate as lazy } from "../../service-worker/lazy.ts";
import { useOpenedOnce } from "../../ui/opened-once.ts";
import type { CreatePollDialog } from "./create-poll-dialog.tsx";

const LoadedDialog = lazy(async () => {
  const module = await import("./create-poll-dialog.tsx");

  return { default: module.CreatePollDialog };
});

/** The create-poll dialog, loaded the first time it opens. */
export function LazyCreatePollDialog(props: ComponentProps<typeof CreatePollDialog>) {
  const opened = useOpenedOnce(props.open);

  return opened ? (
    <Suspense fallback={null}>
      <LoadedDialog {...props} />
    </Suspense>
  ) : null;
}
