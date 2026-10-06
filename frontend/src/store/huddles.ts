/**
 * The S5 slices: who is in each room's call (`huddles`) and each stage's roster and stream
 * (`stages`). Pure transitions like the rest of the store; `store.ts` wraps them.
 */
import type { HuddlePresence } from "../gen/HuddlePresence.ts";
import type { HuddlePresenceList } from "../gen/HuddlePresenceList.ts";
import type { StageDetail } from "../gen/StageDetail.ts";
import type { StageState } from "../gen/StageState.ts";
import { mergeUserList } from "./ordering.ts";
import type { State } from "./state.ts";

/** A call worth keeping: someone is in it, or a stage stream is live there. */
function occupied(presence: HuddlePresence): boolean {
  return presence.participants.length > 0 || presence.live;
}

/** One room's call changed (the `huddle.presence` event, or a room's own fetch). */
export function setHuddlePresence(state: State, presence: HuddlePresence): State {
  const held = state.huddles[presence.roomId];

  if (held === undefined && !occupied(presence)) {
    return state;
  }

  const huddles = { ...state.huddles };

  if (occupied(presence)) {
    huddles[presence.roomId] = presence;
  } else {
    delete huddles[presence.roomId];
  }

  return { ...state, huddles };
}

/**
 * The aggregate poll: every call in the viewer's rooms. Rooms it doesn't list have nobody in
 * their call any more (a grant that expired quietly has no event of its own).
 */
export function loadHuddlePresence(state: State, list: HuddlePresenceList): State {
  const huddles: Record<number, HuddlePresence> = {};

  for (const presence of list.rooms) {
    if (occupied(presence)) {
      huddles[presence.roomId] = presence;
    }
  }

  return { ...state, huddles, users: mergeUserList(state.users, list.users) };
}

/** A stage's roster and stream (the `stage.updated` event, or a write's reply). */
export function setStage(state: State, stage: StageState): State {
  return { ...state, stages: { ...state.stages, [stage.roomId]: stage } };
}

/** `GET /rooms/:id/stage`: the stage plus every member's profile. */
export function loadStageDetail(state: State, detail: StageDetail): State {
  return setStage({ ...state, users: mergeUserList(state.users, detail.users) }, detail.stage);
}
