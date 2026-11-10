/**
 * Settings → Agents' copy of what the server holds.
 *
 * An agent's profile and access: latest sent wins. Every rename, access save, removal and reload
 * of the whole list takes a number from `nextSequence()` when it is sent, and an answer lands only
 * where no answer to a later-sent request has landed. So a slow reload never puts back an older
 * name, and an older reload can't bring back a removed agent (it keeps its number). An older
 * reload's tokens are still merged by state, below.
 *
 * Its tokens and connections: their state only ever moves forward, from active to expired to
 * revoked, and they are never deleted. So every answer's state is merged in by that, not by when it
 * was sent (the server may carry out two requests in the other order): a token any answer showed
 * revoked stays revoked, one an answer doesn't mention stays as it is, and a new one is added.
 * What else can change about one (when it was last used, whether an administrator blocked its app,
 * why it was revoked) goes back and forth, so for that the latest sent wins, as for profiles: a
 * slow answer can't unblock a connection a later one showed blocked. Why it was revoked sticks
 * like the state: once any answer says, no answer without a reason clears it.
 *
 * Whether it's paused: only pausing and resuming change that. A pause or resume answer sets only
 * that, the latest sent winning among them. Any other answer (a reload, a rename, an access save)
 * speaks for it only if it was sent while no pause or resume for that agent was out, and after the
 * last one was answered or failed; otherwise the held pause state stays. So no answer that may
 * predate a pause can undo it, whichever order they come back in.
 *
 * And which agents a save went unanswered for: it may or may not have landed, so the cache can't
 * tell what the server holds for them until an answer that speaks for the whole agent (its
 * profile and its pause state), sent after that save, says. A pause or resume answer never does.
 */
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";

