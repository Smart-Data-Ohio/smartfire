import { Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { directs } from "../../sync/directs.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { toast } from "../../ui/toast-store.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { ROOM_KIND_ICON } from "../room/room-icon.ts";
import { matchScore, normalizeQuery } from "../switcher/match.ts";
import type { SwitcherItem } from "../switcher/ranking.ts";
import { useSuggestible } from "./use-suggestible.ts";

const SHOWN = 4;

/** The best few items of a kind whose names match `needle` closely (no fuzzy scatter). */
function closest(
  items: readonly SwitcherItem[],
  needle: string,
  keep: (item: SwitcherItem) => boolean,
): SwitcherItem[] {
  return items
    .flatMap((item) => {
      const score = keep(item) ? matchScore(item.label, needle) : null;

      // 400 and up: the words appear in the name as typed, not as a scattered subsequence.
      return score === null || score < 400 ? [] : [{ item, score }];
    })
    .sort((left, right) => right.score - left.score)
    .slice(0, SHOWN)
    .map((entry) => entry.item);
}

/** A person you can message: their DM, opened (or made) on click. */
function PersonMatch({ item }: { readonly item: SwitcherItem }) {
  const navigate = useNavigate();
  const userId = item.userId ?? 0;
  const [creating, setCreating] = useState(false);

  const open = () => {
    if (item.roomId !== null) {
      void navigate({ to: "/r/$roomId", params: { roomId: item.roomId } });

      return;
    }

    // One create at a time: a second click while it's on its way would make another request.
    setCreating(true);
    directs.create([userId]).then(
      (row) => {
        setCreating(false);
        void navigate({ to: "/r/$roomId", params: { roomId: row.room.id } });
      },
      (error: Error) => {
        setCreating(false);
        toast({
          title: "Couldn't open the conversation",
          description: error.message,
          tone: "danger",
        });
      },
    );
  };

  return (
    <Button
      variant="ghost"
      className="search-match"
      data-search-nav
      loading={creating}
      onClick={open}
    >
      <UserAvatar userId={userId} size={24} presence decorative />
      <span className="search-match-label">{item.label}</span>
    </Button>
  );
}

/**
 * People and channels whose names match the searched words, matched on the client from the
 * switcher's data (the search reply carries messages and sections only).
 */
export function SearchMatches({ words }: { readonly words: readonly string[] }) {
  const items = useSuggestible(true);
  const needle = normalizeQuery(words.join(" "));

  if (needle === "") {
    return null;
  }

  const people = closest(items, needle, (item) => item.kind === "person" && item.userId !== null);

  const channels = closest(
    items,
    needle,
    (item) => item.kind === "room" && item.roomKind !== null && item.roomKind !== "direct",
  );

  if (people.length === 0 && channels.length === 0) {
    return null;
  }

  return (
    <div className="search-matches">
      {people.length === 0 ? null : (
        <section className="search-matches-group" aria-label="People">
          <h3 className="search-section-heading">People</h3>
          <div className="search-matches-row">
            {people.map((item) => (
              <PersonMatch key={item.key} item={item} />
            ))}
          </div>
        </section>
      )}
      {channels.length === 0 ? null : (
        <section className="search-matches-group" aria-label="Channels">
          <h3 className="search-section-heading">Channels</h3>
          <div className="search-matches-row">
            {channels.map((item) =>
              item.roomId === null ? null : (
                <Link
                  key={item.key}
                  to="/r/$roomId"
                  params={{ roomId: item.roomId }}
                  className="search-match"
                  data-search-nav
                >
                  <Icon name={ROOM_KIND_ICON[item.roomKind ?? "open"]} size={16} />
                  <span className="search-match-label">{item.label}</span>
                </Link>
              ),
            )}
          </div>
        </section>
      )}
    </div>
  );
}
