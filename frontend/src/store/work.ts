/**
 * Work tracking in the store (S4). A thread's work facts live on the thread itself
 * (`state.threads[id].work`, kept by `thread.updated`); this slice holds each open pane's
 * `WorkDetail`, the facts that detail was loaded with, the writes in flight and their overlays,
 * and the work list per filter. List membership reloads when shown again; loaded rows follow
 * live work facts. Server revisions order the thread fields, while local observations order
 * untimestamped tracking absence, owner eligibility and links.
 */
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { WorkDetail } from "../gen/WorkDetail.ts";
import type { WorkFacts } from "../gen/WorkFacts.ts";
import type { WorkFilter } from "../gen/WorkFilter.ts";
import type { WorkLink } from "../gen/WorkLink.ts";
import type { WorkList } from "../gen/WorkList.ts";
import type { WorkListRow } from "../gen/WorkListRow.ts";
import type { WorkStatus } from "../gen/WorkStatus.ts";
import { mergeSteps } from "./agents.ts";
import type { LoadStatus, Thread } from "./model.ts";
import { mergeUserList } from "./ordering.ts";
import { landsOver, mergeRevision } from "./revision.ts";
import type { State } from "./state.ts";
import { loadThreadDetail, upsertThread } from "./threads.ts";

/** One filter's work list. */
export interface WorkListState {
  readonly status: LoadStatus;
  /** Most recently updated first, as the server sent them. */
  readonly rows: readonly WorkListRow[];
  readonly error: string | null;
  /** Bumped by every load; older replies still merge records but cannot replace this list. */
  readonly generation: number;
}

interface WorkOverlay {
  readonly before: WorkFacts | null;
  readonly shown: WorkFacts | null;
  readonly confirmed: WorkFacts | null;
}

interface WorkFields {
  readonly tracking: number;
  readonly ownerActive: number;
  readonly links: number;
}

export interface WorkSlice {
  /** By thread id: the open pane's work section; absent while untracked or not loaded. */
  readonly details: Readonly<Record<number, WorkDetail>>;
  /** By thread id: the facts the held detail goes with (`null`: loaded untracked). */
  readonly heldFacts: Readonly<Record<number, WorkFacts | null>>;
  /** By thread id: work writes on their way (no refetch while any is). */
  readonly writes: Readonly<Record<number, number>>;
  readonly lists: Readonly<Partial<Record<WorkFilter, WorkListState>>>;
  /** Each pending write keeps its base revision, displayed copy and latest confirmed fields. */
  readonly overlays: Readonly<Record<number, WorkOverlay>>;
  /** Neither null tracking, owner eligibility nor links carries a server timestamp. */
  readonly fields: Readonly<Record<number, WorkFields>>;
}

/** A read captures only fields outside WorkFacts.updatedAt, never record freshness. */
export type WorkRead = WorkSlice["fields"];

export const emptyWork: WorkSlice = {
  details: {},
  heldFacts: {},
  writes: {},
  lists: {},
  overlays: {},
  fields: {},
};

const emptyFields: WorkFields = { tracking: 0, ownerActive: 0, links: 0 };

export function captureWorkRead(state: State): WorkRead {
  return state.work.fields;
}

function withWork(state: State, change: Partial<WorkSlice>): State {
  return { ...state, work: { ...state.work, ...change } };
}

function withoutOverlay(state: State, threadId: number): State {
  const { [threadId]: _overlay, ...overlays } = state.work.overlays;

  return withWork(state, { overlays });
}

function showFacts(
  state: State,
  thread: Thread,
  observations: {
    readonly absence?: boolean;
    readonly eligibility?: boolean;
    readonly links?: boolean;
  } = {},
): State {
  const before = state.threads[thread.id]?.work ?? null;
  const fields = state.work.fields[thread.id] ?? emptyFields;
  const next = upsertThread(state, thread);
  const lists = { ...next.work.lists };

  for (const filter of WORK_FILTERS) {
    const list = lists[filter];

    if (list !== undefined) {
      lists[filter] = {
        ...list,
        rows: list.rows.map((row) =>
          row.thread.id === thread.id
            ? { ...row, thread: { ...row.thread, work: thread.work } }
            : row,
        ),
      };
    }
  }

  return withWork(next, {
    lists,
    fields: {
      ...next.work.fields,
      [thread.id]: {
        // A newer tracked revision also supersedes a held, untimestamped absence snapshot.
        tracking:
          fields.tracking +
          Number(
            observations.absence === true ||
              (before === null) !== (thread.work === null) ||
              (before !== null && thread.work !== null && thread.work.updatedAt > before.updatedAt),
          ),
        ownerActive:
          fields.ownerActive +
          Number(
            observations.eligibility === true ||
              before?.owner?.id !== thread.work?.owner?.id ||
              before?.ownerActive !== thread.work?.ownerActive,
          ),
        links:
          fields.links +
          Number(
            observations.links === true ||
              !sameLinks(before?.links ?? [], thread.work?.links ?? []),
          ),
      },
    },
  });
}