export interface AgentCache {
  readonly list: PersonalAgentList;
  /** The newest request applied to each agent's profile and access, removed agents included. */
  readonly applied: ReadonlyMap<number, number>;
  /** The newest pause or resume applied to each agent's pause state. */
  readonly pausing: ReadonlyMap<number, number>;
  /** How many pauses and resumes are out for each agent. */
  readonly pausesOut: ReadonlyMap<number, number>;
  /** When each agent's last pause or resume came back, as a number taken then. */
  readonly pauseSettled: ReadonlyMap<number, number>;
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

/** Whether the answer to request `at`, other than a pause or resume, speaks for the pause state. */
function speaksForPause(cache: AgentCache, agentId: number, at: number): boolean {
  return (cache.pausesOut.get(agentId) ?? 0) === 0 && at > (cache.pauseSettled.get(agentId) ?? 0);
}

/** `agent` with `from`'s pause state. */
function pausedAs(agent: PersonalAgent, from: PersonalAgent): PersonalAgent {
  return { ...agent, pausedAt: from.pausedAt, pausedBy: from.pausedBy };
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

  // Why it was revoked sticks like the state: once any answer gave a reason, it stays, whatever
  // order the answers come in (one sent before the server knew it may have none).
  const reason = held.connection?.revokedReason ?? fresh.connection?.revokedReason ?? null;

  const connection =
    details.connection === null
      ? null
      : { ...details.connection, revokedReason: state === "revoked" ? reason : null };

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

/**
 * `cache.unconfirmed` without agent `agentId`, if the answer to request `at` speaks for the whole
 * agent and was sent after its unanswered save.
 */
function confirmed(cache: AgentCache, agentId: number, at: number): ReadonlyMap<number, number> {
  const unconfirmed = cache.unconfirmed;
  const since = unconfirmed.get(agentId);

  if (since === undefined || at <= since || !speaksForPause(cache, agentId, at)) {
    return unconfirmed;
  }

  const next = new Map(unconfirmed);

  next.delete(agentId);

  return next;
}

/** `cache` with only the states and revocation reasons of an older load's tokens merged in. */
function withOlderTokens(cache: AgentCache, older: PersonalAgentList): AgentCache {
  let tokensAt = cache.tokensAt;

  const agents = cache.list.agents.map((agent) => {
    const listed = older.agents.find((a) => a.id === agent.id);

    if (listed === undefined) return agent;

    const tokens = mergedTokens(agent.credentials, listed.credentials, null, tokensAt);

    tokensAt = tokens.tokensAt;

    return { ...agent, credentials: tokens.credentials };
  });

  return { ...cache, list: { ...cache.list, agents }, tokensAt };
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

    return {
      list: fresh,
      applied: new Map(),
      pausing: new Map(),
      pausesOut: new Map(),
      pauseSettled: new Map(),
      listSequence: at,
      unconfirmed: new Map(),
      tokensAt,
    };
  }

  // An older load says nothing new about profiles or the agents listed, but its tokens' states
  // and revocation reasons still only move forward, so those are merged in all the same.
  if (at <= cache.listSequence) return withOlderTokens(cache, fresh);

  const listed = new Set(fresh.agents.map((agent) => agent.id));
  let tokensAt = cache.tokensAt;

  const agents = fresh.agents.flatMap((agent) => {
    const held = cache.list.agents.find((a) => a.id === agent.id);
    const profileNewer = (cache.applied.get(agent.id) ?? 0) > at;

    // Removed after this load was sent.
    if (held === undefined) return profileNewer ? [] : [agent];

    const tokens = mergedTokens(held.credentials, agent.credentials, at, tokensAt);

    tokensAt = tokens.tokensAt;

    const profile = profileNewer ? held : agent;
    const pause = !profileNewer && speaksForPause(cache, agent.id, at) ? agent : held;

    return [{ ...pausedAs(profile, pause), credentials: tokens.credentials }];
  });

  // Connected after this load was sent, so it couldn't list them.
  const later = cache.list.agents.filter(
    (agent) => !listed.has(agent.id) && (cache.applied.get(agent.id) ?? 0) > at,
  );

  const unconfirmed = [...cache.unconfirmed.keys()].reduce(
    (current, agentId) => confirmed({ ...cache, unconfirmed: current }, agentId, at),
    cache.unconfirmed,
  );

  return {
    ...cache,
    list: { ...fresh, agents: [...agents, ...later] },
    listSequence: at,
    unconfirmed,
    tokensAt,
  };
}

/**
 * An agent as the answer to request `at` (a rename or an access save) gives it; its pause state
 * only if that answer speaks for it.
 */
export function withAgent(cache: AgentCache, agent: PersonalAgent, at: number): AgentCache {
  const held = cache.list.agents.find((a) => a.id === agent.id);

  if (held === undefined) return cache;

  const newer = at > profileAt(cache, agent.id);
  const applied = new Map(cache.applied);

  if (newer) applied.set(agent.id, at);

  const pause = newer && speaksForPause(cache, agent.id, at) ? agent : held;
  const next = pausedAs(newer ? agent : held, pause);

  return {
    ...withAnswer({ ...cache, applied }, held, next, agent, at),
    unconfirmed: confirmed(cache, agent.id, at),
  };
}

/** A pause or resume for agent `agentId` sent: until it's back, other answers keep its pause state. */
export function withPauseSent(cache: AgentCache, agentId: number): AgentCache {
  const pausesOut = new Map(cache.pausesOut);

  pausesOut.set(agentId, (pausesOut.get(agentId) ?? 0) + 1);

  return { ...cache, pausesOut };
}

/**
 * The pause or resume sent as request `at` back, as of `settled` (a number taken when it came
 * back): its answer's pause state, or `null` when it failed, whether or not it landed.
 */
export function withPauseSettled(
  cache: AgentCache,
  agentId: number,
  answer: PersonalAgent | null,
  at: number,
  settled: number,
): AgentCache {
  const pausesOut = new Map(cache.pausesOut);
  const pauseSettled = new Map(cache.pauseSettled);

  pausesOut.set(agentId, Math.max((pausesOut.get(agentId) ?? 0) - 1, 0));
  pauseSettled.set(agentId, settled);

  const back = { ...cache, pausesOut, pauseSettled };
  const held = cache.list.agents.find((a) => a.id === agentId);

  if (answer === null || held === undefined) return back;

  const newer = at > (cache.pausing.get(agentId) ?? 0);
  const pausing = new Map(cache.pausing);

  if (newer) pausing.set(agentId, at);

  return withAnswer({ ...back, pausing }, held, newer ? pausedAs(held, answer) : held, answer, at);
}

/** `cache` with `held` replaced by `next` and the answer's tokens merged in. */
function withAnswer(
  cache: AgentCache,
  held: PersonalAgent,
  next: PersonalAgent,
  answer: PersonalAgent,
  at: number,
): AgentCache {
  const tokens = mergedTokens(held.credentials, answer.credentials, at, cache.tokensAt);
  const merged = { ...next, credentials: tokens.credentials };

  return {
    ...cache,
    list: {
      ...cache.list,
      agents: cache.list.agents.map((a) => (a.id === held.id ? merged : a)),
    },
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
