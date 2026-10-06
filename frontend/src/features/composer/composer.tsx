import { type KeyboardEvent, useLayoutEffect, useRef, useState } from "react";
import { store, useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Beam } from "../../ui/beam.tsx";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import type { IconName } from "../../ui/icons/icon.tsx";
import { Kbd } from "../../ui/kbd.tsx";
import { insertLink, markerForChord, type TextEdit, toggleWrap } from "./markdown-keys.ts";
import { TypingIndicator, useAgentReplying } from "./typing-indicator.tsx";
import "./composer.css";

const DRAFT_PREFIX = "smartfire.draft.";

function readDraft(roomId: number): string {
  try {
    return sessionStorage.getItem(`${DRAFT_PREFIX}${roomId}`) ?? "";
  } catch {
    return "";
  }
}

function writeDraft(roomId: number, text: string): void {
  try {
    if (text === "") {
      sessionStorage.removeItem(`${DRAFT_PREFIX}${roomId}`);
    } else {
      sessionStorage.setItem(`${DRAFT_PREFIX}${roomId}`, text);
    }
  } catch {
    // Without storage a draft only lives while the room is open.
  }
}

const IS_MAC = /Mac|iPhone|iPad/.test(navigator.platform);

const MOD = IS_MAC ? "⌘" : "Ctrl";

interface FormatButton {
  readonly icon: IconName;
  readonly label: string;
  readonly marker: string;
  readonly shortcut: readonly string[];
}

const FORMATS: readonly FormatButton[] = [
  { icon: "bold", label: "Bold", marker: "**", shortcut: [MOD, "B"] },
  { icon: "italic", label: "Italic", marker: "_", shortcut: [MOD, "I"] },
  { icon: "strikethrough", label: "Strikethrough", marker: "~~", shortcut: [MOD, "⇧", "X"] },
  { icon: "code", label: "Code", marker: "`", shortcut: [MOD, "E"] },
  { icon: "link", label: "Link", marker: "link", shortcut: [MOD, "⇧", "U"] },
];

/** The composer's placeholder: "Message #general", or "Message Ada" in a DM. */
function usePlaceholder(roomId: number): string {
  return useStore((state) => {
    const row = state.sidebar.rows[roomId];
    const detail = state.rooms[roomId]?.detail;
    const kind = detail?.room.kind ?? row?.room.kind;
    const name = detail?.displayName ?? row?.displayName;

    if (name === undefined) {
      return "Message";
    }

    return kind === "direct" ? `Message ${name}` : `Message #${name}`;
  });
}

/**
 * The Markdown composer: a card that grows with its text (to half the viewport), sends on Enter
 * (Shift+Enter for a new line), formats with the toolbar or the chords, keeps a per-room draft,
 * and tells the room you're typing. While an agent replies here, its border carries the beam.
 */
export function Composer({ roomId }: { readonly roomId: number }) {
  const [text, setText] = useState(() => readDraft(roomId));
  const [selection, setSelection] = useState<{ start: number; end: number } | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const placeholder = usePlaceholder(roomId);
  const agentReplying = useAgentReplying(roomId);
  const canSend = text.trim() !== "";

  // Grow with the text; CSS caps it at half the viewport and scrolls beyond that.
  useLayoutEffect(() => {
    const textarea = textareaRef.current;

    if (textarea === null) {
      return;
    }

    textarea.style.height = "auto";
    textarea.style.height = `${textarea.scrollHeight}px`;

    if (selection !== null) {
      textarea.setSelectionRange(selection.start, selection.end);
      setSelection(null);
    }
  });

  const update = (next: string) => {
    setText(next);
    writeDraft(roomId, next);
    actions.noteActivity();
    actions.setTyping(roomId, next.trim() !== "");
  };

  const apply = (edit: TextEdit) => {
    update(edit.value);
    setSelection({ start: edit.start, end: edit.end });
    textareaRef.current?.focus();
  };

  const format = (marker: string) => {
    const textarea = textareaRef.current;

    if (textarea === null) {
      return;
    }

    const current = { value: text, start: textarea.selectionStart, end: textarea.selectionEnd };

    apply(marker === "link" ? insertLink(current) : toggleWrap(current, marker));
  };

  const send = () => {
    if (!canSend) {
      return;
    }

    if (store.getState().timelines[roomId]?.after != null) {
      void actions.jumpToPresent(roomId);
    }

    actions.send(roomId, text.trimEnd());
    actions.setTyping(roomId, false);
    setText("");
    writeDraft(roomId, "");
    textareaRef.current?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.nativeEvent.isComposing) {
      return;
    }

    if (event.key === "Enter" && !event.shiftKey && !event.altKey) {
      event.preventDefault();
      send();

      return;
    }

    if (event.metaKey || event.ctrlKey) {
      const marker = markerForChord(event.key, event.shiftKey);

      if (marker !== null) {
        event.preventDefault();
        format(marker);
      }
    }
  };

  return (
    <div className="composer">
      <Beam active={agentReplying} radius={12}>
        <div className="composer-card">
          <textarea
            ref={textareaRef}
            className="composer-input"
            value={text}
            placeholder={placeholder}
            aria-label={placeholder}
            rows={1}
            onChange={(event) => update(event.target.value)}
            onKeyDown={onKeyDown}
            onBlur={() => actions.setTyping(roomId, false)}
          />
          <div className="composer-toolbar">
            <div className="composer-formats">
              {FORMATS.map((entry) => (
                <IconButton
                  key={entry.icon}
                  icon={entry.icon}
                  label={entry.label}
                  shortcut={entry.shortcut}
                  size="sm"
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => format(entry.marker)}
                />
              ))}
            </div>
            <span className="composer-hint" data-visible={canSend || undefined} aria-hidden="true">
              <Kbd keys={["⇧", "⏎"]} /> new line
            </span>
            <Button
              variant={canSend ? "primary" : "ghost"}
              size="sm"
              icon="send"
              className="composer-send"
              aria-label="Send message"
              aria-keyshortcuts="Enter"
              disabled={!canSend}
              onMouseDown={(event) => event.preventDefault()}
              onClick={send}
            />
          </div>
        </div>
      </Beam>
      <TypingIndicator roomId={roomId} />
    </div>
  );
}
