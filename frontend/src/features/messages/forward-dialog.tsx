import { type ReactNode, useEffect, useRef, useState } from "react";
import type { ForwardDestination } from "../../gen/ForwardDestination.ts";
import type { ForwardTarget } from "../../gen/ForwardTarget.ts";
import { SuccessCheck } from "../../motion/success-check.tsx";
import type { MessageDTO } from "../../store/model.ts";
import { store } from "../../store/store.ts";
import type { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button, Spinner } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { toast } from "../../ui/toast-store.ts";
import { plainText } from "./commands.ts";
import { filterDestinations, MAX_FORWARDS, targetKey } from "./forward-targets.ts";

interface ForwardDialogProps {
  readonly message: MessageDTO;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly destinations: readonly ForwardDestination[] };

/** How long the "Forwarded" check shows before the dialog closes. */
const DONE_DWELL_MS = 900;

function Preview({ message }: { readonly message: MessageDTO }) {
  const author = store.getState().users[message.creatorId]?.name ?? "Someone";
  const text = plainText(message);

  return (
    <figure className="forward-preview">
      <figcaption className="forward-preview-author">{author}</figcaption>
      <p className="forward-preview-text">
        {text === "" ? (message.attachment?.filename ?? "Attachment") : text}
      </p>
    </figure>
  );
}

/**
 * Forward a message: search the rooms (and their open threads) the viewer may post to, tick up to
 * five, add an optional note, send. The button turns into a drawn check before the dialog closes.
 */
export default function ForwardDialog({ message, open, onOpenChange }: ForwardDialogProps) {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<readonly ForwardTarget[]>([]);
  const [note, setNote] = useState("");
  const [sending, setSending] = useState(false);
  const [done, setDone] = useState(false);
  const closeTimer = useRef(0);

  useEffect(() => {
    let live = true;

    actions.messages.forwardDestinations().then(
      (list) => {
        if (live) setLoad({ status: "ready", destinations: list.destinations });
      },
      (error: ActionError) => {
        if (live) setLoad({ status: "error", message: error.message });
      },
    );

    return () => {
      live = false;
      window.clearTimeout(closeTimer.current);
    };
  }, []);

  const isSelected = (target: ForwardTarget) =>
    selected.some((entry) => targetKey(entry) === targetKey(target));

  const toggle = (target: ForwardTarget, on: boolean) => {
    setSelected((current) =>
      on
        ? [...current, target].slice(0, MAX_FORWARDS)
        : current.filter((entry) => targetKey(entry) !== targetKey(target)),
    );
  };

  const send = () => {
    if (selected.length === 0 || sending || done) {
      return;
    }

    setSending(true);

    actions.messages.forward(message.id, note.trim() === "" ? null : note.trim(), selected).then(
      () => {
        setSending(false);
        setDone(true);
        closeTimer.current = window.setTimeout(() => onOpenChange(false), DONE_DWELL_MS);
      },
      (error: ActionError) => {
        setSending(false);
        toast({
          title: "Couldn't forward the message",
          description: error.message,
          tone: "danger",
        });
      },
    );
  };

  const full = selected.length >= MAX_FORWARDS;
  const shown = load.status === "ready" ? filterDestinations(load.destinations, query) : [];

  let list: ReactNode;

  if (load.status === "loading") {
    list = (
      <div className="forward-status" role="status">
        <Spinner label="Loading conversations" />
      </div>
    );
  } else if (load.status === "error") {
    list = <p className="forward-status">Couldn't load conversations: {load.message}</p>;
  } else if (shown.length === 0) {
    list = <p className="forward-status">No conversations match “{query.trim()}”.</p>;
  } else {
    list = (
      <ul className="forward-list" aria-label="Conversations">
        {shown.map((destination) => {
          const room = { roomId: destination.roomId, threadId: null };

          return (
            <li key={destination.roomId} className="forward-room">
              <div className="forward-option">
                <Checkbox
                  checked={isSelected(room)}
                  disabled={full && !isSelected(room)}
                  onCheckedChange={(on) => toggle(room, on)}
                  label={
                    <span className="forward-option-label">
                      <Icon
                        name={destination.direct ? "dms" : "hash"}
                        size={14}
                        className="forward-option-icon"
                      />
                      <span className="forward-option-name">{destination.name}</span>
                    </span>
                  }
                />
              </div>
              {destination.threads.length > 0 ? (
                <ul className="forward-threads" aria-label={`Threads in ${destination.name}`}>
                  {destination.threads.map((thread) => {
                    const target = { roomId: destination.roomId, threadId: thread.id };

                    return (
                      <li key={thread.id} className="forward-option forward-option-thread">
                        <Checkbox
                          checked={isSelected(target)}
                          disabled={full && !isSelected(target)}
                          onCheckedChange={(on) => toggle(target, on)}
                          label={
                            <span className="forward-option-label">
                              <Icon name="thread" size={14} className="forward-option-icon" />
                              <span className="forward-option-name">{thread.name}</span>
                            </span>
                          }
                        />
                      </li>
                    );
                  })}
                </ul>
              ) : null}
            </li>
          );
        })}
      </ul>
    );
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Forward message"
      dirty={selected.length > 0 || note.trim() !== ""}
      footer={
        <>
          <span className="forward-count" aria-live="polite">
            {selected.length === 0
              ? `Choose up to ${MAX_FORWARDS}`
              : `${selected.length} of ${MAX_FORWARDS} chosen`}
          </span>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          {done ? (
            <Button variant="primary" aria-live="polite">
              <SuccessCheck size={16} /> Forwarded
            </Button>
          ) : (
            <Button
              variant="primary"
              icon="forward"
              disabled={selected.length === 0}
              loading={sending}
              loadingLabel="Forwarding"
              onClick={send}
            >
              Forward
            </Button>
          )}
        </>
      }
    >
      <div className="forward">
        <Preview message={message} />
        <div className="forward-search">
          <Icon name="search" size={14} className="forward-search-icon" />
          <input
            className="input forward-search-input"
            type="search"
            placeholder="Search conversations"
            aria-label="Search conversations"
            value={query}
            data-autofocus
            onChange={(event) => setQuery(event.target.value)}
          />
        </div>
        <div className="forward-scroll">{list}</div>
        <label className="forward-note">
          <span className="forward-note-label">Add a note (optional)</span>
          <textarea
            className="input forward-note-input"
            rows={2}
            maxLength={50_000}
            value={note}
            placeholder="Say something about it"
            onChange={(event) => setNote(event.target.value)}
          />
        </label>
      </div>
    </Dialog>
  );
}
