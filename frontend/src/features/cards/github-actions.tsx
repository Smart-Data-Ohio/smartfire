import { type FormEvent, useEffect, useId, useState } from "react";
import type { GithubPullRequestActions } from "../../gen/GithubPullRequestActions.ts";
import type { GithubReviewKind } from "../../gen/GithubReviewKind.ts";
import { actions, type GithubCardScope } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Menu, MenuItem } from "../../ui/menu.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";

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
  const [body, setBody] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const [notice, setNotice] = useState<string | undefined>(undefined);
  const [busy, setBusy] = useState(false);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (body.trim() === "") {
      setError("Write a comment first.");
      setNotice(undefined);

      return;
    }

    setBusy(true);
    setError(undefined);

    actions.cards.commentOnGithub(roomId, pullRequestId, scope, body).then(
      (result) => {
        setBusy(false);
        setBody("");
        setNotice(result.notice);
        posted(result.notice);
      },
      (failure: Error) => {
        setBusy(false);
        setNotice(undefined);
        setError(failure.message);
        onFailure(failure.message);
      },
    );
  };

  return (
    <form className="github-comment" onSubmit={submit} noValidate>
      <NoteField
        label="Comment"
        value={body}
        error={error}
        onChange={(value) => {
          setBody(value);
          setError(undefined);
        }}
      />
      <Button type="submit" variant="secondary" size="sm" loading={busy} loadingLabel="Posting">
        Comment
      </Button>
      {notice === undefined ? null : (
        <p className="github-notice" role="status">
          {notice}
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
  const [body, setBody] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const [busy, setBusy] = useState(false);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (choice.empty !== null && body.trim() === "") {
      setError(choice.empty);

      return;
    }

    setBusy(true);
    setError(undefined);

    actions.cards.reviewGithub(roomId, pullRequestId, scope, choice.event, body).then(
      (result) => {
        posted(result.notice);
        onClose();
      },
      (failure: Error) => {
        setBusy(false);
        setError(failure.message);
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
          value={body}
          error={error}
          onChange={(value) => {
            setBody(value);
            setError(undefined);
          }}
        />
        <div className="github-dialog-actions">
          <Button variant="secondary" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" variant="primary" size="sm" loading={busy} loadingLabel="Posting">
            {choice.submit}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}

function ReviewMenu(props: ScopeProps & FailureProps) {
  const [choice, setChoice] = useState<ReviewChoice | null>(null);

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
          <MenuItem key={review.event} onSelect={() => setChoice(review)}>
            {review.label}
          </MenuItem>
        ))}
      </Menu>
      {choice === null ? null : (
        <ReviewDialog {...props} choice={choice} onClose={() => setChoice(null)} />
      )}
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
  const [reviewers, setReviewers] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState(0);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (reviewers.trim() === "") {
      setError(REVIEWERS_EMPTY);
      setAttempt((count) => count + 1);

      return;
    }

    setBusy(true);
    setError(undefined);

    actions.cards.requestGithubReviewers(roomId, pullRequestId, scope, reviewers).then(
      (result) => {
        posted(result.notice);
        onClose();
      },
      (failure: Error) => {
        setBusy(false);
        setError(failure.message);
        setAttempt((count) => count + 1);
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
          value={reviewers}
          error={error}
          attempt={attempt}
          data-autofocus
          onChange={(event) => {
            setReviewers(event.target.value);
            setError(undefined);
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
            loading={busy}
            loadingLabel="Requesting"
          >
            Request reviewers
          </Button>
        </div>
      </form>
    </Dialog>
  );
}

function ReviewersButton(props: ScopeProps & FailureProps) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button variant="secondary" size="sm" onClick={() => setOpen(true)}>
        Request reviewers
      </Button>
      {open ? <ReviewersDialog {...props} onClose={() => setOpen(false)} /> : null}
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
 * `message.cards`, which drops the preview so a mounted card fetches it again.
 */
export function GithubActions({ roomId, pullRequestId, scope }: ScopeProps) {
  const [capabilities, setCapabilities] = useState<GithubPullRequestActions | null>(null);
  const [generation, setGeneration] = useState(0);

  // generation is the retry after a refused write, so a rejected token hides the controls.
  // biome-ignore lint/correctness/useExhaustiveDependencies: generation is the retry trigger, not an input
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
  }, [roomId, pullRequestId, generation]);

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
          {capabilities.canReview ? <ReviewMenu {...shared} /> : null}
          {capabilities.canRequestReviewers ? <ReviewersButton {...shared} /> : null}
        </div>
      ) : null}
    </div>
  );
}
