/**
 * Comment text, review and reviewer dialogs, and a write still on its way, keyed by room and
 * pull request. The card's preview can refresh underneath (`message.cards`); this outlives that
 * remount, and a dialog closed while the write is pending, so a second submit does not post again.
 */
import { useSyncExternalStore } from "react";
import type { GithubReviewKind } from "../../gen/GithubReviewKind.ts";

export interface GithubDraft {
  readonly comment: string;
  readonly commentError: string | undefined;
  readonly commentNotice: string | undefined;
  readonly commentPending: boolean;
  readonly review: GithubReviewKind | null;
  readonly reviewNote: string;
  readonly reviewError: string | undefined;
  readonly reviewPending: boolean;
  readonly reviewersOpen: boolean;
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
  reviewNote: "",
  reviewError: undefined,
  reviewPending: false,
  reviewersOpen: false,
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

export function patchGithubDraft(
  roomId: number,
  pullRequestId: number,
  change: Partial<GithubDraft>,
): void {
  const key = githubDraftKey(roomId, pullRequestId);

  drafts.set(key, { ...readByKey(key), ...change });
  emit();
}

/** Tests start from an empty pad; a draft is otherwise kept for the life of the page. */
export function resetGithubDrafts(): void {
  drafts.clear();
  emit();
}

export function useGithubDraft(roomId: number, pullRequestId: number): GithubDraft {
  const key = githubDraftKey(roomId, pullRequestId);

  return useSyncExternalStore(
    subscribe,
    () => readByKey(key),
    () => EMPTY,
  );
}
