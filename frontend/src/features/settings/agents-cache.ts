/**
 * Settings → Agents' copy of what the server holds.
 *
 * An agent's profile and access: latest sent wins. Every rename, access save, removal and reload
 * of the whole list takes a number from `nextSequence()` when it is sent, and an answer lands only
 * where no answer to a later-sent request has landed. So a slow reload never puts back an older
 * name, and an older reload can't bring back a removed agent (it keeps its number).
 *
 * Its tokens and connections: their state only ever moves forward, from active to expired to
 * revoked, and they are never deleted. So every answer's state is merged in by that, not by when it
 * was sent (the server may carry out two requests in the other order): a token any answer showed
 * revoked stays revoked, one an answer doesn't mention stays as it is, and a new one is added.
 * What else can change about one (when it was last used, whether an administrator blocked its app,
 * why it was revoked) goes back and forth, so for that the latest sent wins, as for profiles: a
 * slow answer can't unblock a connection a later one showed blocked. Why it was revoked comes with
 * the revocation, so it's taken from the latest-sent answer that shows it revoked.
 *
 * And which agents a save went unanswered for: it may or may not have landed, so the cache can't
 * tell what the server holds for them until an answer to a request sent after it says.
 */
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";

export interface AgentCache {
  readonly list: PersonalAgentList;
  /** The newest request applied to each agent's profile and access, removed agents included. */
  readonly applied: ReadonlyMap<number, number>;
  /** The newest whole-list load applied: it speaks for everything not written to since. */
  readonly listSequence: number;
  /** Agents with a save that went unanswered, by the number that save was sent as. */
  readonly unconfirmed: ReadonlyMap<number, number>;
  /** The newest request whose answer set each token's details (all but its state), by token. */
  readonly tokensAt: ReadonlyMap<number, number>;
}

let sequence = 0;

/** The next request's number; numbers only grow. */
export function nextSequence(): number {
  sequence += 1;

  return sequence;
}

/** The newest request applied to agent `agentId`'s profile and access. */
function profileAt(cache: AgentCache, agentId: number): number {
  return Math.max(cache.applied.get(agentId) ?? 0, cache.listSequence);
}

/** How far along a token is; it never goes back. */
const STAGE: Readonly<Record<PersonalAgentCredential["state"], number>> = {
  active: 0,
  expired: 1,
  revoked: 2,
};

/** `fresh`'s state when it's further along than `held`'s; else `held`'s. */
function furthest(
  held: PersonalAgentCredential,
  fresh: PersonalAgentCredential,
): PersonalAgentCredential {
  return STAGE[fresh.state] > STAGE[held.state] ? fresh : held;
}

/** One token from two answers: the furthest state, the latest-sent answer's details. */
function mergedToken(
  held: PersonalAgentCredential,
  fresh: PersonalAgentCredential,
  freshNewer: boolean,
): PersonalAgentCredential {
  const details = freshNewer ? fresh : held;
  const state = furthest(held, fresh).state;

  // The reason belongs to a revocation: from the details when they show it, else the other answer.
  const revokedBy = details.state === "revoked" ? details : furthest(held, fresh);

  const connection =
    details.connection === null
      ? null
      : {
          ...details.connection,
          revokedReason: state === "revoked" ? (revokedBy.connection?.revokedReason ?? null) : null,
        };

  return { ...details, state, connection };
}

interface Tokens {
  readonly credentials: PersonalAgentCredential[];
  readonly tokensAt: ReadonlyMap<number, number>;
}

/**
 * `held` tokens with `incoming` ones from the answer to request `at` merged in: each the furthest
 * along, its details from the latest-sent answer, new ones added. `at` is `null` for tokens the
 * page made up itself (a rotation's replaced token), whose details are never newer than held ones.
 */
function mergedTokens(
  held: readonly PersonalAgentCredential[],
  incoming: readonly PersonalAgentCredential[],
  at: number | null,
  tokensAt: ReadonlyMap<number, number>,
): Tokens {
  const next = new Map(tokensAt);

  const newer = (id: number) => {
    if (at === null || at <= (tokensAt.get(id) ?? 0)) return false;
    next.set(id, at);

    return true;
  };

  const merged = held.map((token) => {
    const fresh = incoming.find((t) => t.id === token.id);

    return fresh === undefined ? token : mergedToken(token, fresh, newer(token.id));
  });

  const added = incoming.filter((fresh) => !held.some((token) => token.id === fresh.id));

  for (const token of added) newer(token.id);

  return { credentials: [...merged, ...added], tokensAt: next };
}

/** `unconfirmed` without agent `agentId`, if request `at` was sent after its unanswered save. */
function confirmed(
  unconfirmed: ReadonlyMap<number, number>,
  agentId: number,
  at: number,
): ReadonlyMap<number, number> {
  const since = unconfirmed.get(agentId);

  if (since === undefined || at <= since) return unconfirmed;

  const next = new Map(unconfirmed);

  next.delete(agentId);

  return next;
}

