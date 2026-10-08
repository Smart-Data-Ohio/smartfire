/**
 * Agents at work, for the room: who among a room's agents is working now (from `agent.status`),
 * and what each says it's doing (its working presence, which lapses at `expiresAt` with no event,
 * so the hook re-renders when it's up).
 */
import { useEffect, useState } from "react";
import { useStore as useZustandStore } from "zustand";
import { useShallow } from "zustand/react/shallow";
import { isWorking, roomAgentIds } from "../../store/agents.ts";
import type { State } from "../../store/state.ts";
import { store, useStore } from "../../store/store.ts";

const NONE: readonly number[] = [];

/** The user ids of a room's agents that are working now (not suspended), lowest id first. */
export function workingAgentIds(state: State, roomId: number): readonly number[] {
  const ids = roomAgentIds(state, roomId).filter((id) => isWorking(state.users[id]));

  return ids.length === 0 ? NONE : ids;
}

/** {@link workingAgentIds} as a hook; the array changes only when the ids do. */
export function useWorkingAgents(roomId: number): readonly number[] {
  return useZustandStore(
    store,
    useShallow((state: State) => workingAgentIds(state, roomId)),
  );
}

/**
 * What an agent (by its user id) says it's doing now, or `null` when unset or lapsed. The lapse
 * comes with no event: a timer re-renders at `expiresAt`.
 */
export function useWorkingPresence(userId: number | undefined): string | null {
  const presence = useStore((state) => {
    const agentId = userId === undefined ? undefined : state.users[userId]?.agent?.agentId;

    return agentId === undefined ? undefined : state.agents.working[agentId];
  });

  const [lapsed, setLapsed] = useState<string | null>(null);

  useEffect(() => {
    if (presence === undefined) {
      return;
    }

    const timer = window.setTimeout(
      () => setLapsed(presence.expiresAt),
      Math.max(0, Date.parse(presence.expiresAt) - Date.now()),
    );

    return () => window.clearTimeout(timer);
  }, [presence]);

  return presence === undefined || lapsed === presence.expiresAt ? null : presence.text;
}

/** "Ember is working", "Ember and Scout are working", "Several agents are working". */
export function workingSentence(names: readonly string[]): string {
  if (names.length === 1) {
    return `${names[0]} is working`;
  }

  if (names.length === 2) {
    return `${names[0]} and ${names[1]} are working`;
  }

  return "Several agents are working";
}
