import { Schema, SchemaGetter } from "effect";
import type { WorkDetail as GeneratedWorkDetail } from "../../gen/WorkDetail.ts";
import type { WorkFacts as GeneratedWorkFacts } from "../../gen/WorkFacts.ts";
import type { WorkHandoffReceiver as GeneratedWorkHandoffReceiver } from "../../gen/WorkHandoffReceiver.ts";
import type { WorkHistoryEntry as GeneratedWorkHistoryEntry } from "../../gen/WorkHistoryEntry.ts";
import type { WorkHistoryHandoff as GeneratedWorkHistoryHandoff } from "../../gen/WorkHistoryHandoff.ts";
import type { WorkHistoryKind as GeneratedWorkHistoryKind } from "../../gen/WorkHistoryKind.ts";
import type { WorkLink as GeneratedWorkLink } from "../../gen/WorkLink.ts";
import type { WorkLinkKind as GeneratedWorkLinkKind } from "../../gen/WorkLinkKind.ts";
import type { WorkOwnerCandidate as GeneratedWorkOwnerCandidate } from "../../gen/WorkOwnerCandidate.ts";
import type { WorkOwnerSnapshot as GeneratedWorkOwnerSnapshot } from "../../gen/WorkOwnerSnapshot.ts";
import type { WorkPullRequestState as GeneratedWorkPullRequestState } from "../../gen/WorkPullRequestState.ts";
import type { WorkStatus as GeneratedWorkStatus } from "../../gen/WorkStatus.ts";
import { AgentStep } from "./agents.ts";
import { AgentId, UserId, WorkEventId, WorkLinkId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { RowTimestamp, Timestamp } from "./time.ts";
import { tolerantLiterals } from "./tolerant.ts";
import { User } from "./user.ts";

/**
 * The work pieces a thread carries, kept apart from `work.ts` (whose list needs threads) so
 * `thread.ts` and the work module don't import each other.
 */

/** `channel_threads.work_status`, strict: what `UpdateWork.status` sends. */
export const WorkStatus = Schema.Literals(["planned", "in_progress", "blocked", "done"]);

export type WorkStatus = typeof WorkStatus.Type;

export type WorkStatusPin = Assert<Pinned<typeof WorkStatus, GeneratedWorkStatus>>;

/**
 * A work status as read: on every thread list and thread event, so a status added after this
 * build decodes to `"unknown"` instead of failing the whole list or sync batch.
 */
export const WorkStatusRead = tolerantLiterals(["planned", "in_progress", "blocked", "done"]);

export type WorkStatusRead = typeof WorkStatusRead.Type;

export type WorkStatusReadPin = Assert<Pinned<typeof WorkStatusRead, GeneratedWorkStatus>>;

/**
 * Whether `url` is safe in an `href`: an `https://` URL or a site-relative path (one leading
 * slash, not `//` or `/\`, which browsers read as another host). The server sends only these
 * for work URLs; this checks again so nothing else (a `javascript:` URL) reaches the DOM.
 */
export function isSafeWorkHref(url: string): boolean {
  return /^https:\/\//i.test(url) || /^\/(?![/\\])/.test(url);
}

/** `runUrl` as read: `null` unless it's an `https://` URL, as the classic page shows it. */
const RunUrl = Schema.NullOr(Schema.String).pipe(
  Schema.decodeTo(Schema.NullOr(Schema.String), {
    decode: SchemaGetter.transform((url) =>
      url !== null && /^https:\/\//i.test(url) ? url : null,
    ),
    encode: SchemaGetter.transform((url) => url),
  }),
);

/** Tolerant: a kind added after this build decodes to `"unknown"` (show the label and URL). */
export const WorkLinkKind = tolerantLiterals(["pull_request", "event", "drive_file"]);

export type WorkLinkKind = typeof WorkLinkKind.Type;

export type WorkLinkKindPin = Assert<Pinned<typeof WorkLinkKind, GeneratedWorkLinkKind>>;

/** Tolerant, like `WorkLinkKind`. */
export const WorkPullRequestState = tolerantLiterals(["open", "draft", "merged", "closed"]);

export type WorkPullRequestState = typeof WorkPullRequestState.Type;

export type WorkPullRequestStatePin = Assert<
  Pinned<typeof WorkPullRequestState, GeneratedWorkPullRequestState>
>;

/**
 * A linked pull request, calendar event or Drive file, the same for every viewer. `title` is a
 * public repository's pull request title only; an event's `url` is its classic page.
 */
export const WorkLink = Schema.Struct({
  id: WorkLinkId,
  kind: WorkLinkKind,
  label: Schema.String,
  url: Schema.String,
  pullRequestState: Schema.NullOr(WorkPullRequestState),
  title: Schema.NullOr(Schema.String),
  eventStartsAt: Schema.NullOr(Timestamp),
  eventTimeZone: Schema.NullOr(Schema.String),
  eventCancelled: Schema.Boolean,
});

export type WorkLink = typeof WorkLink.Type;

export type WorkLinkPin = Assert<Pinned<typeof WorkLink, GeneratedWorkLink>>;

/** The links as read, less any whose URL isn't safe in an `href` (`isSafeWorkHref`). */
const WorkLinks = Schema.Array(WorkLink).pipe(
  Schema.decodeTo(Schema.toType(Schema.Array(WorkLink)), {
    decode: SchemaGetter.transform((links) => links.filter((link) => isSafeWorkHref(link.url))),
    encode: SchemaGetter.transform((links) => links),
  }),
);

/**
 * A tracked thread's facts, on `Thread.work` for every room member. `thread.updated` brings new
 * facts on every work change; a client holding the `WorkDetail` refetches the thread when they
 * differ from the ones it holds. `owner` is the whole user, since thread events carry no
 * `users`; `null` reads "Unassigned".
 */
export const WorkFacts = Schema.Struct({
  status: WorkStatusRead,
  owner: Schema.NullOr(User),
  ownerActive: Schema.Boolean,
  runUrl: RunUrl,
  resultUpdatedAt: Schema.NullOr(Timestamp),
  links: WorkLinks,
  tags: Schema.Array(Schema.String),
  /** Every message, streaming ones and system notes included, as classic board rows count. */
  messageCount: Schema.Number,
  updatedAt: RowTimestamp,
});

export type WorkFacts = typeof WorkFacts.Type;

export type WorkFactsPin = Assert<Pinned<typeof WorkFacts, GeneratedWorkFacts>>;

/** One owner choice; `provider` and `description` are an agent's, `null` for people. */
export const WorkOwnerCandidate = Schema.Struct({
  userId: UserId,
  provider: Schema.NullOr(Schema.String),
  description: Schema.NullOr(Schema.String),
});

export type WorkOwnerCandidate = typeof WorkOwnerCandidate.Type;

export type WorkOwnerCandidatePin = Assert<
  Pinned<typeof WorkOwnerCandidate, GeneratedWorkOwnerCandidate>
>;

export const WorkHandoffReceiver = Schema.Struct({ agentId: AgentId, userId: UserId });

export type WorkHandoffReceiver = typeof WorkHandoffReceiver.Type;

export type WorkHandoffReceiverPin = Assert<
  Pinned<typeof WorkHandoffReceiver, GeneratedWorkHandoffReceiver>
>;

/** Tolerant, like `WorkLinkKind`. */
export const WorkHistoryKind = tolerantLiterals(["update", "assignment", "handoff", "result"]);

export type WorkHistoryKind = typeof WorkHistoryKind.Type;

export type WorkHistoryKindPin = Assert<Pinned<typeof WorkHistoryKind, GeneratedWorkHistoryKind>>;

/**
 * An owner as recorded at the time; the name survives renames and deleted accounts. A `null`
 * name reads "Unassigned", as in the classic history.
 */
export const WorkOwnerSnapshot = Schema.Struct({
  userId: Schema.NullOr(UserId),
  name: Schema.NullOr(Schema.String),
});

export type WorkOwnerSnapshot = typeof WorkOwnerSnapshot.Type;

export type WorkOwnerSnapshotPin = Assert<
  Pinned<typeof WorkOwnerSnapshot, GeneratedWorkOwnerSnapshot>
>;

export const WorkHistoryHandoff = Schema.Struct({
  summary: Schema.String,
  linkCount: Schema.Int,
  questionCount: Schema.Int,
});

export type WorkHistoryHandoff = typeof WorkHistoryHandoff.Type;

export type WorkHistoryHandoffPin = Assert<
  Pinned<typeof WorkHistoryHandoff, GeneratedWorkHistoryHandoff>
>;

/**
 * One line of the work history. A `null` status is an ordinary (untracked) thread, a `null`
 * owner "Unassigned", and a `null` actor (or one with no user) "Former member".
 */
export const WorkHistoryEntry = Schema.Struct({
  id: WorkEventId,
  kind: WorkHistoryKind,
  createdAt: Timestamp,
  actorId: Schema.NullOr(UserId),
  fromStatus: Schema.NullOr(WorkStatusRead),
  toStatus: Schema.NullOr(WorkStatusRead),
  fromOwner: Schema.NullOr(WorkOwnerSnapshot),
  toOwner: Schema.NullOr(WorkOwnerSnapshot),
  note: Schema.NullOr(Schema.String),
  handoff: Schema.NullOr(WorkHistoryHandoff),
});

export type WorkHistoryEntry = typeof WorkHistoryEntry.Type;

export type WorkHistoryEntryPin = Assert<
  Pinned<typeof WorkHistoryEntry, GeneratedWorkHistoryEntry>
>;

/**
 * The thread page's work section for this viewer (`ThreadDetail.work`). `ownerCandidates` is
 * empty unless the viewer may assign the work, `handoffReceivers` unless they may manage it.
 */
export const WorkDetail = Schema.Struct({
  resultMarkdown: Schema.NullOr(Schema.String),
  resultHtml: Schema.NullOr(Schema.String),
  resultUpdatedById: Schema.NullOr(UserId),
  steps: Schema.Array(AgentStep),
  history: Schema.Array(WorkHistoryEntry),
  ownerCandidates: Schema.Array(WorkOwnerCandidate),
  handoffReceivers: Schema.Array(WorkHandoffReceiver),
});

export type WorkDetail = typeof WorkDetail.Type;

export type WorkDetailPin = Assert<Pinned<typeof WorkDetail, GeneratedWorkDetail>>;