/** A whole-list load sent as request `at`, over what `cache` holds (if anything yet). */
export function withList(
  cache: AgentCache | null,
  fresh: PersonalAgentList,
  at: number,
): AgentCache {
  if (cache === null) {
    const tokensAt = new Map(
      fresh.agents.flatMap((agent) => agent.credentials.map((token) => [token.id, at] as const)),
    );

    return { list: fresh, applied: new Map(), listSequence: at, unconfirmed: new Map(), tokensAt };
  }

  if (at <= cache.listSequence) return cache;

  const listed = new Set(fresh.agents.map((agent) => agent.id));
  let tokensAt = cache.tokensAt;

  const agents = fresh.agents.flatMap((agent) => {
    const held = cache.list.agents.find((a) => a.id === agent.id);
    const profileNewer = (cache.applied.get(agent.id) ?? 0) > at;

    // Removed after this load was sent.
    if (held === undefined) return profileNewer ? [] : [agent];

    const tokens = mergedTokens(held.credentials, agent.credentials, at, tokensAt);

    tokensAt = tokens.tokensAt;

    return [{ ...(profileNewer ? held : agent), credentials: tokens.credentials }];
  });

  // Connected after this load was sent, so it couldn't list them.
  const later = cache.list.agents.filter(
    (agent) => !listed.has(agent.id) && (cache.applied.get(agent.id) ?? 0) > at,
  );

  const unconfirmed = [...cache.unconfirmed.keys()].reduce(
    (current, agentId) => confirmed(current, agentId, at),
    cache.unconfirmed,
  );

  return {
    list: { ...fresh, agents: [...agents, ...later] },
    applied: cache.applied,
    listSequence: at,
    unconfirmed,
    tokensAt,
  };
}

/** An agent as the answer to request `at` (a rename or an access save) gives it. */
export function withAgent(cache: AgentCache, agent: PersonalAgent, at: number): AgentCache {
  const held = cache.list.agents.find((a) => a.id === agent.id);

  if (held === undefined) return cache;

  const tokens = mergedTokens(held.credentials, agent.credentials, at, cache.tokensAt);
  const newer = at > profileAt(cache, agent.id);
  const next = { ...(newer ? agent : held), credentials: tokens.credentials };
  const applied = new Map(cache.applied);

  if (newer) applied.set(agent.id, at);

  return {
    ...cache,
    list: { ...cache.list, agents: cache.list.agents.map((a) => (a.id === agent.id ? next : a)) },
    applied,
    unconfirmed: confirmed(cache.unconfirmed, agent.id, at),
    tokensAt: tokens.tokensAt,
  };
}

/** A save for agent `agentId`, sent as request `at`, that went unanswered. */
export function withUnconfirmed(cache: AgentCache, agentId: number, at: number): AgentCache {
  if (!cache.list.agents.some((a) => a.id === agentId)) return cache;

  const unconfirmed = new Map(cache.unconfirmed);

  unconfirmed.set(agentId, Math.max(unconfirmed.get(agentId) ?? 0, at));

  return { ...cache, unconfirmed };
}

/** Whether a save for agent `agentId` went unanswered and nothing sent since has answered. */
export function isUnconfirmed(cache: AgentCache, agentId: number): boolean {
  return cache.unconfirmed.has(agentId);
}

/** A new agent, connected by the request answered as `at`. */
export function withCreated(cache: AgentCache, agent: PersonalAgent, at: number): AgentCache {
  if (cache.list.agents.some((a) => a.id === agent.id)) return cache;

  const applied = new Map(cache.applied);
  const tokensAt = new Map(cache.tokensAt);

  applied.set(agent.id, at);

  for (const token of agent.credentials) tokensAt.set(token.id, at);

  return {
    ...cache,
    list: { ...cache.list, agents: [...cache.list.agents, agent] },
    applied,
    tokensAt,
  };
}

/** Agent `agentId` gone, by request `at`; an older load won't bring it back. */
export function withRemoved(cache: AgentCache, agentId: number, at: number): AgentCache {
  const applied = new Map(cache.applied);
  const unconfirmed = new Map(cache.unconfirmed);

  applied.set(agentId, Math.max(profileAt(cache, agentId), at));
  unconfirmed.delete(agentId);

  return {
    ...cache,
    list: { ...cache.list, agents: cache.list.agents.filter((a) => a.id !== agentId) },
    applied,
    unconfirmed,
  };
}

/**
 * Agent `agentId`'s tokens as the answer to request `at` gives them: an issued token, a rotated
 * one with the one it replaced, or a revocation's whole list. `at` is `null` when they carry no
 * details newer than the cache's (a rotation's replaced token is the page's own copy, marked
 * revoked): then they only move tokens along and add new ones.
 */
export function withTokens(
  cache: AgentCache,
  agentId: number,
  incoming: readonly PersonalAgentCredential[],
  at: number | null,
): AgentCache {
  const held = cache.list.agents.find((a) => a.id === agentId);

  if (held === undefined) return cache;

  const tokens = mergedTokens(held.credentials, incoming, at, cache.tokensAt);

  return {
    ...cache,
    list: {
      ...cache.list,
      agents: cache.list.agents.map((a) =>
        a.id === agentId ? { ...a, credentials: tokens.credentials } : a,
      ),
    },
    tokensAt: tokens.tokensAt,
  };
}
