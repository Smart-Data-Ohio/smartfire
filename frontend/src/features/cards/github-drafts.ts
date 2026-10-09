/**
 * Comment text, review and reviewer dialogs, and a write still on its way, keyed by room and
 * pull request. The card's preview can refresh underneath (`message.cards`); this outlives that
 * remount, and a dialog closed while the write is pending, so a second submit does not post again.
 * The open dialog belongs to the mounted copy that opened it (`reviewOwner`, `reviewersOwner`).
 * An entry that is empty and not pending is dropped, so cleared drafts do not accumulate.
 */
import { useSyncExternalStore } from "react";
import type { GithubReviewKind } from "../../gen/GithubReviewKind.ts";

export interface GithubDraft {
  readonly comment: string;
  readonly commentError: string | undefined;
  readonly commentNotice: string | undefined;
  readonly commentPending: boolean;
  readonly review: GithubReviewKind | null;
  /** The `useId` of the card copy that opened the review dialog; null while it is closed. */
  readonly reviewOwner: string | null;
  readonly reviewNote: string;
  readonly reviewError: string | undefined;
  readonly reviewPending: boolean;
  readonly reviewersOpen: boolean;
  /** The `useId` of the card copy that opened the reviewers dialog. */
  readonly reviewersOwner: string | null;
  readonly reviewers: string;
  readonly reviewersError: string | undefined;
  readonly reviewersAttempt: number;
  readonly reviewersPending: boolean;
}

const EMPTY: GithubDraft = {
  comment: "",
  commentError: undefined,
  commentNotice: undefined,
  commentPending: false,
  review: null,
  reviewOwner: null,
  reviewNote: "",
  reviewError: undefined,
  reviewPending: false,
  reviewersOpen: false,
  reviewersOwner: null,
  reviewers: "",
  reviewersError: undefined,
  reviewersAttempt: 0,
  reviewersPending: false,
};

const drafts = new Map<string, GithubDraft>();

const listeners = new Set<() => void>();

export function githubDraftKey(roomId: number, pullRequestId: number): string {
  return `${roomId}:${pullRequestId}`;
}

function readByKey(key: string): GithubDraft {
  return drafts.get(key) ?? EMPTY;
}

function emit(): void {
  for (const listener of listeners) {
    listener();
  }
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);

  return () => {
    listeners.delete(listener);
  };
}

export function readGithubDraft(roomId: number, pullRequestId: number): GithubDraft {
  return readByKey(githubDraftKey(roomId, pullRequestId));
}

/** Nothing to show and nothing in flight: the map should not keep the key. */
function vacant(draft: GithubDraft): boolean {
  return (
    draft.comment === "" &&
    draft.commentError === undefined &&
    draft.commentNotice === undefined &&
    !draft.commentPending &&
    draft.review === null &&
    draft.reviewOwner === null &&
    draft.reviewNote === "" &&
    draft.reviewError === undefined &&
    !draft.reviewPending &&
    !draft.reviewersOpen &&
    draft.reviewersOwner === null &&
    draft.reviewers === "" &&
    draft.reviewersError === undefined &&
    !draft.reviewersPending
  );
}

export function patchGithubDraft(
  roomId: number,
  pullRequestId: number,
  change: Partial<GithubDraft>,
): void {
  const key = githubDraftKey(roomId, pullRequestId);
  const next = { ...readByKey(key), ...change };

  if (vacant(next)) {
    if (!drafts.delete(key)) {
      return;
    }

    emit();

    return;
  }

  drafts.set(key, next);
  emit();
}

/** Tests start from an empty pad. */
export function resetGithubDrafts(): void {
  drafts.clear();
  emit();
}

/** How many drafts are held. An empty, idle one is not. */
export function githubDraftCount(): number {
  return drafts.size;
}

export function useGithubDraft(roomId: number, pullRequestId: number): GithubDraft {
  const key = githubDraftKey(roomId, pullRequestId);

  return useSyncExternalStore(
    subscribe,
    () => readByKey(key),
    () => EMPTY,
  );
}