/** Only status/assignment fields changed locally overlay the confirmed copy. */
function overlayFacts(overlay: WorkOverlay, confirmed: WorkFacts | null): WorkFacts | null {
  const { before, shown } = overlay;

  if (before === null || shown === null || confirmed === null) {
    return shown;
  }

  if (confirmed === before) {
    return shown;
  }

  return {
    ...confirmed,
    status: shown.status === before.status ? confirmed.status : shown.status,
    owner: shown.owner === before.owner ? confirmed.owner : shown.owner,
    ownerActive:
      shown.owner?.id === before.owner?.id && shown.ownerActive === before.ownerActive
        ? confirmed.ownerActive
        : shown.ownerActive,
  };
}

function mergedFacts(state: State, thread: Thread, event: boolean, read?: WorkRead) {
  const id = thread.id;
  const overlay = state.work.overlays[id];
  const stored = overlay === undefined ? state.threads[id]?.work : overlay.confirmed;
  const incoming = thread.work;
  const fields = state.work.fields[id] ?? emptyFields;
  const captured = read?.[id] ?? emptyFields;
  const trackingCurrent = read === undefined || captured.tracking === fields.tracking;
  const eligibilityCurrent = read === undefined || captured.ownerActive === fields.ownerActive;
  const linksCurrent = read === undefined || captured.links === fields.links;

  let confirmed = stored ?? null;
  let observedEligibility = false;
  let observedLinks = false;

  if (incoming === null) {
    if (event || (trackingCurrent && overlay === undefined)) {
      confirmed = null;
    }
  } else if (stored != null || trackingCurrent) {
    confirmed = mergeRevision(stored, incoming);

    // Link rows can change without advancing the thread's work revision.
    observedLinks = linksCurrent;

    const links = linksCurrent ? incoming.links : (stored?.links ?? confirmed.links);

    confirmed = confirmed.links === links ? confirmed : { ...confirmed, links };

    // Membership/grants may change eligibility without republishing the work revision.
    if (confirmed.owner?.id === incoming.owner?.id) {
      observedEligibility = eligibilityCurrent;

      const ownerActive = eligibilityCurrent
        ? incoming.ownerActive
        : stored != null && stored.owner?.id === incoming.owner?.id
          ? stored.ownerActive
          : confirmed.ownerActive;

      confirmed = confirmed.ownerActive === ownerActive ? confirmed : { ...confirmed, ownerActive };
    }
  }

  const pending =
    overlay !== undefined &&
    (incoming === null
      ? !event
      : confirmed === null ||
        (overlay.before !== null && incoming.updatedAt <= overlay.before.updatedAt));

  return {
    facts: pending ? overlayFacts(overlay, confirmed) : confirmed,
    pending,
    confirmed,
    observedEligibility,
    observedLinks,
    observedAbsence: incoming === null && !pending && (event || trackingCurrent),
  };
}

/** Every event, read and list merges the same server record. */
export function receiveWorkThread(
  state: State,
  thread: Thread,
  read?: WorkRead,
  source: "event" | "read" = "event",
): State {
  const result = mergedFacts(state, thread, source === "event", read);
  const overlay = state.work.overlays[thread.id];

  const next =
    result.pending && overlay !== undefined
      ? withWork(state, {
          overlays: {
            ...state.work.overlays,
            [thread.id]: { ...overlay, confirmed: result.confirmed },
          },
        })
      : withoutOverlay(state, thread.id);

  return showFacts(
    next,
    { ...thread, work: result.facts },
    {
      absence: result.observedAbsence,
      eligibility: result.observedEligibility,
      links: result.observedLinks,
    },
  );
}

/** Thread permissions and membership land independently of the work record. */
export function loadWorkThreadDetail(state: State, detail: ThreadDetail, read?: WorkRead): State {
  const next = receiveWorkThread(state, detail.thread, read, "read");
  const thread = next.threads[detail.thread.id] ?? detail.thread;

  return mergeWorkDetail(loadThreadDetail(next, { ...detail, thread }), detail);
}

