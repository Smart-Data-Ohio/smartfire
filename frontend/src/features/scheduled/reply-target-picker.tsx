import { useEffect, useId, useState } from "react";
import type { MessagePage } from "../../gen/MessagePage.ts";
import type { ScheduledMessage } from "../../gen/ScheduledMessage.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { snippet } from "../messages/message-content.tsx";

interface ReplyTargetPickerProps {
  readonly item: ScheduledMessage;
  readonly value: number | null;
  readonly onChange: (value: number | null) => void;
}

export function ReplyTargetPicker({ item, value, onChange }: ReplyTargetPickerProps) {
  const id = useId();
  const [page, setPage] = useState<MessagePage | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    actions.scheduled.replyPage(item.roomId, item.threadId, null).then(
      (loaded) => {
        if (current) {
          setPage(loaded);
          setBusy(false);
        }
      },
      (failure: Error) => {
        if (current) {
          setError(failure.message);
          setBusy(false);
        }
      },
    );

    return () => {
      current = false;
    };
  }, [item.roomId, item.threadId]);

  const older = () => {
    if (page === null || page.before === null) return;
    setBusy(true);
    setError(null);
    actions.scheduled.replyPage(item.roomId, item.threadId, page.before).then(
      (loaded) => {
        setPage({
          ...loaded,
          messages: [...loaded.messages, ...page.messages],
          users: [...loaded.users, ...page.users],
        });
        setBusy(false);
      },
      (failure: Error) => {
        setError(failure.message);
        setBusy(false);
      },
    );
  };

  const candidates =
    page?.messages.filter(
      (message) =>
        !message.systemNote && message.roomId === item.roomId && message.threadId === item.threadId,
    ) ?? [];

  const selected = candidates.some((message) => message.id === value);

  return (
    <div className="scheduled-reply-picker">
      <label htmlFor={id} className="scheduled-edit-label">
        Reply target
      </label>
      <select
        id={id}
        className="scheduled-edit-target"
        value={value ?? ""}
        disabled={busy}
        onChange={(event) =>
          onChange(event.target.value === "" ? null : Number(event.target.value))
        }
      >
        <option value="">No reply</option>
        {value === null || selected ? null : <option value={value}>Current reply target</option>}
        {candidates.map((message) => (
          <option key={message.id} value={message.id}>
            {page?.users.find((user) => user.id === message.creatorId)?.name ?? "Someone"}:{" "}
            {snippet(message)}
          </option>
        ))}
      </select>
      {busy ? <p role="status">Loading messages…</p> : null}
      {error === null ? null : (
        <p className="scheduled-edit-error" role="alert">
          {error}
        </p>
      )}
      {page?.before == null ? null : (
        <Button size="sm" variant="secondary" disabled={busy} onClick={older}>
          Load older messages
        </Button>
      )}
    </div>
  );
}
