/**
 * Who may open the links editor. Classic `work_threads/links` lets any active member of the
 * room add and remove links, with no management right required. A thread that isn't tracked
 * has nothing to link, and that is said outright, in the same words as a handoff refusal,
 * never as a blank route.
 */

import { UNTRACKED_HANDOFF } from "./handoff-access.ts";

/** Why the links editor can't open, or null when it can. */
export function linksRefusal(input: { readonly tracked: boolean }): string | null {
  return input.tracked ? null : UNTRACKED_HANDOFF;
}