/** The filters, in the work page's tab order. */
export const WORK_FILTERS: readonly WorkFilter[] = ["open", "done", "all", "agents", "boards"];

export const emptyWorkList: WorkListState = {
  status: "idle",
  rows: [],
  error: null,
  generation: 0,
};

/** One filter's list, or an empty one never loaded. */
export function workListOf(state: State, filter: WorkFilter): WorkListState {
  return state.work.lists[filter] ?? emptyWorkList;
}

function sameLink(a: WorkLink, b: WorkLink): boolean {
  return (
    a.id === b.id &&
    a.kind === b.kind &&
    a.label === b.label &&
    a.url === b.url &&
    a.pullRequestState === b.pullRequestState &&
    a.title === b.title &&
    a.eventStartsAt === b.eventStartsAt &&
    a.eventTimeZone === b.eventTimeZone &&
    a.eventCancelled === b.eventCancelled
  );
}

function sameLinks(a: readonly WorkLink[], b: readonly WorkLink[]): boolean {
  return (
    a.length === b.length &&
    a.every((link, index) => {
      const other = b[index];

      return other !== undefined && sameLink(link, other);
    })
  );
}

/** Whether two copies have the same revision and displayed work facts, links included. */
export function sameWorkFacts(a: WorkFacts | null, b: WorkFacts | null): boolean {
  if (a === null || b === null) {
    return a === b;
  }

  return (
    a.updatedAt === b.updatedAt &&
    a.status === b.status &&
    (a.owner?.id ?? null) === (b.owner?.id ?? null) &&
    a.ownerActive === b.ownerActive &&
    a.runUrl === b.runUrl &&
    a.resultUpdatedAt === b.resultUpdatedAt &&
    sameLinks(a.links, b.links)
  );
}

/**
 * Whether the pane's loaded work detail is behind the live facts. Writes and overlays defer
 * refetching; owner eligibility and links do not change the result or history it holds.
 */
export function workDetailStale(state: State, threadId: number): boolean {
  const held = state.work.heldFacts[threadId];
  const live = state.threads[threadId]?.work;

  return (
    held !== undefined &&
    live !== undefined &&
    (state.work.writes[threadId] ?? 0) === 0 &&
    state.work.overlays[threadId] === undefined &&
    !sameWorkFacts(
      held === null || live === null
        ? held
        : { ...held, ownerActive: live.ownerActive, links: live.links },
      live,
    )
  );
}

/** Result/history use the work revision; steps have their own independent revisions. */
function mergeWorkDetail(state: State, detail: ThreadDetail): State {
  const id = detail.thread.id;
  const held = state.work.details[id];
  const incoming = detail.thread.work;
  const live = state.threads[id]?.work;
  const heldFacts = state.work.heldFacts[id];

  const accepted =
    state.work.overlays[id] === undefined &&
    (incoming === null
      ? live == null
      : live != null && landsOver(live, incoming) && landsOver(heldFacts, incoming));

  const steps = mergeSteps(held?.steps ?? [], detail.work?.steps ?? []);

  if (!accepted) {
    if (held === undefined) {
      // The first detail was overtaken by an event; remember that its content is incomplete.
      return state.work.overlays[id] === undefined
        ? withWork(state, {
            heldFacts: { ...state.work.heldFacts, [id]: incoming },
          })
        : state;
    }

    const fields = detail.work;

    return withWork(state, {
      details: {
        ...state.work.details,
        [id]: {
          ...held,
          steps: [...steps],
          ownerCandidates: fields?.ownerCandidates ?? held.ownerCandidates,
          handoffReceivers: fields?.handoffReceivers ?? held.handoffReceivers,
        },
      },
    });
  }

  const { [id]: _old, ...others } = state.work.details;
  const base = detail.work;

  // Assignment candidates depend on current membership, not the work revision.
  return withWork(state, {
    details: base === null ? others : { ...others, [id]: { ...base, steps: [...steps] } },
    heldFacts: { ...state.work.heldFacts, [id]: incoming },
  });
}

/** A `ThreadDetail` landed: merge its work section by the same rule as the pane mutation. */
export function landWorkDetail(state: State, detail: ThreadDetail): State {
  return loadWorkThreadDetail(state, detail);
}

