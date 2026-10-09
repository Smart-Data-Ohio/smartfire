/**
 * The links editor follows `/app/r/$roomId/t/$threadId/links` (a board's query stays put).
 * Opening and closing replace the entry, the same way a board post's link form does, so a
 * classic links URL and a pasted room URL don't pile history under the editor.
 */
import { useMatchRoute, useNavigate } from "@tanstack/react-router";
import { parseBoardSearch } from "../../lib/board-search.ts";
import { useStore } from "../../store/store.ts";

export function useLinksRoute(threadId: number) {
  const navigate = useNavigate();
  const matchRoute = useMatchRoute();
  const roomId = useStore((state) => state.threads[threadId]?.roomId ?? null);
  const open = matchRoute({ to: "/r/$roomId/t/$threadId/links", includeSearch: false }) !== false;

  const go = (toLinks: boolean) => {
    if (roomId === null) {
      return;
    }

    void navigate({
      to: toLinks ? "/r/$roomId/t/$threadId/links" : "/r/$roomId/t/$threadId",
      params: { roomId, threadId },
      search: parseBoardSearch,
      replace: true,
    });
  };

  return {
    open,
    roomId,
    openLinks: () => go(true),
    closeLinks: () => go(false),
  };
}
