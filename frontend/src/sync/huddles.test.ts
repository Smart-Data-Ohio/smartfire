import { expect, it } from "vitest";
import type { HuddleNotice } from "../gen/HuddleNotice.ts";
import type { HuddleRing } from "../gen/HuddleRing.ts";
import { onHuddleSignal } from "./huddles.ts";
import { emitSyncEvents } from "./signals.ts";

it("consumes ignored caller publications without ringing or showing a join notice", () => {
  const received: number[] = [];
  const unsubscribe = onHuddleSignal((signal) => received.push(signal.seq));

  const ring: HuddleRing = {
    activityItemId: 41,
    event: "started",
    state: "unread",
    roomId: 5,
    roomName: "Room",
    callerName: "Caller",
    silent: false,
  };

  const notice: HuddleNotice = {
    kind: "joined",
    roomId: 5,
    roomName: "Room",
    userId: 2,
    userName: "Caller",
    inCall: true,
    rejoin: false,
  };

  try {
    emitSyncEvents([
      { seq: 1, topic: "user", type: "huddle.ring", data: ring },
      {
        seq: 2,
        topic: "user",
        type: "huddle.ring",
        data: { ...ring, silent: true, unchanged: true },
      },
      { seq: 3, topic: "user", type: "huddle.notice", data: notice },
      { seq: 4, topic: "user", type: "huddle.notice", data: { ...notice, unchanged: true } },
    ]);

    expect(received).toEqual([1, 3]);
  } finally {
    unsubscribe();
  }
});
