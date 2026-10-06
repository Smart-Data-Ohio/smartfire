/**
 * Wires the call controller to the page, once, from the app shell: the call's keyboard
 * shortcuts, focus and visibility, the page going away, device changes, the room's presence,
 * the S5 sync signals, another account signing in, and the presence poll. Renders the device
 * check while one is open.
 */
import { lazy, Suspense, useEffect } from "react";
import { store } from "../../store/store.ts";
import { onHuddleSignal } from "../../sync/huddles.ts";
import { callController } from "./call-controller.ts";
import { callStore, useCall } from "./call-store.ts";
import { startPresencePolling } from "./presence.ts";
import "./huddle.css";

const PrejoinDialog = lazy(() => import("./prejoin-dialog.tsx"));

export function HuddleRoot() {
  const prejoin = useCall((state) => state.phase === "prejoin");

  useEffect(() => {
    const stopPolling = startPresencePolling();

    const keyDown = (event: KeyboardEvent) => {
      callController.keyDown(event);
    };

    const keyUp = (event: KeyboardEvent) => {
      callController.keyUp(event);
    };

    const blurred = () => {
      callController.blurred();
    };

    const visibility = () => {
      if (document.visibilityState === "visible") {
        callController.visible();
      } else {
        callController.hidden();
      }
    };

    const pageHide = () => {
      callController.endForPageChange();
    };

    const devicesChanged = () => {
      callController.devicesChanged();
    };

    window.addEventListener("keydown", keyDown);
    window.addEventListener("keyup", keyUp);
    window.addEventListener("blur", blurred);
    window.addEventListener("pagehide", pageHide);
    document.addEventListener("visibilitychange", visibility);
    navigator.mediaDevices?.addEventListener("devicechange", devicesChanged);

    // The call room's presence maps LiveKit identities to people (volumes, names).
    const stopPresence = store.subscribe((state, previous) => {
      const roomId = callStore.getState().roomId;

      if (roomId !== null && state.huddles[roomId] !== previous.huddles[roomId]) {
        callController.presenceChanged();
      }
    });

    // A second account in this browser ends the first one's call.
    const stopAccount = store.subscribe((state, previous) => {
      const before = previous.me?.user.id ?? null;
      const after = state.me?.user.id ?? null;

      if (before !== null && after !== before) {
        callController.endForPageChange();
      }
    });

    const stopSignals = onHuddleSignal((signal) => {
      if (signal.type === "huddle.role") {
        void callController.roleChanged(
          signal.data.roomId,
          signal.data.stageRole,
          signal.data.serverMuted,
        );
      } else if (signal.type === "stage.stream.stopped") {
        void callController.streamStopped(signal.data.roomId);
      }
    });

    return () => {
      stopPolling();
      stopPresence();
      stopAccount();
      stopSignals();
      window.removeEventListener("keydown", keyDown);
      window.removeEventListener("keyup", keyUp);
      window.removeEventListener("blur", blurred);
      window.removeEventListener("pagehide", pageHide);
      document.removeEventListener("visibilitychange", visibility);
      navigator.mediaDevices?.removeEventListener("devicechange", devicesChanged);
    };
  }, []);

  return prejoin ? (
    <Suspense fallback={null}>
      <PrejoinDialog />
    </Suspense>
  ) : null;
}
