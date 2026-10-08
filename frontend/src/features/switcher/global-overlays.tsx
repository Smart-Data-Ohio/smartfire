import { useNavigate, useParams } from "@tanstack/react-router";
import { Suspense, useEffect, useEffectEvent, useState } from "react";
import { IS_APPLE } from "../../lib/shortcuts.ts";
import { lazyForUpdate as lazy } from "../../service-worker/lazy.ts";
import { ignoreModuleResourceLoadError } from "../../service-worker/update-required.ts";
import { store } from "../../store/store.ts";
import { useDestination } from "../shell/view-store.ts";
import { type GlobalShortcut, globalShortcut } from "./global-keys.ts";
import {
  closeOverlay,
  type Overlay,
  overlayChunks,
  overlayStore,
  toggleOverlay,
  useOverlay,
} from "./overlay-store.ts";
import { adjacentRoom, isUnreadStop, sidebarOrder } from "./room-navigation.ts";

const SwitcherDialog = lazy(overlayChunks.switcher);

const ShortcutsDialog = lazy(overlayChunks.shortcuts);

const NewDirectDialog = lazy(overlayChunks["new-direct"]);

const OVERLAY_FOR = {
  switcher: "switcher",
  shortcuts: "shortcuts",
  "new-direct": "new-direct",
} as const satisfies Partial<Record<GlobalShortcut, Overlay>>;

/**
 * Some other feature's modal (a confirm, the forward dialog) is up: keys leave it alone. While
 * one of these overlays is open, the open dialog is that overlay.
 */
function foreignModalOpen(): boolean {
  return overlayStore.getState().open === null && document.querySelector("dialog[open]") !== null;
}

/** How long after the shell mounts the overlays' chunks are fetched. */
const PRELOAD_DELAY_MS = 1500;

/** Warms the overlays' chunks once first paint is done, so the first ⌘K opens without a fetch. */
function usePreloadOverlays(): void {
  useEffect(() => {
    const timer = window.setTimeout(() => {
      for (const load of Object.values(overlayChunks)) {
        void load().catch(ignoreModuleResourceLoadError);
      }
    }, PRELOAD_DELAY_MS);

    return () => window.clearTimeout(timer);
  }, []);
}

/**
 * App-wide overlays and keys: the quick switcher (⌘K), the shortcuts dialog (⌘/), the new-DM
 * picker (⌘⇧K) and Alt+↑/↓ (Alt+Shift+↑/↓ for unread) through the sidebar's conversations.
 * Mounted once by the shell. Keys a focused control already handled (`defaultPrevented`) pass
 * through, and none of them fire over another feature's open dialog.
 */
export function GlobalOverlays() {
  const navigate = useNavigate();
  const params = useParams({ strict: false });
  const destination = useDestination();
  const open = useOverlay();
  const [seen, setSeen] = useState<ReadonlySet<Overlay>>(new Set());
  const currentRoomId = params.roomId ?? null;

  if (open !== null && !seen.has(open)) {
    setSeen(new Set([...seen, open]));
  }

  usePreloadOverlays();

  const step = (shortcut: GlobalShortcut) => {
    const rows = sidebarOrder(
      store.getState().sidebar,
      destination === "dms" ? (section) => section.key === "direct" : undefined,
    );

    const direction = shortcut === "next-room" || shortcut === "next-unread" ? 1 : -1;
    const unreadOnly = shortcut === "next-unread" || shortcut === "previous-unread";

    const target = adjacentRoom(
      rows,
      currentRoomId,
      direction,
      unreadOnly ? isUnreadStop : undefined,
    );

    if (target !== null) {
      void navigate({ to: "/r/$roomId", params: { roomId: target } });
    }
  };

  const onKeyDown = useEffectEvent((event: KeyboardEvent) => {
    const shortcut = event.defaultPrevented ? null : globalShortcut(event, IS_APPLE);

    if (shortcut === null || foreignModalOpen()) {
      return;
    }

    if (shortcut === "switcher" || shortcut === "shortcuts" || shortcut === "new-direct") {
      event.preventDefault();

      if (!event.repeat) {
        toggleOverlay(OVERLAY_FOR[shortcut]);
      }

      return;
    }

    if (overlayStore.getState().open === null) {
      event.preventDefault();
      step(shortcut);
    }
  });

  useEffect(() => {
    const listener = (event: KeyboardEvent) => onKeyDown(event);

    window.addEventListener("keydown", listener);

    return () => window.removeEventListener("keydown", listener);
  }, []);

  const onOpenChange = (overlay: Overlay) => (next: boolean) => {
    if (!next) {
      closeOverlay(overlay);
    }
  };

  return (
    <>
      <Suspense fallback={null}>
        {seen.has("switcher") ? (
          <SwitcherDialog open={open === "switcher"} onOpenChange={onOpenChange("switcher")} />
        ) : null}
      </Suspense>
      <Suspense fallback={null}>
        {seen.has("shortcuts") ? (
          <ShortcutsDialog open={open === "shortcuts"} onOpenChange={onOpenChange("shortcuts")} />
        ) : null}
      </Suspense>
      <Suspense fallback={null}>
        {seen.has("new-direct") ? (
          <NewDirectDialog open={open === "new-direct"} onOpenChange={onOpenChange("new-direct")} />
        ) : null}
      </Suspense>
    </>
  );
}
