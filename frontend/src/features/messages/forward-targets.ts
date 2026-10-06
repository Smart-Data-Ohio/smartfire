import type { ForwardDestination } from "../../gen/ForwardDestination.ts";
import type { ForwardTarget } from "../../gen/ForwardTarget.ts";

/** A forward goes to 1 to 5 places (`message_forwards#create`). */
export const MAX_FORWARDS = 5;

/** A stable key for a destination: `12` for a room, `12:7` for a thread in it. */
export function targetKey(target: ForwardTarget): string {
  return target.threadId === null ? `${target.roomId}` : `${target.roomId}:${target.threadId}`;
}

/**
 * The destinations matching `query` (case-insensitive, on the room's or a thread's name). A room
 * that matches keeps all its threads; otherwise only its matching threads show, under it.
 */
export function filterDestinations(
  destinations: readonly ForwardDestination[],
  query: string,
): ForwardDestination[] {
  const needle = query.trim().toLowerCase();

  if (needle === "") {
    return [...destinations];
  }

  return destinations.flatMap((destination) => {
    if (destination.name.toLowerCase().includes(needle)) {
      return [destination];
    }

    const threads = destination.threads.filter((thread) =>
      thread.name.toLowerCase().includes(needle),
    );

    return threads.length === 0 ? [] : [{ ...destination, threads }];
  });
}
