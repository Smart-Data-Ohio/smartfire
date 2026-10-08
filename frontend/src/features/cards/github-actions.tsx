import { type FormEvent, useEffect, useId, useState } from "react";
import type { GithubPullRequestActions } from "../../gen/GithubPullRequestActions.ts";
import type { GithubReviewKind } from "../../gen/GithubReviewKind.ts";
import { actions, type GithubCardScope } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Menu, MenuItem } from "../../ui/menu.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { patchGithubDraft, readGithubDraft, useGithubDraft } from "./github-drafts.ts";

interface ScopeProps {
  readonly roomId: number;
  readonly pullRequestId: number;
  readonly scope: GithubCardScope;
}

const REVIEWS = [
  {
    event: "approve",
    label: "Approve",
    title: "Approve",
    submit: "Approve",
    empty: null,
  },
  {
    event: "request_changes",
    label: "Request changes",
    title: "Request changes",
    submit: "Request changes",
    empty: "Add a note describing the requested changes.",
  },
  {
    event: "comment",
    label: "Comment",
    title: "Comment",
    submit: "Comment",
    empty: "Add a note for the review comment.",
  },
] as const satisfies readonly {
  readonly event: GithubReviewKind;
  readonly label: string;
  readonly title: string;
  readonly submit: string;
  readonly empty: string | null;
}[];

type ReviewChoice = (typeof REVIEWS)[number];

const REVIEWERS_EMPTY = "Enter GitHub usernames separated by commas.";

/** A short note, the comment box and the review dialog's body. */
function NoteField({
  label,
  value,
  error,
  onChange,
}: {
  readonly label: string;
  readonly value: string;
  readonly error: string | undefined;
  readonly onChange: (value: string) => void;
}) {
  const id = useId();
  const invalid = error !== undefined && error !== "";

  return (
    <div className={`field${invalid ? " is-error" : ""}`}>
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      <textarea
        id={id}
        className={`input github-note${invalid ? " is-error" : ""}`}
        rows={3}
        value={value}
        data-autofocus
        aria-invalid={invalid || undefined}
        aria-describedby={invalid ? `${id}-error` : undefined}
        onChange={(event) => onChange(event.target.value)}
      />
      {invalid ? (
        <p id={`${id}-error`} className="field-error" role="alert">
          {error}
        </p>
      ) : null}
    </div>
  );
}

function posted(notice: string): void {
  toast({ title: notice, tone: "success" });
}

function CommentForm({ roomId, pullRequestId, scope, onFailure }: ScopeProps & FailureProps) {
  const draft = useGithubDraft(roomId, pullRequestId);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const current = readGithubDraft(roomId, pullRequestId);

    if (current.commentPending) {
      return;
    }

    if (current.comment.trim() === "") {
      patchGithubDraft(roomId, pullRequestId, {
        commentError: "Write a comment first.",
        commentNotice: undefined,
      });

      return;
    }

    patchGithubDraft(roomId, pullRequestId, { commentPending: true, commentError: undefined });

    actions.cards.commentOnGithub(roomId, pullRequestId, scope, current.comment).then(
      (result) => {
        patchGithubDraft(roomId, pullRequestId, {
          commentPending: false,
          comment: "",
          commentNotice: result.notice,
        });
        posted(result.notice);
      },
      (failure: Error) => {
        patchGithubDraft(roomId, pullRequestId, {
          commentPending: false,
          commentNotice: undefined,
          commentError: failure.message,
        });
        onFailure(failure.message);
      },
    );
  };

  return (
    <form className="github-comment" onSubmit={submit} noValidate>
      <NoteField
        label="Comment"
        value={draft.comment}
        error={draft.commentError}
        onChange={(value) => {
          patchGithubDraft(roomId, pullRequestId, { comment: value, commentError: undefined });
        }}
      />
      <Button
        type="submit"
        variant="secondary"
        size="sm"
        loading={draft.commentPending}
        loadingLabel="Posting"
      >
        Comment
      </Button>
      {draft.commentNotice === undefined ? null : (
        <p className="github-notice" role="status">
          {draft.commentNotice}
        </p>
      )}
    </form>
  );
}

