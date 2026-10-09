import { useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { uuid7 } from "../../lib/uuid7.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Composer, type ComposerDraft } from "../composer/composer.tsx";
import { newThreadDraftKey } from "../composer/draft.ts";
import { PaneFrame } from "../panes/pane-frame.tsx";
import { PaneEmpty } from "../panes/pane-states.tsx";
import { MessageRow } from "../room/message-row.tsx";
import { ThreadSkeleton } from "./thread-timeline.tsx";

/** A thread's default name, as the server makes it: the parent's first line, up to 100 characters. */
export function defaultThreadName(markdown: string | null, bodyHtml: string): string {
  const source =
    markdown ?? new DOMParser().parseFromString(bodyHtml, "text/html").body.textContent ?? "";

  const line = source
    .split("\n")
    .map((part) => part.trim())
    .find((part) => part !== "");

  return line === undefined ? "New thread" : line.slice(0, 100);
}

/**
 * `/r/$roomId/t/new?parent=`: the root message on top, an optional name, and the composer whose
 * first reply starts the thread; the pane then moves to the new thread (replacing the draft in
 * history). If the message already has a thread (someone beat us to it), that one opens instead.
 */
export function NewThreadPane({
  roomId,
  parentId,
}: {
  readonly roomId: number;
  readonly parentId: number;
}) {
  const navigate = useNavigate();
  const parent = useStore((state) => state.messages[parentId]);
  const timelineReady = useStore((state) => state.timelines[roomId]?.status === "ready");
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  const [name, setName] = useState("");
  // One id for every attempt from this pane, so a retry after a lost reply can't start a second
  // thread: the server answers the one the first attempt made.
  const [clientMessageId] = useState(() => uuid7(Date.now()));
  // A link can name a message outside the loaded window: look it up once before giving up.
  const [lookedUp, setLookedUp] = useState(false);
  const missing = timelineReady && parent === undefined;
  const existing = parent?.thread?.threadId ?? null;

  useEffect(() => {
    if (!missing || lookedUp) return;

    let live = true;

    void actions.loadAround(roomId, parentId).finally(() => {
      if (live) setLookedUp(true);
    });

    return () => {
      live = false;
    };
  }, [missing, lookedUp, roomId, parentId]);

  useEffect(() => {
    if (existing !== null) {
      void navigate({
        to: "/r/$roomId/t/$threadId",
        params: { roomId, threadId: existing },
        replace: true,
      });
    }
  }, [existing, navigate, roomId]);

  const submit = (draft: ComposerDraft): Promise<void> =>
    actions.threads
      .create(roomId, parentId, draft.markdown, {
        name: name.trim() === "" ? null : name.trim(),
        attachmentSignedId: draft.attachmentSignedId,
        driveFileIds: draft.driveFileIds,
        clientMessageId,
      })
      .then(
        (threadId) => {
          void navigate({
            to: "/r/$roomId/t/$threadId",
            params: { roomId, threadId },
            replace: true,
          });
        },
        (error: Error) => {
          toast({ title: "Couldn't start the thread", description: error.message, tone: "danger" });

          throw error;
        },
      );

  const placeholder =
    parent === undefined
      ? "Thread name"
      : defaultThreadName(parent.markdownSource, parent.bodyHtml);

  return (
    <PaneFrame
      title="New thread"
      footer={
        parent === undefined ? undefined : (
          <Composer
            roomId={roomId}
            onSubmit={submit}
            placeholder="Reply…"
            draftKey={newThreadDraftKey(roomId, parent.id)}
          />
        )
      }
    >
      {parent === undefined ? (
        missing && lookedUp ? (
          <PaneEmpty
            icon="thread"
            title="Message not found"
            text="It may have been deleted, or it's further back than what's loaded."
          />
        ) : (
          <ThreadSkeleton />
        )
      ) : (
        <div className="new-thread">
          <MessageRow
            message={parent}
            groupStart
            mentionsMe={viewerId !== null && parent.bodyHtml.includes(`data-user-id="${viewerId}"`)}
            focused={false}
            live={false}
            inThread
          />
          <div className="thread-replies-rule">
            <span className="thread-replies-label">Start a thread</span>
          </div>
          <div className="new-thread-name">
            <TextField
              label="Thread name (optional)"
              value={name}
              maxLength={100}
              placeholder={placeholder}
              onChange={(event) => setName(event.target.value)}
            />
          </div>
        </div>
      )}
    </PaneFrame>
  );
}
