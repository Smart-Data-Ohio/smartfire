/** Work-only seed state, shared by the model, handlers and seed without importing them. */
import type { AgentCapability } from "../../src/gen/AgentCapability.ts";
import type { ThreadRecord } from "../s2/model.ts";
import type { World } from "../seed.ts";

export const S4_BOARD = { roomId: 41, name: "Product roadmap" } as const;

export interface WorkState {
  readonly boardPosts: ThreadRecord[];
  /** Explicit capability sets; other seeded agents keep all work capabilities. */
  readonly agentCapabilities: Map<`${number}:${number}`, ReadonlySet<AgentCapability>>;
  nextHistoryId: number;
  nextLinkId: number;
}

const states = new WeakMap<World, WorkState>();

export function workStateOf(world: World): WorkState {
  const existing = states.get(world);

  if (existing !== undefined) return existing;

  const fresh: WorkState = {
    boardPosts: [],
    agentCapabilities: new Map(),
    nextHistoryId: 1,
    nextLinkId: 1,
  };

  states.set(world, fresh);

  return fresh;
}
