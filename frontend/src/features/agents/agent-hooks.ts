/**
 * The hooks the agent pages read with. Each page loads when it's shown (a profile or directory
 * already held stays on screen while it reloads) and keeps up with `agent.status` after that.
 */
import { useCallback, useEffect, useMemo } from "react";
import type { AgentDirectoryRow } from "../../gen/AgentDirectoryRow.ts";
import { type AgentProfileEntry, profileOf } from "../../store/agents.ts";
import type { LoadStatus } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";

/** The directory as a page reads it. */
export interface AgentDirectoryView {
  /** `idle`/`loading` (skeleton), `ready`, or `error` (nothing shown). */
  readonly status: LoadStatus;
  /** A reload's failure while rows stay shown, or the first load's. */
  readonly error: string | null;
  /** In the server's order: active agents first, then by name. */
  readonly rows: readonly AgentDirectoryRow[];
  readonly reload: () => void;
}

/** The agent directory, loaded each time it's shown and kept live by `agent.status`. */
export function useAgentDirectory(): AgentDirectoryView {
  const directory = useStore((state) => state.agents.directory);
  const byId = useStore((state) => state.agents.rows);
  const reload = useCallback(() => void actions.agents.loadDirectory(), []);

  useEffect(reload, [reload]);

  return useMemo(
    () => ({
      status: directory.status,
      error: directory.error,
      rows: directory.ids.flatMap((id) => byId[id] ?? []),
      reload,
    }),
    [directory, byId, reload],
  );
}

/** One agent's profile, loaded each time it's shown and kept live by `agent.status`. */
export function useAgentProfile(agentId: number): AgentProfileEntry & { reload: () => void } {
  const entry = useStore((state) => profileOf(state, agentId));
  const reload = useCallback(() => void actions.agents.loadProfile(agentId), [agentId]);

  useEffect(reload, [reload]);

  return useMemo(() => ({ ...entry, reload }), [entry, reload]);
}
