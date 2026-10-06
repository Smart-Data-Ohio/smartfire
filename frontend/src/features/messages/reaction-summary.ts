/** How many names a reaction tooltip lists before "and N others". */
export const NAMED_REACTORS = 6;

/** "A", "A and B", "A, B and C". */
function joinNames(names: readonly string[]): string {
  if (names.length <= 1) {
    return names[0] ?? "";
  }

  return `${names.slice(0, -1).join(", ")} and ${names.at(-1)}`;
}

/**
 * The reaction tooltip, Slack's way: "You, Maya Okafor and 2 others reacted with Thumbs up". The
 * viewer comes first as "You"; unknown people (not loaded yet) count among the others.
 */
export function reactorSummary(
  reactorIds: readonly number[],
  viewerId: number | null,
  nameOf: (id: number) => string | undefined,
  title: string,
): string {
  const you = viewerId !== null && reactorIds.includes(viewerId);
  const others = reactorIds.filter((id) => id !== viewerId);

  const known = others.flatMap((id) => {
    const name = nameOf(id);

    return name === undefined ? [] : [name];
  });

  const named = (you ? ["You", ...known] : known).slice(0, NAMED_REACTORS);
  const rest = reactorIds.length - named.length;

  if (rest === 0) {
    return `${joinNames(named)} reacted with ${title}`;
  }

  if (named.length === 0) {
    return `${rest} ${rest === 1 ? "person" : "people"} reacted with ${title}`;
  }

  return `${named.join(", ")} and ${rest} ${rest === 1 ? "other" : "others"} reacted with ${title}`;
}

/** The pill's accessible name: "Thumbs up: 3 reactions, including yours". */
export function reactionLabel(title: string, count: number, mine: boolean): string {
  const reactions = `${count} ${count === 1 ? "reaction" : "reactions"}`;

  return `${title}: ${reactions}${mine ? ", including yours" : ""}`;
}