function ReviewDialog({
  roomId,
  pullRequestId,
  scope,
  choice,
  onClose,
  onFailure,
}: ScopeProps & {
  readonly choice: ReviewChoice;
  readonly onClose: () => void;
  readonly onFailure: (message: string) => void;
}) {
  const draft = useGithubDraft(roomId, pullRequestId);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const current = readGithubDraft(roomId, pullRequestId);

    if (current.reviewPending) {
      return;
    }

    if (choice.empty !== null && current.reviewNote.trim() === "") {
      patchGithubDraft(roomId, pullRequestId, { reviewError: choice.empty });

      return;
    }

    patchGithubDraft(roomId, pullRequestId, { reviewPending: true, reviewError: undefined });

    actions.cards.reviewGithub(roomId, pullRequestId, scope, choice.event, current.reviewNote).then(
      (result) => {
        patchGithubDraft(roomId, pullRequestId, {
          reviewPending: false,
          review: null,
          reviewNote: "",
          reviewError: undefined,
        });
        posted(result.notice);
      },
      (failure: Error) => {
        patchGithubDraft(roomId, pullRequestId, {
          reviewPending: false,
          reviewError: failure.message,
        });
        onFailure(failure.message);
      },
    );
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => (open ? undefined : onClose())}
      title={choice.title}
      size="sm"
    >
      <form className="github-comment" onSubmit={submit} noValidate>
        <NoteField
          label="Note"
          value={draft.reviewNote}
          error={draft.reviewError}
          onChange={(value) => {
            patchGithubDraft(roomId, pullRequestId, { reviewNote: value, reviewError: undefined });
          }}
        />
        <div className="github-dialog-actions">
          <Button variant="secondary" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="primary"
            size="sm"
            loading={draft.reviewPending}
            loadingLabel="Posting"
          >
            {choice.submit}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}

function ReviewMenu({
  instanceId,
  ...props
}: ScopeProps & FailureProps & { readonly instanceId: string }) {
  const { roomId, pullRequestId } = props;
  const draft = useGithubDraft(roomId, pullRequestId);
  const choice = REVIEWS.find((review) => review.event === draft.review) ?? null;
  const owned = choice !== null && draft.reviewOwner === instanceId;

  return (
    <>
      <Menu
        label="Review this pull request"
        trigger={(trigger) => (
          <Button {...trigger} variant="secondary" size="sm" trailingIcon="chevron-down">
            Review
          </Button>
        )}
      >
        {REVIEWS.map((review) => (
          <MenuItem
            key={review.event}
            onSelect={() =>
              patchGithubDraft(roomId, pullRequestId, {
                review: review.event,
                reviewOwner: instanceId,
              })
            }
          >
            {review.label}
          </MenuItem>
        ))}
      </Menu>
      {owned && choice !== null ? (
        <ReviewDialog
          {...props}
          choice={choice}
          onClose={() =>
            patchGithubDraft(roomId, pullRequestId, { review: null, reviewOwner: null })
          }
        />
      ) : null}
    </>
  );
}

function ReviewersDialog({
  roomId,
  pullRequestId,
  scope,
  onClose,
  onFailure,
}: ScopeProps & { readonly onClose: () => void; readonly onFailure: (message: string) => void }) {
  const draft = useGithubDraft(roomId, pullRequestId);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const current = readGithubDraft(roomId, pullRequestId);

    if (current.reviewersPending) {
      return;
    }

    if (current.reviewers.trim() === "") {
      patchGithubDraft(roomId, pullRequestId, {
        reviewersError: REVIEWERS_EMPTY,
        reviewersAttempt: current.reviewersAttempt + 1,
      });

      return;
    }

    patchGithubDraft(roomId, pullRequestId, {
      reviewersPending: true,
      reviewersError: undefined,
    });

    actions.cards.requestGithubReviewers(roomId, pullRequestId, scope, current.reviewers).then(
      (result) => {
        patchGithubDraft(roomId, pullRequestId, {
          reviewersPending: false,
          reviewersOpen: false,
          reviewers: "",
          reviewersError: undefined,
        });
        posted(result.notice);
      },
      (failure: Error) => {
        const latest = readGithubDraft(roomId, pullRequestId);

        patchGithubDraft(roomId, pullRequestId, {
          reviewersPending: false,
          reviewersError: failure.message,
          reviewersAttempt: latest.reviewersAttempt + 1,
        });
        onFailure(failure.message);
      },
    );
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => (open ? undefined : onClose())}
      title="Request reviewers"
      size="sm"
    >
      <form className="github-comment" onSubmit={submit} noValidate>
        <TextField
          label="Reviewers"
          hint="GitHub usernames, separated by commas."
          value={draft.reviewers}
          error={draft.reviewersError}
          attempt={draft.reviewersAttempt}
          data-autofocus
          onChange={(event) => {
            patchGithubDraft(roomId, pullRequestId, {
              reviewers: event.target.value,
              reviewersError: undefined,
            });
          }}
        />
        <div className="github-dialog-actions">
          <Button variant="secondary" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="primary"
            size="sm"
            loading={draft.reviewersPending}
            loadingLabel="Requesting"
          >
            Request reviewers
          </Button>
        </div>
      </form>
    </Dialog>
  );
}

