import type { Presence, User } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import type { PresenceStatus } from "../../ui/avatar.tsx";

const AVATAR_STATUS = {
  online: "online",
  idle: "away",
  dnd: "dnd",
  offline: "offline",
} as const satisfies Record<Presence, PresenceStatus>;

/** Bots and agents get the violet identity: bot-avatars tiles and the AGENT tag. */
export function isAgent(user: User | undefined): boolean {
  return user?.role === "bot";
}

/** A person's presence dot, or `undefined` while unknown (no dot rather than a wrong one). */
export function usePresenceStatus(userId: number | undefined): PresenceStatus | undefined {
  return useStore((state) => {
    const entry = userId === undefined ? undefined : state.presence[userId];

    return entry === undefined ? undefined : AVATAR_STATUS[entry.presence];
  });
}

export function useUser(userId: number | undefined): User | undefined {
  return useStore((state) => (userId === undefined ? undefined : state.users[userId]));
}

/** The name to show for someone whose profile hasn't arrived yet. */
export const UNKNOWN_NAME = "Someone";
