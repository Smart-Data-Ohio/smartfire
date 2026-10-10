import type { MessageDTO } from "../../store/model.ts";
import { store } from "../../store/store.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { plainText } from "./commands.ts";
import { fileSummary, messageFiles } from "./message-files.ts";

interface DeleteDialogProps {
  readonly message: MessageDTO;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  readonly onConfirm: () => void;
}

/** "Delete message?": a confirmation with the message quoted, Cancel focused first. */
export default function DeleteDialog({
  message,
  open,
  onOpenChange,
  onConfirm,
}: DeleteDialogProps) {
  const author = store.getState().users[message.creatorId]?.name ?? "Someone";
  const text = plainText(message);
  const own = store.getState().me?.user.id === message.creatorId;

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      role="alertdialog"
      size="sm"
      title="Delete message?"
      description={
        own
          ? "It's removed for everyone, along with its reactions and pins. This can't be undone."
          : `You're deleting ${author}'s message for everyone. This can't be undone.`
      }
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)} data-autofocus>
            Cancel
          </Button>
          <Button
            variant="danger"
            icon="trash"
            onClick={() => {
              onConfirm();
              onOpenChange(false);
            }}
          >
            Delete
          </Button>
        </>
      }
    >
      <figure className="forward-preview">
        <figcaption className="forward-preview-author">{author}</figcaption>
        <p className="forward-preview-text">
          {text === "" ? fileSummary(message) : text}
          {text !== "" && messageFiles(message).length > 1 ? ` · ${fileSummary(message)}` : null}
        </p>
      </figure>
    </Dialog>
  );
}
