/** The Members pane's grouping: Starred, then Online, then Offline, as the classic panel has it. */
import type { Member } from "../../gen/Member.ts";
import type { Presence, User, UserPresence } from "../../store/model.ts";

export interface MemberEntry {
  readonly userId: number;
  readonly name: string;
  readonly presence: Presence;
  readonly statusText: string | null;
  readonly starred: boolean;
  readonly bot: boolean;
  readonly viewer: boolean;
}

export interface MemberSection {
  readonly key: "starred" | "online" | "offline";
  readonly label: string;
  readonly members: readonly MemberEntry[];
}

export interface MemberGroupingInput {
  readonly members: readonly Member[];
  readonly users: Readonly<Record<number, User>>;
  /** Live presence from the store; it wins over the list's snapshot. */
  readonly presence: Readonly<Record<number, UserPresence>>;
  /** Local star flips, ahead of the list's flags. */
  readonly stars: Readonly<Record<number, boolean>>;
  readonly query: string;
  readonly viewerId: number | null;
}

/** Folds case and accents, so "zoe" finds "Zoë". */
export function foldName(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .trim();
}

function entry(member: Member, input: MemberGroupingInput): MemberEntry {
  const user = input.users[member.userId];
  const live = input.presence[member.userId];

  return {
    userId: member.userId,
    name: user?.name ?? "Someone",
    presence: live?.presence ?? member.presence,
    statusText: live?.statusText ?? member.statusText,
    starred: input.stars[member.userId] ?? member.starred,
    bot: user?.role === "bot",
    viewer: member.userId === input.viewerId,
  };
}

/**
 * Members matching the search (anywhere in the name, case- and accent-blind), in the server's
 * name order, in sections: Starred (if any; starred people appear only there), Online (anything
 * but offline) and Offline. Empty sections are left out.
 */
export function groupMembers(input: MemberGroupingInput): MemberSection[] {
  const query = foldName(input.query);

  const entries = input.members
    .map((member) => entry(member, input))
    .filter((member) => query === "" || foldName(member.name).includes(query));

  const starred = entries.filter((member) => member.starred);
  const rest = entries.filter((member) => !member.starred);

  const sections: MemberSection[] = [
    { key: "starred", label: "Starred", members: starred },
    {
      key: "online",
      label: "Online",
      members: rest.filter((member) => member.presence !== "offline"),
    },
    {
      key: "offline",
      label: "Offline",
      members: rest.filter((member) => member.presence === "offline"),
    },
  ];

  return sections.filter((section) => section.members.length > 0);
}

/** Only active humans other than the viewer can be starred. */
export function canStar(member: MemberEntry): boolean {
  return !member.bot && !member.viewer;
}