function ReviewersButton({
  instanceId,
  ...props
}: ScopeProps & FailureProps & { readonly instanceId: string }) {
  const { roomId, pullRequestId } = props;
  const draft = useGithubDraft(roomId, pullRequestId);
  const owned = draft.reviewersOpen && draft.reviewersOwner === instanceId;

  return (
    <>
      <Button
        variant="secondary"
        size="sm"
        onClick={() =>
          patchGithubDraft(roomId, pullRequestId, {
            reviewersOpen: true,
            reviewersOwner: instanceId,
          })
        }
      >
        Request reviewers
      </Button>
      {owned ? (
        <ReviewersDialog
          {...props}
          onClose={() =>
            patchGithubDraft(roomId, pullRequestId, {
              reviewersOpen: false,
              reviewersOwner: null,
            })
          }
        />
      ) : null}
    </>
  );
}

interface FailureProps {
  /** A write was refused. The caller asks again what the viewer can do (a rejected token hides it). */
  readonly onFailure: (message: string) => void;
}

/**
 * Comment, review and request reviewers on a pull request, shown only for the flags
 * `GET .../actions` returns. A success refetches the card; GitHub's webhook also publishes
 * `message.cards`, which refetches the preview without unmounting a draft or an open dialog.
 * A write stays pending if its dialog is closed, so opening it again does not post twice.
 * The dialog belongs to this mounted copy: the timeline and the discussion each have their own,
 * and only the one that opened it renders the modal.
 */
export function GithubActions({
  roomId,
  pullRequestId,
  scope,
  previewGeneration,
}: ScopeProps & { readonly previewGeneration: number }) {
  const instanceId = useId();
  const [capabilities, setCapabilities] = useState<GithubPullRequestActions | null>(null);
  const [generation, setGeneration] = useState(0);

  // The copy that opened a dialog unmounts: close it here, and leave the typed draft.
  useEffect(() => {
    return () => {
      const current = readGithubDraft(roomId, pullRequestId);

      if (current.reviewOwner === instanceId) {
        patchGithubDraft(roomId, pullRequestId, { review: null, reviewOwner: null });
      }

      if (current.reviewersOwner === instanceId) {
        patchGithubDraft(roomId, pullRequestId, {
          reviewersOpen: false,
          reviewersOwner: null,
        });
      }
    };
  }, [instanceId, roomId, pullRequestId]);

  // generation retries after a refused write. previewGeneration moves when the preview is
  // invalidated or fetched again, so a discussion created under a mounted card shows its controls.
  // biome-ignore lint/correctness/useExhaustiveDependencies: both counts are retry triggers, not inputs
  useEffect(() => {
    let live = true;

    actions.cards.githubActions(roomId, pullRequestId).then(
      (next) => {
        if (live) {
          setCapabilities(next);
        }
      },
      () => {
        if (live) {
          setCapabilities(null);
        }
      },
    );

    return () => {
      live = false;
    };
  }, [roomId, pullRequestId, generation, previewGeneration]);

  if (
    capabilities === null ||
    (!capabilities.canComment && !capabilities.canReview && !capabilities.canRequestReviewers)
  ) {
    return null;
  }

  const onFailure = (message: string) => {
    toast({ title: message, tone: "danger" });
    setGeneration((count) => count + 1);
  };

  const shared = { roomId, pullRequestId, scope, onFailure };

  return (
    <div className="github-actions">
      {capabilities.canComment ? <CommentForm {...shared} /> : null}
      {capabilities.canReview || capabilities.canRequestReviewers ? (
        <div className="github-action-row">
          {capabilities.canReview ? <ReviewMenu {...shared} instanceId={instanceId} /> : null}
          {capabilities.canRequestReviewers ? (
            <ReviewersButton {...shared} instanceId={instanceId} />
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
