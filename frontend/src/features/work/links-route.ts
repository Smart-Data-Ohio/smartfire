/**
 * The links editor follows `/app/r/$roomId/t/$threadId/links` (a board's query stays put).
 * An in-app open (the work bar, a board post) pushes that URL with {@link pushedOverState}, and
 * closing steps back, so the thread entry underneath is the one that was already there. A direct
 * arrival (a classic links URL, a paste) has no such flag, and closing replaces this entry with
 * the thread. Cancel and a successful link both close this way.
 */
import { useMatchRoute, useNavigate } from "@tanstack/react-router";
import { parseBoardSearch } from "../../lib/board-search.ts";
import { useStore } from "../../store/store.ts";
import { pushedOverState, useClosePushedOver } from "./pushed-over.ts";

export function useLinksRoute(threadId: number, roomIdOverride?: number | null) {
  const navigate = useNavigate();
  const matchRoute = useMatchRoute();
  const closeOver = useClosePushedOver();
  const storedRoomId = useStore((state) => state.threads[threadId]?.roomId ?? null);
  const roomId = roomIdOverride === undefined ? storedRoomId : roomIdOverride;
  const open = matchRoute({ to: "/r/$roomId/t/$threadId/links", includeSearch: false }) !== false;

  const replaceWithThread = () => {
    if (roomId === null) {
      return;
    }

    void navigate({
      to: "/r/$roomId/t/$threadId",
      params: { roomId, threadId },
      search: parseBoardSearch,
      replace: true,
    });
  };

  return {
    open,
    roomId,
    openLinks: () => {
      if (roomId === null) {
        return;
      }

      void navigate({
        to: "/r/$roomId/t/$threadId/links",
        params: { roomId, threadId },
        search: parseBoardSearch,
        state: pushedOverState(),
      });
    },
    closeLinks: () => closeOver(replaceWithThread),
  };
}
