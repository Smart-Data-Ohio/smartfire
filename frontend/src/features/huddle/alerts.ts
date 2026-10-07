/**
 * The page's call notices and incoming rings, wired to the browser: the call store says whether
 * the viewer is in a call, the sounds play through the viewer's quiet settings, and Join goes to
 * the room (through the navigator the huddle root registers) and joins its call.
 */
import type { HuddleRing } from "../../gen/HuddleRing.ts";
import { store } from "../../store/store.ts";
import { huddles } from "../../sync/huddles.ts";
import { callController } from "./call-controller.ts";
import { callStore } from "./call-store.ts";
import { CallNotices } from "./notices.ts";
import { IncomingCalls } from "./ring.ts";
import { playJoinSound, Ringer, soundsMuted } from "./sounds.ts";

/** In (or joining) this room's call: prejoin doesn't count, as in the classic notices. */
function inCall(roomId: number): boolean {
  const { roomId: callRoomId, phase } = callStore.getState();

  return (
    callRoomId === roomId &&
    (phase === "connecting" || phase === "connected" || phase === "reconnecting")
  );
}

/**
 * The hint a join starts with: a stage's role says whether to expect publishing (the token
 * decides); elsewhere there is none.
 */
export function joinHint(roomId: number): boolean | null {
  const state = store.getState();
  const row = state.sidebar.rows[roomId];

  if (row?.room.kind !== "stage") {
    return null;
  }

  const viewerId = state.me?.user.id;
  const member = state.stages[roomId]?.members.find((entry) => entry.userId === viewerId);
  const role = member?.role ?? row.membership.stageRole;

  return role !== null && role !== "listener";
}

type Navigator = (roomId: number) => Promise<void>;

let navigateTo: Navigator = async () => undefined;

/** The huddle root hands over the router's navigation. */
export function setCallNavigator(next: Navigator): void {
  navigateTo = next;
}

/** Goes to a room and joins its call (a banner's or a ring's Join). */
export async function goJoin(roomId: number, roomName: string): Promise<void> {
  await navigateTo(roomId);
  await callController.join(roomId, roomName, joinHint(roomId));
}

function notifyHidden(ring: HuddleRing): (() => void) | null {
  if (
    document.visibilityState !== "hidden" ||
    typeof Notification === "undefined" ||
    Notification.permission !== "granted"
  ) {
    return null;
  }

  try {
    const notification = new Notification(`${ring.callerName} started a huddle`, {
      body: `Join the huddle in ${ring.roomName}`,
      tag: `huddle-invitation-${ring.roomId}`,
    });

    notification.onclick = () => {
      window.focus();
      notification.close();
    };

    return () => notification.close();
  } catch {
    return null;
  }
}

const ringer = new Ringer();

export const callNotices = new CallNotices({
  inCall,
  playJoinSound: () => {
    if (!soundsMuted(store.getState().me, new Date())) {
      playJoinSound();
    }
  },
});

export const incomingCalls = new IncomingCalls({
  inCall,
  startRinging: () => ringer.start(),
  stopRinging: () => ringer.stop(),
  notify: notifyHidden,
  answer: (activityItemId, action) => {
    void huddles.answerRing(activityItemId, action);
  },
  join: (roomId, roomName) => {
    void goJoin(roomId, roomName);
  },
});
