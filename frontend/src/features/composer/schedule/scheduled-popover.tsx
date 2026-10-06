import { useState } from "react";
import type { ScheduledMessage } from "../../../gen/ScheduledMessage.ts";
import { Button } from "../../../ui/button.tsx";
import { IconButton } from "../../../ui/icon-button.tsx";
import { Popover } from "../../../ui/popover.tsx";
import { toast } from "../../../ui/toast-store.ts";
import { CustomTimeDialog } from "./custom-time-dialog.tsx";
import { sendAtLabel } from "./presets.ts";
import { scheduled } from "./scheduled-store.ts";

interface ScheduledPopoverProps {
  readonly items: readonly ScheduledMessage[];
}

/** The Markdown as one quiet line: markers dropped, whitespace folded. */
function excerpt(markdown: string): string {
  return markdown
    .replace(/[*_~`>#]+/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

function failed(title: string) {
  return (error: Error) => toast({ title, description: error.message, tone: "danger" });
}

/**
 * "2 scheduled" beside the send button: a popover listing this conversation's pending scheduled
 * messages with their send time, and for each: reschedule, send now, cancel.
 */
export function ScheduledPopover({ items }: ScheduledPopoverProps) {
  const [editing, setEditing] = useState<ScheduledMessage | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);

  if (items.length === 0 && editing === null) {
    return null;
  }

  const run = (item: ScheduledMessage, work: () => Promise<void>, failure: string) => {
    setBusyId(item.id);
    work()
      .catch(failed(failure))
      .finally(() => setBusyId(null));
  };

  const label = `${items.length} scheduled`;

  return (
    <>
      <Popover
        label="Scheduled messages"
        placement="top-end"
        trigger={(props) => (
          <Button
            {...props}
            variant="ghost"
            size="sm"
            icon="clock"
            className="composer-scheduled"
            aria-label={`${label} messages`}
            onMouseDown={(event) => event.preventDefault()}
          >
            <span className="composer-scheduled-long">{label}</span>
            <span className="composer-scheduled-short">{items.length}</span>
          </Button>
        )}
      >
        <div className="scheduled-list">
          <header className="scheduled-list-head">
            <span className="scheduled-list-title">Scheduled messages</span>
            <span className="scheduled-list-count">{items.length}</span>
          </header>
          {items.length === 0 ? (
            <p className="scheduled-list-empty">Nothing scheduled here.</p>
          ) : (
            <ul className="scheduled-items" tabIndex={-1} data-autofocus>
              {items.map((item) => (
                <li
                  key={item.id}
                  className="scheduled-item"
                  aria-busy={busyId === item.id || undefined}
                >
                  <div className="scheduled-item-text">
                    <span className="scheduled-item-when">
                      {sendAtLabel(new Date(item.sendAt), new Date())}
                    </span>
                    <span className="scheduled-item-body">{excerpt(item.markdownSource)}</span>
                  </div>
                  <div className="scheduled-item-actions">
                    <IconButton
                      icon="calendar-clock"
                      label="Change time"
                      size="sm"
                      disabled={busyId === item.id}
                      onClick={() => setEditing(item)}
                    />
                    <IconButton
                      icon="send"
                      label="Send now"
                      size="sm"
                      disabled={busyId === item.id}
                      onClick={() =>
                        run(item, () => scheduled.sendNow(item), "Couldn't send it now")
                      }
                    />
                    <IconButton
                      icon="trash"
                      label="Cancel scheduled message"
                      size="sm"
                      className="scheduled-cancel"
                      disabled={busyId === item.id}
                      onClick={() => run(item, () => scheduled.cancel(item), "Couldn't cancel it")}
                    />
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </Popover>
      <CustomTimeDialog
        open={editing !== null}
        onOpenChange={(open) => {
          if (!open) {
            setEditing(null);
          }
        }}
        title="Reschedule message"
        confirmLabel="Reschedule"
        initial={editing === null ? null : new Date(editing.sendAt)}
        onConfirm={async (at) => {
          if (editing !== null) {
            await scheduled.reschedule(editing, at.toISOString());
            toast({ title: `Rescheduled for ${sendAtLabel(at, new Date()).replace(/^T/, "t")}` });
          }
        }}
      />
    </>
  );
}
