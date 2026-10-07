import { Link } from "@tanstack/react-router";
import { type ReactNode, useCallback } from "react";
import type { GithubCardRef } from "../../gen/GithubCardRef.ts";
import type { GithubChangedFiles } from "../../gen/GithubChangedFiles.ts";
import type { GithubChecks } from "../../gen/GithubChecks.ts";
import type { GithubPullRequest } from "../../gen/GithubPullRequest.ts";
import type { GithubPullRequestStatus } from "../../gen/GithubPullRequestStatus.ts";
import type { GithubReview } from "../../gen/GithubReview.ts";
import { githubKey } from "../../store/cards.ts";
import type { MessageDTO } from "../../store/model.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import type { IconName } from "../../ui/icons/icon.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { useNow } from "../threads/use-now.ts";
import { BrandMark } from "./brand-marks.tsx";
import { ago } from "./format.ts";
import { usePreview } from "./use-preview.ts";

interface Look {
  readonly icon: IconName;
  readonly label: string;
}

const STATUS = {
  open: { icon: "git-pull-request", label: "Open" },
  draft: { icon: "git-pull-request-draft", label: "Draft" },
  merged: { icon: "git-merge", label: "Merged" },
  closed: { icon: "git-pull-request-closed", label: "Closed" },
} satisfies { readonly [Status in GithubPullRequestStatus]: Look };

const REVIEW = {
  approved: { icon: "circle-check", label: "Approved" },
  changes_requested: { icon: "circle-alert", label: "Changes requested" },
  review_required: { icon: "eye", label: "Review required" },
} satisfies { readonly [Review in GithubReview]: Look };

const CHECKS = {
  passing: { icon: "circle-check", label: "Checks passing" },
  pending: { icon: "circle-dot", label: "Checks pending" },
  failing: { icon: "circle-x", label: "Checks failing" },
} satisfies { readonly [Checks in GithubChecks]: Look };

/** "owner/repo #123", the pull request's address in a line. */
function Address({
  owner,
  repo,
  number,
}: {
  readonly owner: string;
  readonly repo: string;
  readonly number: number;
}) {
  return (
    <span className="github-address">
      <BrandMark name="github" size={14} />
      <span>
        {owner}/{repo}
      </span>
      <span className="tabular">#{number}</span>
    </span>
  );
}

/** The card's frame while the pull request is on its way (also GitHub's own "loading" state). */
function GithubSkeleton({ card }: { readonly card: GithubCardRef }) {
  return (
    <section
      className="card github-card"
      aria-busy="true"
      aria-label={`Pull request ${card.owner}/${card.repo} #${card.number}`}
    >
      <div className="github-head">
        <Address owner={card.owner} repo={card.repo} number={card.number} />
      </div>
      <Skeleton width="80%" height={16} />
      <Skeleton width="55%" height={12} />
      <div className="github-badges">
        <Skeleton width={72} height={20} radius="pill" />
        <Skeleton width={110} height={20} radius="pill" />
        <Skeleton width={96} height={20} radius="pill" />
      </div>
    </section>
  );
}

/** The changed files in the thread header: name, status, and the +/- counts. */
function ChangedFiles({ files }: { readonly files: GithubChangedFiles }) {
  const more = files.totalCount - files.files.length;

  return (
    <div className="github-files">
      <div className="github-files-head">
        {files.totalCount} {files.totalCount === 1 ? "file" : "files"} changed
      </div>
      <ul className="github-file-list">
        {files.files.map((file) => (
          <li key={file.filename} className="github-file" data-status={file.status ?? undefined}>
            <span className="github-file-name" title={file.filename}>
              {file.filename}
            </span>
            <span className="github-file-diff tabular">
              <span className="github-additions">+{file.additions}</span>
              <span className="github-deletions">−{file.deletions}</span>
            </span>
          </li>
        ))}
      </ul>
      {more > 0 ? <div className="card-subtle">and {more} more</div> : null}
    </div>
  );
}

interface LoadedProps {
  readonly message: MessageDTO;
  readonly pull: GithubPullRequest;
  /** It heads the pull request's discussion thread: files instead of "Discuss". */
  readonly header: boolean;
}