/** A work write started (`+1`) or settled (`-1`). */
export function countWorkWrite(state: State, threadId: number, delta: 1 | -1): State {
  const count = Math.max(0, (state.work.writes[threadId] ?? 0) + delta);
  const { [threadId]: _count, ...others } = state.work.writes;

  return withWork(state, { writes: count === 0 ? others : { ...others, [threadId]: count } });
}

/**
 * The facts a status change shows at once: a new status (tracking starts unassigned, with no
 * links), or `null` to stop tracking. A new local record starts below any confirmed revision.
 */
export function optimisticFacts(
  current: WorkFacts | null,
  status: WorkStatus | null,
): WorkFacts | null {
  if (status === null) {
    return null;
  }

  return current === null
    ? {
        status,
        owner: null,
        ownerActive: false,
        runUrl: null,
        resultUpdatedAt: null,
        links: [],
        updatedAt: "0001-01-01T00:00:00.000000Z",
      }
    : { ...current, status };
}

/** Show a write's optimistic facts while preserving the confirmed base for merge and rollback. */
export function putWorkFacts(state: State, threadId: number, facts: WorkFacts | null): State {
  const thread = state.threads[threadId];

  if (thread === undefined) {
    return state;
  }

  const next = showFacts(state, { ...thread, work: facts });

  return withWork(next, {
    overlays: {
      ...next.work.overlays,
      [threadId]: { before: thread.work, shown: facts, confirmed: thread.work },
    },
  });
}

/** A refusal restores only the overlay belonging to this write. */
export function rollbackWork(state: State, threadId: number, shown: WorkFacts | null): State {
  const overlay = state.work.overlays[threadId];
  const thread = state.threads[threadId];

  return overlay?.shown !== shown || thread === undefined
    ? state
    : showFacts(withoutOverlay(state, threadId), { ...thread, work: overlay.confirmed });
}

/** Replies confirm ties; ordinary GETs at the base revision leave the overlay in flight. */
export function landWorkReply(
  state: State,
  detail: ThreadDetail,
  shown: WorkFacts | null | undefined,
  read?: WorkRead,
): State {
  const id = detail.thread.id;
  const overlay = state.work.overlays[id];
  const incoming = detail.thread.work;

  if (
    overlay !== undefined &&
    overlay.shown === shown &&
    (incoming === null || landsOver(overlay.before, incoming))
  ) {
    const thread = state.threads[id];

    if (thread !== undefined) {
      state = showFacts(withoutOverlay(state, id), { ...thread, work: overlay.confirmed });

      // The reply settles its own tracking/assignment; links may have changed independently.
      const fields = state.work.fields[id] ?? emptyFields;

      const replyRead =
        read === undefined
          ? undefined
          : {
              ...read,
              [id]: { ...fields, links: (read[id] ?? emptyFields).links },
            };

      return loadWorkThreadDetail(state, detail, replyRead);
    }
  }

  return loadWorkThreadDetail(state, detail, read);
}

function updateList(
  state: State,
  filter: WorkFilter,
  change: (list: WorkListState) => WorkListState,
): State {
  return withWork(state, {
    lists: { ...state.work.lists, [filter]: change(workListOf(state, filter)) },
  });
}

/** A load started: a list never shown says loading; a shown one keeps its rows. */
export function setWorkListLoading(state: State, filter: WorkFilter): State {
  return updateList(state, filter, (list) => ({
    ...list,
    status: list.status === "ready" ? "ready" : "loading",
    error: null,
    generation: list.generation + 1,
  }));
}

/** A load failed: a list never shown says so; a shown one keeps its rows and the message. */
export function setWorkListFailed(
  state: State,
  filter: WorkFilter,
  error: string,
  generation: number,
): State {
  return updateList(state, filter, (list) =>
    list.generation === generation
      ? { ...list, status: list.status === "ready" ? "ready" : "error", error }
      : list,
  );
}

/** Merge each row's facts and users; only the current load replaces the list's rows. */
export function landWorkList(
  state: State,
  filter: WorkFilter,
  list: WorkList,
  generation: number,
  read?: WorkRead,
): State {
  const users = mergeUserList(state.users, list.users);

  let next = users === state.users ? state : { ...state, users };

  const rows = list.threads.map((row) => {
    next = receiveWorkThread(next, row.thread, read, "read");

    return { ...row, thread: next.threads[row.thread.id] ?? row.thread };
  });

  return workListOf(state, filter).generation !== generation
    ? next
    : updateList(next, filter, (held) => ({
        ...held,
        status: "ready",
        rows,
        error: null,
      }));
}
