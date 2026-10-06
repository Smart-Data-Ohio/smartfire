import { type KeyboardEvent, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { MessageDTO } from "../../store/model.ts";
import type { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button, Spinner } from "../../ui/button.tsx";
import { Kbd } from "../../ui/kbd.tsx";
import { toast } from "../../ui/toast-store.ts";
import {
  insertLink,
  markerForChord,
  type TextEdit,
  toggleWrap,
} from "../composer/markdown-keys.ts";

interface MessageEditorProps {
  readonly message: MessageDTO;
  /** The edit ended (saved, cancelled or failed to load); the row restores focus. */
  readonly onClose: () => void;
  /** Saving an empty message with no file means deleting it: the row asks first. */
  readonly onRequestDelete: () => void;
}

/** What Enter does in the editor, matching the composer: Enter saves, Shift/Alt+Enter is a newline. */
export function editorKeyAction(
  event: Pick<KeyboardEvent, "key" | "shiftKey" | "altKey" | "metaKey" | "ctrlKey">,
): "save" | "cancel" | null {
  if (event.key === "Escape") {
    return "cancel";
  }

  if (event.key !== "Enter") {
    return null;
  }

  if (event.metaKey || event.ctrlKey) {
    return "save";
  }

  return event.shiftKey || event.altKey ? null : "save";
}

/**
 * The in-place editor: the row's body becomes a Markdown box (loaded from the server when the
 * message carries no source), Enter saves, Esc cancels, Shift+Enter makes a new line, and the
 * composer's formatting chords work. Saving unchanged text just closes.
 */
export function MessageEditor({ message, onClose, onRequestDelete }: MessageEditorProps) {
  const [text, setText] = useState<string | null>(message.markdownSource);
  const [original, setOriginal] = useState<string | null>(message.markdownSource);
  const [saving, setSaving] = useState(false);
  const [selection, setSelection] = useState<{ start: number; end: number } | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const placedCaret = useRef(false);
  const onCloseRef = useRef(onClose);

  useLayoutEffect(() => {
    onCloseRef.current = onClose;
  });

  // Rich-text-only bodies: fetch the Markdown the server would edit.
  useEffect(() => {
    if (message.markdownSource !== null) {
      return;
    }

    let live = true;

    actions.messages.source(message.id).then(
      (source) => {
        if (live) {
          setText(source);
          setOriginal(source);
        }
      },
      (error: ActionError) => {
        if (live) {
          toast({
            title: "Couldn't open the message for editing",
            description: error.message,
            tone: "danger",
          });
          onCloseRef.current();
        }
      },
    );

    return () => {
      live = false;
    };
  }, [message.id, message.markdownSource]);

  // Grow with the text, and put the caret at the end once the text is in.
  useLayoutEffect(() => {
    const textarea = textareaRef.current;

    if (textarea === null) {
      return;
    }

    textarea.style.height = "auto";
    textarea.style.height = `${textarea.scrollHeight}px`;

    if (!placedCaret.current) {
      placedCaret.current = true;
      textarea.focus({ preventScroll: true });
      textarea.setSelectionRange(textarea.value.length, textarea.value.length);
      // The whole editor, buttons included, into view: the row grew past the list's edge.
      textarea.closest(".message-editor")?.scrollIntoView({ block: "nearest" });
    }

    if (selection !== null) {
      textarea.setSelectionRange(selection.start, selection.end);
      setSelection(null);
    }
  });

  const save = () => {
    if (text === null || saving) {
      return;
    }

    const markdown = text.trimEnd();

    if (markdown === (original ?? "").trimEnd()) {
      onClose();

      return;
    }

    if (markdown.trim() === "" && message.attachment === null) {
      onRequestDelete();

      return;
    }

    setSaving(true);

    actions.messages.edit(message.id, markdown).then(onClose, (error: ActionError) => {
      setSaving(false);
      toast({ title: "Couldn't save your edit", description: error.message, tone: "danger" });
    });
  };

  const apply = (edit: TextEdit) => {
    setText(edit.value);
    setSelection({ start: edit.start, end: edit.end });
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.nativeEvent.isComposing) {
      return;
    }

    const action = editorKeyAction(event);

    if (action !== null) {
      event.preventDefault();
      event.stopPropagation();

      if (action === "save") {
        save();
      } else {
        onClose();
      }

      return;
    }

    if ((event.metaKey || event.ctrlKey) && text !== null) {
      const marker = markerForChord(event.key, event.shiftKey);

      if (marker !== null) {
        event.preventDefault();

        const current = {
          value: text,
          start: event.currentTarget.selectionStart,
          end: event.currentTarget.selectionEnd,
        };

        apply(marker === "link" ? insertLink(current) : toggleWrap(current, marker));
      }
    }
  };

  if (text === null) {
    return (
      <div className="message-editor message-editor-loading" role="status">
        <Spinner label="Loading the message" />
      </div>
    );
  }

  return (
    <div className="message-editor enter-fade">
      <div className="message-editor-card">
        <textarea
          ref={textareaRef}
          className="message-editor-input"
          value={text}
          rows={1}
          aria-label="Edit message"
          disabled={saving}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={onKeyDown}
        />
      </div>
      <div className="message-editor-footer">
        <span className="message-editor-hint">
          <Kbd keys={["Esc"]} /> to cancel <span aria-hidden="true">·</span> <Kbd keys={["⏎"]} /> to
          save
        </span>
        <span className="message-editor-actions">
          <Button variant="ghost" size="sm" onClick={onClose} disabled={saving}>
            Cancel
          </Button>
          <Button variant="primary" size="sm" onClick={save} loading={saving} loadingLabel="Saving">
            Save
          </Button>
        </span>
      </div>
    </div>
  );
}