function Loaded({ message, pull, header }: LoadedProps) {
  const now = useNow();
  const status = STATUS[pull.status];

  return (
    <section
      className="card github-card"
      data-status={pull.status}
      aria-label={`Pull request: ${pull.title}`}
    >
      <div className="github-head">
        <Address owner={pull.owner} repo={pull.repo} number={pull.number} />
        <span className="card-tag github-status" data-status={pull.status}>
          <Icon name={status.icon} size={12} />
          {status.label}
        </span>
      </div>
      <a className="github-title" href={pull.url} target="_blank" rel="noopener noreferrer">
        {pull.title}
      </a>
      <div className="github-meta card-subtle">
        {pull.authorLogin === null ? null : (
          <span className="github-author">
            {pull.authorAvatarUrl === null ? null : (
              <img
                className="github-avatar"
                src={pull.authorAvatarUrl}
                alt=""
                width={16}
                height={16}
              />
            )}
            {pull.authorLogin}
          </span>
        )}
        {pull.baseBranch === null || pull.headBranch === null ? null : (
          <span className="github-branches">
            <code>{pull.baseBranch}</code>
            <span aria-hidden="true">←</span>
            <span className="visually-hidden">from</span>
            <code>{pull.headBranch}</code>
          </span>
        )}
        {pull.githubUpdatedAt === null ? null : (
          <span>Updated {ago(pull.githubUpdatedAt, now)}</span>
        )}
      </div>
      <div className="github-badges">
        {pull.review === null ? null : (
          <span className="card-tag" data-review={pull.review}>
            <Icon name={REVIEW[pull.review].icon} size={12} />
            {REVIEW[pull.review].label}
          </span>
        )}
        {pull.checks === null ? (
          <span className="card-tag" data-tone="neutral">
            No checks
          </span>
        ) : (
          <span className="card-tag" data-checks={pull.checks}>
            <Icon name={CHECKS[pull.checks].icon} size={12} />
            {CHECKS[pull.checks].label}
          </span>
        )}
        {header ? null : pull.discussionThreadId === null ? (
          <Link
            to="/r/$roomId/t/new"
            params={{ roomId: message.roomId }}
            search={{ parent: message.id }}
            className="card-link github-discuss"
            preload={false}
          >
            <Icon name="message-circle" size={14} />
            Discuss
          </Link>
        ) : (
          <Link
            to="/r/$roomId/t/$threadId"
            params={{ roomId: message.roomId, threadId: pull.discussionThreadId }}
            className="card-link github-discuss"
            preload={false}
          >
            <Icon name="message-circle" size={14} />
            Discussion
          </Link>
        )}
      </div>
      {header && pull.files !== null ? <ChangedFiles files={pull.files} /> : null}
    </section>
  );
}

/** "Couldn't load": the reason, a way to try again, and the link itself. */
function Failed({
  card,
  reason,
  onRetry,
}: {
  readonly card: GithubCardRef;
  readonly reason: string;
  readonly onRetry: () => void;
}) {
  return (
    <section
      className="card github-card"
      data-state="failed"
      aria-label={`Pull request ${card.owner}/${card.repo} #${card.number}`}
    >
      <div className="github-head">
        <Address owner={card.owner} repo={card.repo} number={card.number} />
      </div>
      <div className="card-error">
        <Icon name="circle-alert" size={14} />
        <span>Couldn't load this pull request. {reason}</span>
      </div>
      <div className="github-badges">
        <Button variant="secondary" size="sm" icon="refresh-cw" onClick={onRetry}>
          Retry
        </Button>
        <a className="card-link" href={card.url} target="_blank" rel="noopener noreferrer">
          Open on GitHub
          <Icon name="arrow-up-right" size={12} />
        </a>
      </div>
    </section>
  );
}

interface PreviewProps {
  readonly message: MessageDTO;
  readonly card: GithubCardRef;
  /** Fetch the thread header's variant (with files) for this thread, instead of the message's. */
  readonly threadId: number | null;
  /** Shown instead when the thread variant isn't there (the thread isn't this one's discussion). */
  readonly fallback: ReactNode;
}

/** The pull request fetched in one scope: under the message, or as a thread's header. */
function GithubPreview({ message, card, threadId, fallback }: PreviewProps) {
  const load = useCallback(
    () =>
      actions.cards.loadGithub(
        message.roomId,
        card.pullRequestId,
        threadId === null ? { messageId: message.id } : { threadId },
      ),
    [message.roomId, card.pullRequestId, message.id, threadId],
  );

  const key = githubKey(
    message.roomId,
    card.pullRequestId,
    threadId === null ? { messageId: message.id } : { threadId },
  );

  const preview = usePreview("github", key, load);
  const value = preview?.value ?? null;
  const retry = () => load().catch(() => undefined);

  if (value === null) {
    if (preview?.status !== "error") {
      return <GithubSkeleton card={card} />;
    }

    return threadId === null ? (
      <Failed card={card} reason={preview.error ?? ""} onRetry={retry} />
    ) : (
      fallback
    );
  }

  switch (value.state) {
    case "hidden":
      return null;
    case "loading":
      return <GithubSkeleton card={card} />;
    case "failed":
      return <Failed card={card} reason={value.message} onRetry={retry} />;
    case "loaded":
      return <Loaded message={message} pull={value} header={threadId !== null} />;
  }
}

/**
 * A GitHub pull request the message links to, as the viewer may see it: status, review and
 * checks, branches and "Discuss". Fetched per viewer; a private repository the viewer can't
 * read shows nothing. On the root of a thread (`threadId`) it first asks for that thread's
 * header variant: when the thread is the pull request's discussion, it lists the changed files.
 */
export function GithubCard({
  message,
  card,
  threadId,
}: {
  readonly message: MessageDTO;
  readonly card: GithubCardRef;
  readonly threadId: number | null;
}) {
  const below = <GithubPreview message={message} card={card} threadId={null} fallback={null} />;

  return threadId === null ? (
    below
  ) : (
    <GithubPreview message={message} card={card} threadId={threadId} fallback={below} />
  );
}
