import { useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useRef, useState } from "react";
import type { Pin } from "../../gen/Pin.ts";
import type { PinList } from "../../gen/PinList.ts";
import { formatFull } from "../../lib/time.ts";
import type { MessageDTO } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { SkeletonReveal } from "../../ui/skeleton.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Tooltip } from "../../ui/tooltip.tsx";
import { BodyHtml } from "../messages/body-html.tsx";
import { UNKNOWN_NAME, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { timeAgo } from "../threads/thread-format.ts";
import { useNow } from "../threads/use-now.ts";
import { PaneFrame, RoomName } from "./pane-frame.tsx";
import { PaneEmpty, PaneError, PaneListSkeleton } from "./pane-states.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly list: PinList };

export interface PinnedEntry {
  readonly pin: Pin;
  readonly message: MessageDTO;
}

/**
 * The pins in the list's order (newest first) with their messages, minus any unpinned since: by
 * this pane (`removed`) or anywhere else (the store's copy of the message says so).
 */
export function pinnedEntries(
  list: PinList,
  removed: ReadonlySet<number>,
  held: Readonly<Record<number, MessageDTO>>,
): PinnedEntry[] {
  const byId = new Map(list.messages.map((message) => [message.id, message]));

  return list.pins.flatMap((pin): PinnedEntry[] => {
    const message = byId.get(pin.messageId);

    if (
      message === undefined ||
      removed.has(pin.messageId) ||
      held[pin.messageId]?.pinned === false
    ) {
      return [];
    }

    return [{ pin, message }];
  });
}

interface PinCardProps {
  readonly entry: PinnedEntry;
  readonly now: number;
  readonly onJump: (message: MessageDTO) => void;
  readonly onUnpin: (message: MessageDTO) => void;
}

function PinCard({ entry, now, onJump, onUnpin }: PinCardProps) {
  const { pin, message } = entry;
  const author = useUser(message.creatorId);
  const pinner = useUser(pin.pinnerId);

  return (
    <li className="pin-card enter-fade">
      <header className="pin-card-header">
        <UserAvatar userId={message.creatorId} size={24} decorative />
        <span className="pin-card-author">{author?.name ?? UNKNOWN_NAME}</span>
        <Tooltip content={formatFull(message.createdAt)} describe={false}>
          <time className="pin-card-time" dateTime={message.createdAt} tabIndex={-1}>
            {timeAgo(message.createdAt, now)}
          </time>
        </Tooltip>
        <span className="pin-card-actions">
          <IconButton
            icon="arrow-up-right"
            size="sm"
            label={message.threadId === null ? "Jump to message" : "Open in thread"}
            onClick={() => onJump(message)}
          />
          <IconButton icon="pin" size="sm" label="Unpin" onClick={() => onUnpin(message)} />
        </span>
      </header>
      <BodyHtml html={message.bodyHtml} className="pin-card-body message-body" />
      {message.attachment === null ? null : (
        <p className="pin-card-file">{message.attachment.filename}</p>
      )}
      <footer className="pin-card-footer">
        Pinned by {pinner?.name ?? UNKNOWN_NAME} · {timeAgo(pin.pinnedAt, now)}
      </footer>
    </li>
  );
}

/**
 * The Pins pane: the room's pinned messages, newest pin first, as compact cards with who pinned
 * them. Jump to one (its thread, for a reply) or unpin it; the list reloads when the room's pin
 * count moves (`message.pinned` from anyone).
 */
export function PinsPane({ roomId }: { readonly roomId: number }) {
  const navigate = useNavigate();
  const now = useNow();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [removed, setRemoved] = useState<ReadonlySet<number>>(() => new Set());
  const held = useStore((state) => state.messages);
  const pinsCount = useStore((state) => state.rooms[roomId]?.detail?.pinsCount ?? null);
  const loadedOnce = useRef(false);

  const reload = useCallback(
    (quiet: boolean) => {
      if (!quiet) {
        setLoad({ status: "loading" });
      }

      actions.messages.pins(roomId).then(
        (list) => {
          loadedOnce.current = true;
          setRemoved(new Set());
          setLoad({ status: "ready", list });
        },
        (error: Error) => {
          if (!quiet) {
            setLoad({ status: "error", message: error.message });
          }
        },
      );
    },
    [roomId],
  );

  // First load, then a quiet refresh whenever the room's pin count changes.
  // biome-ignore lint/correctness/useExhaustiveDependencies: pinsCount is the trigger, not an input
  useEffect(() => {
    reload(loadedOnce.current);
  }, [reload, pinsCount]);

  const unpin = (message: MessageDTO) => {
    setRemoved((ids) => new Set([...ids, message.id]));
    actions.messages.setPinned(message.id, false).catch((error: Error) => {
      setRemoved((ids) => new Set([...ids].filter((id) => id !== message.id)));
      toast({ title: "Couldn't unpin the message", description: error.message, tone: "danger" });
    });
  };

  const jump = (message: MessageDTO) => {
    if (message.threadId === null) {
      void navigate({ to: "/r/$roomId/m/$messageId", params: { roomId, messageId: message.id } });
    } else {
      void navigate({
        to: "/r/$roomId/t/$threadId",
        params: { roomId, threadId: message.threadId },
      });
    }
  };

  const entries = load.status === "ready" ? pinnedEntries(load.list, removed, held) : [];

  return (
    <PaneFrame title="Pinned messages" subtitle={<RoomName roomId={roomId} />}>
      {load.status === "error" ? (
        <PaneError
          message="The pinned messages couldn't be loaded."
          onRetry={() => reload(false)}
        />
      ) : (
        <SkeletonReveal
          loading={load.status === "loading"}
          skeleton={<PaneListSkeleton rows={4} square={24} />}
        >
          {load.status === "ready" && entries.length === 0 ? (
            <PaneEmpty
              icon="pin"
              title="No pinned messages"
              text="Pin important messages from their menu so everyone can find them here."
            />
          ) : (
            <ul className="pin-list">
              {entries.map((entry) => (
                <PinCard
                  key={entry.pin.messageId}
                  entry={entry}
                  now={now}
                  onJump={jump}
                  onUnpin={unpin}
                />
              ))}
            </ul>
          )}
        </SkeletonReveal>
      )}
    </PaneFrame>
  );
}
