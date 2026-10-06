/**
 * What React calls for direct messages and the quick switcher: plain promises over the Effect
 * programs in `direct-actions.ts`. Failures reject with an `ActionError` fit to show.
 */
import type { DirectCandidate } from "../gen/DirectCandidate.ts";
import type { Switcher } from "../gen/Switcher.ts";
import type { RoomDetail, SidebarRow } from "../store/model.ts";
import * as directActions from "./direct-actions.ts";
import { ActionError } from "./run.ts";
import { runAction } from "./runtime.ts";

export const directs = {
  candidates: (): Promise<readonly DirectCandidate[]> => runAction(directActions.candidates()),
  create: (userIds: readonly number[]): Promise<SidebarRow> =>
    runAction(directActions.create(userIds)),
  addMembers: (roomId: number, userIds: readonly number[]): Promise<RoomDetail> =>
    runAction(directActions.addMembers(roomId, userIds)),
  rename: (roomId: number, name: string | null): Promise<RoomDetail> =>
    runAction(directActions.rename(roomId, name)),
  switcher: (): Promise<Switcher> => runAction(directActions.switcherData()),
};

/**
 * The server's reason when it turned the request down as invalid (a 422: nobody new chosen, a
 * group already full), so the dialog can say it in place; `null` for anything else.
 */
export function validationMessage(error: Error): string | null {
  return error instanceof ActionError && error.tag === "Validation" ? error.message : null;
}
