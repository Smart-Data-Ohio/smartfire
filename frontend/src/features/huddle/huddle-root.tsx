/**
 * Wires the call controller to the page, once, from the app shell: the call's keyboard
 * shortcuts, focus and visibility, the page going away, device changes, the room's presence,
 * the S5 sync signals, another account signing in, and the presence poll. Renders the device
 * check while one is open.
 */
import { useNavigate } from "@tanstack/react-router";
import { lazy, Suspense, useEffect } from "react";
import { store } from "../../store/store.ts";
import { onHuddleSignal } from "../../sync/huddles.ts";
import { callNotices, incomingCalls, setCallNavigator } from "./alerts.ts";
import { CallToasts, RingBanner } from "./call-alerts.tsx";
import { callController } from "./call-controller.ts";
import { callStore, useCall } from "./call-store.ts";
import { noticeStore } from "./notices.ts";
import { startPresencePolling } from "./presence.ts";

function noticeBanners() {
  return noticeStore.getState().banners;
}

import "./huddle.css";

const PrejoinDialog = lazy(() => import("./prejoin-dialog.tsx"));

export function HuddleRoot() {
  const prejoin = useCall((state) => state.phase === "prejoin");
  const navigate = useNavigate();

  useEffect(() => {
    setCallNavigator((roomId) => navigate({ to: "/r/$roomId", params: { roomId } }));
  }, [navigate]);

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

    // A second account in this browser ends the first one's call (and its notices and ring).
    const stopAccount = store.subscribe((state, previous) => {
      const before = previous.me?.user.id ?? null;
      const after = state.me?.user.id ?? null;

      if (before !== null && after !== before) {
        callController.endForPageChange();
        callNotices.reset();
        incomingCalls.hide();
      }
    });

    // Presence is authoritative for join banners: an emptied call clears its banner.
    const stopBanners = store.subscribe((state, previous) => {
      if (state.huddles === previous.huddles) {
        return;
      }

      for (const roomId of Object.keys(noticeBanners())) {
        const id = Number(roomId);

        callNotices.presenceChanged(
          id,
          (state.huddles[id]?.participants ?? []).map((participant) => participant.userId),
        );
      }
    });

    // Joining a call answers its banner and its ring.
    const stopCall = callStore.subscribe((state, previous) => {
      const active =
        state.phase === "connecting" ||
        state.phase === "connected" ||
        state.phase === "reconnecting";

      if (
        active &&
        state.roomId !== null &&
        (state.roomId !== previous.roomId || state.phase !== previous.phase)
      ) {
        callNotices.callActive(state.roomId);
        incomingCalls.callActive(state.roomId);
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
      } else if (signal.type === "huddle.notice") {
        callNotices.received(signal.data);
      } else if (signal.type === "huddle.ring") {
        incomingCalls.received(signal.data);
      }
    });

    return () => {
      stopPolling();
      stopPresence();
      stopAccount();
      stopBanners();
      stopCall();
      stopSignals();
      window.removeEventListener("keydown", keyDown);
      window.removeEventListener("keyup", keyUp);
      window.removeEventListener("blur", blurred);
      window.removeEventListener("pagehide", pageHide);
      document.removeEventListener("visibilitychange", visibility);
      navigator.mediaDevices?.removeEventListener("devicechange", devicesChanged);
    };
  }, []);

  return (
    <>
      <RingBanner />
      <CallToasts />
      {prejoin ? (
        <Suspense fallback={null}>
          <PrejoinDialog />
        </Suspense>
      ) : null}
    </>
  );
}
