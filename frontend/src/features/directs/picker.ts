/**
 * The people pickers' logic (new DM, add people): who matches the query, chip selection, and
 * what Enter will do. Pure, so it is tested apart from the dialogs.
 */
import type { DirectCandidate } from "../../gen/DirectCandidate.ts";
import type { User } from "../../store/model.ts";
import { matchScore, normalizeQuery } from "../switcher/match.ts";

/** A direct room holds at most 10 people: the viewer and 9 others. */
export const MAX_OTHERS = 9;

const MAX_RESULTS = 50;

/** A candidate with the profile the list shows. */
export interface PickerOption {
  readonly userId: number;
  readonly name: string;
  readonly agent: boolean;
  readonly starred: boolean;
  /** The viewer: picked alone, it opens their note-to-self. */
  readonly you: boolean;
}

/**
 * The candidates to list for `query`: not already chosen, not `excluded` (a DM's members), with
 * a known, active profile. No query keeps the server's order (starred first, then by name);
 * a query ranks by match, starred breaking ties.
 */
export function pickerOptions(
  candidates: readonly DirectCandidate[],
  users: Readonly<Record<number, User>>,
  query: string,
  hidden: ReadonlySet<number>,
  viewerId: number | null = null,
): readonly PickerOption[] {
  const needle = normalizeQuery(query);

  const scored = candidates.flatMap((candidate) => {
    const user = users[candidate.userId];

    if (user === undefined || user.status !== "active" || hidden.has(candidate.userId)) {
      return [];
    }

    const score = matchScore(user.name, needle);

    return score === null
      ? []
      : [
          {
            option: {
              userId: candidate.userId,
              name: user.name,
              agent: candidate.agent,
              starred: candidate.starred,
              you: candidate.userId === viewerId,
            },
            score: score + (candidate.starred ? 5 : 0),
          },
        ];
  });

  const ordered = needle === "" ? scored : scored.sort((left, right) => right.score - left.score);

  return ordered.slice(0, MAX_RESULTS).map((entry) => entry.option);
}

/** `userId` added to the chips (unless full), or taken out if it was there. */
export function toggleSelected(
  selected: readonly number[],
  userId: number,
  limit: number = MAX_OTHERS,
): readonly number[] {
  if (selected.includes(userId)) {
    return selected.filter((id) => id !== userId);
  }

  return selected.length >= limit ? selected : [...selected, userId];
}

/** Backspace in an empty query takes the last chip off. */
export function removeLast(selected: readonly number[]): readonly number[] {
  return selected.slice(0, -1);
}

/**
 * What creating with these chips makes: nothing yet, your note-to-self (only you), a one-to-one
 * DM, or a group DM. You beside others add nothing: the room is theirs and yours either way.
 */
export type DirectIntent = "none" | "self" | "direct" | "group";

export function directIntent(
  selected: readonly number[],
  viewerId: number | null = null,
): DirectIntent {
  const others = selected.filter((id) => id !== viewerId);

  if (selected.length === 0) {
    return "none";
  }

  if (others.length === 0) {
    return "self";
  }

  return others.length === 1 ? "direct" : "group";
}

/** The people to open a conversation with: the chips without you (the server adds you). */
export function otherIds(selected: readonly number[], viewerId: number | null): readonly number[] {
  return selected.filter((id) => id !== viewerId);
}

/**
 * Adding people to a DM: a group-capable one (more than one other member, or a name) grows in
 * place; a one-to-one can't, so it starts a new group DM with everyone instead (the contract's
 * `DirectRoom#group_capable?`).
 */
export type AddPlan =
  | { readonly kind: "add"; readonly userIds: readonly number[] }
  | { readonly kind: "new-group"; readonly userIds: readonly number[] };

export function addPlan(
  memberIds: readonly number[],
  named: boolean,
  picked: readonly number[],
): AddPlan {
  return memberIds.length > 1 || named
    ? { kind: "add", userIds: picked }
    : { kind: "new-group", userIds: [...memberIds, ...picked] };
}
