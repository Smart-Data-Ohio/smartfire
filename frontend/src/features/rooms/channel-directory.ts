import { useEffect, useRef, useState } from "react";
import type { DirectoryChannel } from "../../gen/DirectoryChannel.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { toast } from "../../ui/toast-store.ts";

/** How long typing has to pause before the directory asks the server. */
const SEARCH_DELAY_MS = 150;

/** The open rooms answering one query, page by page. */
export interface ChannelDirectoryState {
  /** The query `channels` answer; it trails the typed one while a search is on its way. */
  readonly query: string;
  readonly channels: readonly DirectoryChannel[];
  readonly nextCursor: string | null;
  readonly status: "loading" | "ready" | "failed";
  readonly loadingMore: boolean;
}

const LOADING: ChannelDirectoryState = {
  query: "",
  channels: [],
  nextCursor: null,
  status: "loading",
  loadingMore: false,
};

/**
 * `GET /rooms/browse` for `query` while `active`, a short pause after the last keystroke. An
 * answer to an older query never lands over a newer one. `loadMore` appends the next page.
 */
export function useChannelDirectory(
  active: boolean,
  query: string,
): ChannelDirectoryState & { readonly loadMore: () => void } {
  const [state, setState] = useState<ChannelDirectoryState>(LOADING);
  const latest = useRef(0);
  const needle = query.trim();

  useEffect(() => {
    if (!active) {
      return;
    }

    const request = ++latest.current;

    const timer = window.setTimeout(
      () => {
        actions.browseChannels(needle, null).then(
          (page) => {
            if (latest.current === request) {
              setState({
                query: needle,
                channels: page.channels,
                nextCursor: page.nextCursor,
                status: "ready",
                loadingMore: false,
              });
            }
          },
          () => {
            if (latest.current === request) {
              setState((held) => ({ ...held, status: "failed", loadingMore: false }));
            }
          },
        );
      },
      needle === "" ? 0 : SEARCH_DELAY_MS,
    );

    return () => window.clearTimeout(timer);
  }, [active, needle]);

  const loadMore = () => {
    const { nextCursor, loadingMore, status } = state;

    if (nextCursor === null || loadingMore || status !== "ready") {
      return;
    }

    const request = latest.current;

    setState((held) => ({ ...held, loadingMore: true }));
    actions.browseChannels(state.query, nextCursor).then(
      (page) => {
        if (latest.current === request) {
          setState((held) => ({
            ...held,
            channels: [
              ...held.channels,
              ...page.channels.filter(
                (channel) => !held.channels.some((shown) => shown.roomId === channel.roomId),
              ),
            ],
            nextCursor: page.nextCursor,
            loadingMore: false,
          }));
        }
      },
      () => {
        if (latest.current === request) {
          setState((held) => ({ ...held, loadingMore: false }));
        }
      },
    );
  };

  return { ...state, loadMore };
}

/**
 * Whether the viewer is in each channel: a join on its way counts at once, then the live sidebar
 * decides (a hidden row is not joined), and the server's flag covers rooms the sidebar lacks.
 */
export function useChannelJoins() {
  const [pending, setPending] = useState<ReadonlySet<number>>(new Set());
  const rows = useStore((state) => state.sidebar.rows);

  const joined = (channel: DirectoryChannel) => {
    if (pending.has(channel.roomId)) {
      return true;
    }

    const row = rows[channel.roomId];

    return row === undefined ? channel.joined : row.membership.involvement !== "invisible";
  };

  const settle = (roomId: number) =>
    setPending((held) => new Set([...held].filter((id) => id !== roomId)));

  const join = (channel: DirectoryChannel) => {
    if (joined(channel)) {
      return;
    }

    setPending((held) => new Set([...held, channel.roomId]));
    actions.joinOpenRoom(channel.roomId).then(
      () => settle(channel.roomId),
      (failure: Error) => {
        settle(channel.roomId);
        toast({
          title: `Couldn't join #${channel.name}`,
          description: failure.message,
          tone: "danger",
        });
      },
    );
  };

  return { joined, join };
}

/** "1 member", "12 members". */
export function memberCountLabel(count: number): string {
  return count === 1 ? "1 member" : `${count.toLocaleString()} members`;
}
