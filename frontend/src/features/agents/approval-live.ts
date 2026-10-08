import { useEffect, useEffectEvent, useRef } from "react";
import type { AgentApproval } from "../../gen/AgentApproval.ts";
import type { AgentApprovalStatus } from "../../gen/AgentApprovalStatus.ts";
import { store, useStore } from "../../store/store.ts";
import { approvalStatusLabel } from "./approval-format.ts";

/** How long a burst of decisions made elsewhere gathers before it's announced as one. */
export const LIVE_BATCH_MS = 1000;

/**
 * The requests shown as pending (in `watched`) that are decided now, by someone else: every
 * one not in `local`, the viewer's own decisions, which announce themselves.
 */
export function liveDecisions(
  watched: ReadonlyMap<number, AgentApprovalStatus>,
  items: Readonly<Record<number, AgentApproval>>,
  local: ReadonlySet<number>,
): readonly AgentApproval[] {
  const decided: AgentApproval[] = [];

  for (const [id, was] of watched) {
    const now = items[id];

    if (was === "pending" && now !== undefined && now.status !== "pending" && !local.has(id)) {
      decided.push(now);
    }
  }

  return decided;
}

/**
 * What a burst of decisions made elsewhere says: one names its status, decider (when there is
 * one) and summary; several are counted.
 */
export function liveDecisionAnnouncement(
  decided: readonly AgentApproval[],
  nameOf: (userId: number) => string | null,
): string {
  const [only] = decided;

  if (decided.length !== 1 || only === undefined) {
    return `${decided.length} approval requests were decided`;
  }

  const decider = only.decidedById === null ? null : nameOf(only.decidedById);
  const label = approvalStatusLabel(only.status);

  return decider === null ? `${label}: ${only.summary}` : `${label} by ${decider}: ${only.summary}`;
}

/**
 * What to watch next: the shown rows' statuses, plus requests watched as pending that left the
 * list while their copy is still pending (a reload can drop one before its event arrives).
 */
export function nextWatched(
  previous: ReadonlyMap<number, AgentApprovalStatus> | null,
  rows: readonly AgentApproval[],
  items: Readonly<Record<number, AgentApproval>>,
): Map<number, AgentApprovalStatus> {
  const next = new Map(rows.map((row) => [row.id, row.status]));

  for (const [id, was] of previous ?? []) {
    if (was === "pending" && !next.has(id) && items[id]?.status === "pending") {
      next.set(id, "pending");
    }
  }

  return next;
}

interface LiveDecisionNotes {
  /** The viewer is deciding `id` here: its change isn't announced as someone else's. */
  readonly noteLocal: (id: number) => void;
  /** The viewer's decision on `id` was refused: a later change is someone else's again. */
  readonly forgetLocal: (id: number) => void;
}

/**
 * Announces, politely, requests on the shown list (`key`) that someone else decides while it's
 * shown. The first load of each list says nothing, and a burst gathers into one message.
 */
export function useLiveDecisionAnnouncements(
  key: string,
  ready: boolean,
  rows: readonly AgentApproval[],
  announce: (text: string) => void,
): LiveDecisionNotes {
  const items = useStore((state) => state.approvals.items);
  const watched = useRef<{ key: string; statuses: Map<number, AgentApprovalStatus> } | null>(null);
  const local = useRef(new Set<number>());
  const queued = useRef<AgentApproval[]>([]);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const flush = useEffectEvent(() => {
    timer.current = null;

    const decided = queued.current;

    queued.current = [];

    if (decided.length > 0) {
      announce(
        liveDecisionAnnouncement(decided, (userId) => store.getState().users[userId]?.name ?? null),
      );
    }
  });

  useEffect(() => {
    if (!ready) {
      return;
    }

    const held = watched.current;

    if (held !== null && held.key === key) {
      const decided = liveDecisions(held.statuses, items, local.current);

      for (const [id, was] of held.statuses) {
        // The viewer's own decision showed (announced already); a later change is someone else's.
        if (was === "pending" && local.current.has(id) && items[id]?.status !== "pending") {
          local.current.delete(id);
        }
      }

      if (decided.length > 0) {
        queued.current = [...queued.current, ...decided];
        timer.current ??= setTimeout(flush, LIVE_BATCH_MS);
      }
    }

    watched.current = {
      key,
      statuses: nextWatched(held?.key === key ? held.statuses : null, rows, items),
    };
  }, [key, ready, rows, items]);

  useEffect(
    () => () => {
      if (timer.current !== null) {
        clearTimeout(timer.current);
      }
    },
    [],
  );

  return {
    noteLocal: (id) => local.current.add(id),
    forgetLocal: (id) => local.current.delete(id),
  };
}
