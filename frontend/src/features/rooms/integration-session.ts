/**
 * GitHub subscriptions and inbound email, kept for the life of the page rather than the settings
 * tab that shows them. The tab panel remounts when the viewer leaves and comes back, so a read
 * started by that remount can finish before a write that started on the previous panel, and the
 * write's completion used to update only the unmounted copy. The session takes the write: a read
 * that started at an older revision is dropped, and whichever panel is showing renders this.
 */
import { useSyncExternalStore } from "react";
import type { GithubSubscriptionList } from "../../gen/GithubSubscriptionList.ts";
import type { InboundEmail } from "../../gen/InboundEmail.ts";

export type GithubWrite = "subscribe" | "save" | "remove";

export interface GithubSession {
  readonly list: GithubSubscriptionList | null;
  readonly revision: number;
  readonly write: GithubWrite | null;
}

export interface EmailSession {
  readonly email: InboundEmail | null;
  readonly revision: number;
  readonly pending: boolean;
}

const EMPTY_GITHUB: GithubSession = { list: null, revision: 0, write: null };

const EMPTY_EMAIL: EmailSession = { email: null, revision: 0, pending: false };

const githubSessions = new Map<number, GithubSession>();

const emailSessions = new Map<number, EmailSession>();

const githubListeners = new Map<number, Set<() => void>>();

const emailListeners = new Map<number, Set<() => void>>();

function emit(listeners: Map<number, Set<() => void>>, roomId: number): void {
  const set = listeners.get(roomId);

  if (set === undefined) return;

  for (const listener of set) listener();
}

function listen(
  listeners: Map<number, Set<() => void>>,
  roomId: number,
  listener: () => void,
): () => void {
  const existing = listeners.get(roomId);
  const set = existing ?? new Set<() => void>();

  if (existing === undefined) listeners.set(roomId, set);

  set.add(listener);

  return () => {
    set.delete(listener);
  };
}

export function readGithubSession(roomId: number): GithubSession {
  return githubSessions.get(roomId) ?? EMPTY_GITHUB;
}

function putGithub(roomId: number, session: GithubSession): void {
  githubSessions.set(roomId, session);
  emit(githubListeners, roomId);
}

/** A read applies only when no write has landed since it started. */
export function noteGithubLoad(
  roomId: number,
  list: GithubSubscriptionList,
  seenRevision: number,
): boolean {
  const current = readGithubSession(roomId);

  if (current.revision !== seenRevision) return false;

  putGithub(roomId, { ...current, list });

  return true;
}

export function beginGithubWrite(roomId: number, write: GithubWrite): void {
  putGithub(roomId, { ...readGithubSession(roomId), write });
}

export function finishGithubWrite(roomId: number, list: GithubSubscriptionList): void {
  const current = readGithubSession(roomId);

  putGithub(roomId, { list, revision: current.revision + 1, write: null });
}

export function cancelGithubWrite(roomId: number): void {
  const current = readGithubSession(roomId);

  if (current.write === null) return;

  putGithub(roomId, { ...current, write: null });
}

export function useGithubSession(roomId: number): GithubSession {
  return useSyncExternalStore(
    (listener) => listen(githubListeners, roomId, listener),
    () => readGithubSession(roomId),
    () => EMPTY_GITHUB,
  );
}

export function readEmailSession(roomId: number): EmailSession {
  return emailSessions.get(roomId) ?? EMPTY_EMAIL;
}

function putEmail(roomId: number, session: EmailSession): void {
  emailSessions.set(roomId, session);
  emit(emailListeners, roomId);
}

export function noteEmailLoad(roomId: number, email: InboundEmail, seenRevision: number): boolean {
  const current = readEmailSession(roomId);

  if (current.revision !== seenRevision) return false;

  putEmail(roomId, { ...current, email });

  return true;
}

export function beginEmailWrite(roomId: number): void {
  putEmail(roomId, { ...readEmailSession(roomId), pending: true });
}

export function finishEmailWrite(roomId: number, email: InboundEmail): void {
  const current = readEmailSession(roomId);

  putEmail(roomId, { email, revision: current.revision + 1, pending: false });
}

export function cancelEmailWrite(roomId: number): void {
  const current = readEmailSession(roomId);

  if (!current.pending) return;

  putEmail(roomId, { ...current, pending: false });
}

export function useEmailSession(roomId: number): EmailSession {
  return useSyncExternalStore(
    (listener) => listen(emailListeners, roomId, listener),
    () => readEmailSession(roomId),
    () => EMPTY_EMAIL,
  );
}

/** Tests start from an empty page. */
export function resetIntegrationSessions(): void {
  const githubRooms = [...githubListeners.keys()];
  const emailRooms = [...emailListeners.keys()];

  githubSessions.clear();
  emailSessions.clear();

  for (const roomId of githubRooms) emit(githubListeners, roomId);

  for (const roomId of emailRooms) emit(emailListeners, roomId);
}
