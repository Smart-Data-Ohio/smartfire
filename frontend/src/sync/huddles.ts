/**
 * What React and the call controller use for huddles and stages: plain promises over the S5
 * endpoints, landing each answer in the store, plus the S5 sync events as plain signals.
 * Failures reject with an `ActionError` whose `tag` names the API error (`NotFound`,
 * `Forbidden`, `Unavailable`…) and whose message is fit to show.
 */
import * as api from "../api/huddle-endpoints.ts";
import type { HuddleCredentials } from "../gen/HuddleCredentials.ts";
import type { HuddleDetail } from "../gen/HuddleDetail.ts";
import type { HuddleModeration } from "../gen/HuddleModeration.ts";
import type { StageRole } from "../gen/StageRole.ts";
import type { StageStream } from "../gen/StageStream.ts";
import type { StreamQuality } from "../gen/StreamQuality.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { mutations } from "../store/store.ts";
import { runAction, runtime } from "./runtime.ts";
import { onSyncEvents } from "./signals.ts";
import { Topics } from "./topics.ts";

export const huddles = {
  /**
   * Keeps the call's room topic subscribed while the call lasts, wherever the viewer navigates:
   * the stage roster, its stream and the room's call notices arrive there. Ref-counted with the
   * room view's own hold, so call `releaseRoom` once per `holdRoom`.
   */
  holdRoom(roomId: number): void {
    runtime.runFork(Topics.use((topics) => topics.acquire(`room:${roomId}`)));
  },

  releaseRoom(roomId: number): void {
    runtime.runFork(Topics.use((topics) => topics.release(`room:${roomId}`)));
  },

  /** Every call in the viewer's rooms; rooms it leaves out have emptied. */
  refreshPresence: async (): Promise<void> => {
    mutations.loadHuddlePresence(await runAction(api.huddles()));
  },

  /** One room's call; rejects with `NotFound`, `Forbidden` or `Unauthorized` once access ends. */
  room: async (roomId: number): Promise<HuddleDetail> => {
    const detail = await runAction(api.huddle(roomId));

    mutations.mergeUsers(detail.users);
    mutations.setHuddlePresence(detail.presence);

    return detail;
  },

  /** A fresh grant and its LiveKit credentials. */
  join: (roomId: number): Promise<HuddleCredentials> => runAction(api.joinHuddle(roomId)),

  /** Reports this session out of the call. Never rejects: leaving works offline too. */
  leave: async (roomId: number): Promise<void> => {
    await runAction(api.leaveHuddle(roomId)).catch(() => undefined);
  },

  /** The same report as a `keepalive` request, for a page that is going away. */
  leaveOnUnload(roomId: number): void {
    runtime.runFork(api.leaveOnUnload(roomId));
  },

  moderate: (roomId: number, membershipId: number, action: HuddleModeration): Promise<void> =>
    runAction(api.moderateHuddle(roomId, { membershipId, action })),

  /** Loads a stage's roster, stream and profiles into the store. */
  stage: async (roomId: number): Promise<void> => {
    mutations.loadStageDetail(await runAction(api.stage(roomId)));
  },

  changeRole: async (roomId: number, membershipId: number, role: StageRole): Promise<void> => {
    mutations.setStage(await runAction(api.changeStageRole(roomId, membershipId, role)));
  },

  raiseHand: async (roomId: number): Promise<void> => {
    mutations.setStage(await runAction(api.raiseHand(roomId)));
  },

  /** The viewer's own hand (`null`), or a host lowering someone else's. */
  lowerHand: async (roomId: number, membershipId: number | null): Promise<void> => {
    mutations.setStage(await runAction(api.lowerHand(roomId, membershipId)));
  },

  startStream: (roomId: number, quality: StreamQuality): Promise<StageStream> =>
    runAction(api.startStream(roomId, quality)),

  stopStream: (roomId: number, streamId: number | null): Promise<void> =>
    runAction(api.stopStream(roomId, streamId)),

  /** Ends a stream from a page that is going away. */
  stopStreamOnUnload(roomId: number, streamId: number | null): void {
    runtime.runFork(api.stopStreamOnUnload(roomId, streamId));
  },

  /** Answers (`handled`) or dismisses (`read`) a ring's inbox item. Never rejects. */
  answerRing: async (activityItemId: number, action: "handled" | "read"): Promise<void> => {
    await runAction(api.answerRing(activityItemId, action)).catch(() => undefined);
  },
};

/** The sync events the huddle features react to. */
export type HuddleSignal = Extract<
  SyncEvent,
  {
    readonly type:
      | "huddle.presence"
      | "huddle.role"
      | "huddle.notice"
      | "huddle.ring"
      | "stage.updated"
      | "stage.stream.stopped";
  }
>;

function isHuddleSignal(event: SyncEvent): event is HuddleSignal {
  switch (event.type) {
    case "huddle.presence":
    case "huddle.role":
    case "huddle.notice":
    case "huddle.ring":
    case "stage.updated":
    case "stage.stream.stopped":
      return true;
    default:
      return false;
  }
}

/** Calls `listener` with each S5 event as it is applied (after the store has it). */
export function onHuddleSignal(listener: (signal: HuddleSignal) => void): () => void {
  return onSyncEvents((events) => {
    for (const event of events) {
      if (isHuddleSignal(event)) {
        listener(event);
      }
    }
  });
}
