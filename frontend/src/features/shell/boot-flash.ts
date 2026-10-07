import { useEffect } from "react";
import type { Boot } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { toast } from "../../ui/toast-store.ts";

/** Boot data whose flash has been shown, so Strict Mode's second effect doesn't toast it twice. */
const shown = new WeakSet<Boot>();

/** Toasts the notice or alert a server redirect left for this page load, once. */
export function showBootFlash(boot: Boot | null): void {
  const flash = boot?.flash;

  if (boot === null || flash == null || shown.has(boot)) {
    return;
  }

  shown.add(boot);
  toast({ title: flash.message, tone: flash.kind === "alert" ? "danger" : "success" });
}

/** The shell's hook: shows the boot flash once boot data has loaded. */
export function useBootFlash(): void {
  const boot = useStore((state) => state.boot);

  useEffect(() => {
    showBootFlash(boot);
  }, [boot]);
}
