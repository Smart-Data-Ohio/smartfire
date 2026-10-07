import { type ComponentProps, lazy, Suspense } from "react";
import { useOpenedOnce } from "../../ui/opened-once.ts";
import type { CreatePollDialog } from "./create-poll-dialog.tsx";

const LoadedDialog = lazy(() =>
  import("./create-poll-dialog.tsx").then((module) => ({ default: module.CreatePollDialog })),
);

/** The create-poll dialog, loaded the first time it opens. */
export function LazyCreatePollDialog(props: ComponentProps<typeof CreatePollDialog>) {
  const opened = useOpenedOnce(props.open);

  return opened ? (
    <Suspense fallback={null}>
      <LoadedDialog {...props} />
    </Suspense>
  ) : null;
}
