import {
  type ClipboardEvent,
  type KeyboardEvent,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type { SlashCommand } from "../../gen/SlashCommand.ts";
import type { SlashCommandResult } from "../../gen/SlashCommandResult.ts";
import { MOD, type ShortcutId, shortcutKeys } from "../../lib/shortcuts.ts";
import { store, useStore } from "../../store/store.ts";
import { composerActions } from "../../sync/composer-actions.ts";
import { actions } from "../../sync/runtime.ts";
import { Beam } from "../../ui/beam.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import type { IconName } from "../../ui/icons/icon.tsx";
import { Kbd } from "../../ui/kbd.tsx";
import { toast } from "../../ui/toast-store.ts";
import { editLastOwnMessage } from "../messages/edit-last.ts";
import { AttachmentTray } from "./attachments/attachment-tray.tsx";
import { DropOverlay, useDropTarget } from "./attachments/drop-zone.tsx";
import {
  pastedName,
  pendingAttachment,
  type TrayFile,
  useAttachments,
} from "./attachments/use-attachments.ts";
import { AutocompleteList } from "./autocomplete/autocomplete-list.tsx";
import { loadCommands, selectable } from "./autocomplete/suggestions.ts";
import { applyCompletion, findTrigger } from "./autocomplete/trigger.ts";
import { useAutocomplete } from "./autocomplete/use-autocomplete.ts";
import { draftKey as conversationDraftKey, readDraft, writeDraft } from "./draft.ts";
import { ComposerEmojiButton } from "./emoji-button.tsx";
import { useKeyboardInset } from "./keyboard-inset.ts";
import { insertLink, markerForChord, type TextEdit, toggleWrap } from "./markdown-keys.ts";
import { type PlusAction, PlusMenu } from "./plus-menu/plus-menu.tsx";
import { PreviewPanel } from "./preview/preview-panel.tsx";
import { CustomTimeDialog } from "./schedule/custom-time-dialog.tsx";
import { type SchedulePreset, sendAtLabel } from "./schedule/presets.ts";
import { ScheduledPopover } from "./schedule/scheduled-popover.tsx";
import { scheduled, useScheduled } from "./schedule/scheduled-store.ts";
import { SendButton } from "./schedule/send-button.tsx";
import { commandUsage, looksLikeCommand, routeSlash, slashName } from "./slash.ts";
import { TypingIndicator, useAgentReplying } from "./typing-indicator.tsx";
import "./composer.css";
import "./schedule/schedule.css";

/** What the composer hands over when it sends. */
export interface ComposerDraft {
  readonly markdown: string;
  /** A finished direct upload's signed id, or `null`. */
  readonly attachmentSignedId: string | null;
}

export interface ComposerProps {
  readonly roomId: number;
  /** Reply in this thread: drafts, typing and sends are per thread. */
  readonly threadId?: number | null;
  /**
   * Replaces the normal send (the outbox), e.g. the first reply that creates a thread. The
   * composer clears once it resolves and keeps the text if it rejects. Commands and scheduling
   * are off here (there's no thread to run them in yet), and it takes one file.
   */
  readonly onSubmit?: (draft: ComposerDraft) => Promise<void>;
  /** Overrides "Message #general". */
  readonly placeholder?: string;
  /**
   * Where the draft is kept, when it isn't the conversation's own (see `draft.ts`), e.g. a new
   * thread's first reply, so it neither shows nor clears the room's draft.
   */
  readonly draftKey?: string;
}

/** How many files one message can carry along (the rest go as their own messages). */
const MAX_FILES = 10;

interface FormatButton {
  readonly id: ShortcutId;
  readonly icon: IconName;
  readonly label: string;
  readonly marker: string;
}

const FORMATS: readonly FormatButton[] = [
  { id: "bold", icon: "bold", label: "Bold", marker: "**" },
  { id: "italic", icon: "italic", label: "Italic", marker: "_" },
  { id: "strike", icon: "strikethrough", label: "Strikethrough", marker: "~~" },
  { id: "code", icon: "code", label: "Code", marker: "`" },
  { id: "link", icon: "link", label: "Link", marker: "link" },
];

interface ConversationName {
  readonly kind: "direct" | "room" | null;
  readonly name: string | null;
}

/** The conversation's name and kind, for the placeholder and the drop overlay. */
function useConversationName(roomId: number): ConversationName {
  const kind = useStore((state) => {
    const roomKind =
      state.rooms[roomId]?.detail?.room.kind ?? state.sidebar.rows[roomId]?.room.kind ?? null;

    if (roomKind === null) {
      return null;
    }

    return roomKind === "direct" ? "direct" : "room";
  });

  const name = useStore(
    (state) =>
      state.rooms[roomId]?.detail?.displayName ?? state.sidebar.rows[roomId]?.displayName ?? null,
  );

  return { kind, name };
}

function placeholderFor({ kind, name }: ConversationName): string {
  if (name === null) {
    return "Message";
  }

  return kind === "direct" ? `Message ${name}` : `Message #${name}`;
}

function dropLabelFor({ kind, name }: ConversationName, inThread: boolean): string {
  if (inThread) {
    return "They'll be shared in this thread.";
  }

  if (name === null) {
    return "They'll be shared in this conversation.";
  }

  return kind === "direct" ? `They'll be shared with ${name}.` : `They'll be shared in #${name}.`;
}

/** A pasted URL over a selection makes a link; anything else pastes as usual (`null`). */
function linkOnPaste(edit: TextEdit, pasted: string): TextEdit | null {
  const url = pasted.trim();

  if (edit.start === edit.end || !/^https?:\/\/\S+$/.test(url)) {
    return null;
  }

  const label = edit.value.slice(edit.start, edit.end);
  const link = `[${label}](${url})`;
  const value = edit.value.slice(0, edit.start) + link + edit.value.slice(edit.end);
  const caret = edit.start + link.length;

  return { value, start: caret, end: caret };
}

/**
 * The Markdown composer: a card that grows with its text (to half the viewport), sends on Enter
 * (Shift+Enter for a new line), formats with the toolbar or the chords, completes `@` `:` `/`
 * `#`, runs slash commands, uploads files (button, ⌘U, paste, or a drop anywhere on its pane),
 * previews, and schedules. It keeps a per-conversation draft and tells the room you're typing.
 * While an agent replies here, its border carries the beam.
 */
export function Composer({
  roomId,
  threadId = null,
  onSubmit,
  placeholder: placeholderOverride,
  draftKey,
}: ComposerProps) {
  const key = draftKey ?? conversationDraftKey(roomId, threadId);
  const creating = onSubmit !== undefined;
  const [text, setText] = useState(() => readDraft(key));
  const [caret, setCaret] = useState(() => ({ start: text.length, end: text.length }));
  const [restore, setRestore] = useState<{ start: number; end: number } | null>(null);
  const [focused, setFocused] = useState(false);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [waiting, setWaiting] = useState(false);
  const [customOpen, setCustomOpen] = useState(false);
  const [running, setRunning] = useState(false);
  const [usage, setUsage] = useState<SlashCommand | null>(null);
  const rootRef = useRef<HTMLDivElement | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const conversation = useConversationName(roomId);
  const placeholder = placeholderOverride ?? placeholderFor(conversation);
  const agentReplying = useAgentReplying(roomId);
  const attachments = useAttachments(creating ? 1 : MAX_FILES);
  const scheduledHere = useScheduled(roomId, threadId, !creating);
  const trigger = focused ? findTrigger(text, caret.start, caret.end) : null;
  const autocomplete = useAutocomplete(roomId, threadId, trigger, !creating);
  const hasText = text.trim() !== "";
  const hasFiles = attachments.files.length > 0;
  const canSend = (hasText || hasFiles) && !running;

  useKeyboardInset(rootRef);

  // Grow with the text; CSS caps it at half the viewport and scrolls beyond that.
  useLayoutEffect(() => {
    const textarea = textareaRef.current;

    if (textarea === null) {
      return;
    }

    textarea.style.height = "auto";
    textarea.style.height = `${textarea.scrollHeight}px`;

    if (restore !== null) {
      textarea.setSelectionRange(restore.start, restore.end);
      setCaret(restore);
      setRestore(null);
    }
  });

  // The usage line for a command being typed: `/remind <when> <text>`.
  const typedCommand = creating ? null : slashName(text);

  useEffect(() => {
    if (typedCommand === null) {
      setUsage(null);

      return;
    }

    let live = true;

    loadCommands(roomId, threadId).then(
      (commands) => {
        if (live) {
          setUsage(commands.find((command) => command.name === typedCommand) ?? null);
        }
      },
      () => undefined,
    );

    return () => {
      live = false;
    };
  }, [typedCommand, roomId, threadId]);

  /**
   * Focuses the text now and once more next frame: a menu item's select hands focus back to its
   * trigger right after its handler runs, and the composer should win that.
   */
  const focusInput = () => {
    textareaRef.current?.focus();
    requestAnimationFrame(() => textareaRef.current?.focus({ preventScroll: true }));
  };

  const update = (next: string) => {
    setText(next);
    writeDraft(key, next);
    actions.noteActivity();
    actions.setTyping(roomId, next.trim() !== "", threadId);
  };

  const apply = (edit: TextEdit) => {
    update(edit.value);
    setRestore({ start: edit.start, end: edit.end });
    focusInput();
  };

  const currentEdit = (): TextEdit => {
    const textarea = textareaRef.current;

    return {
      value: text,
      start: textarea?.selectionStart ?? caret.start,
      end: textarea?.selectionEnd ?? caret.end,
    };
  };

  const format = (marker: string) => {
    const edit = currentEdit();

    apply(marker === "link" ? insertLink(edit) : toggleWrap(edit, marker));
  };

  /** Puts `snippet` at the caret, with a space before it when it would touch a word. */
  const insertAtCaret = (snippet: string) => {
    const edit = currentEdit();
    const before = edit.value.slice(0, edit.start);
    const space = before === "" || /\s$/.test(before) ? "" : " ";
    const value = before + space + snippet + edit.value.slice(edit.end);
    const at = edit.start + space.length + snippet.length;

    apply({ value, start: at, end: at });
  };

  const clear = () => {
    setText("");
    writeDraft(key, "");
    setPreviewOpen(false);
    focusInput();
  };

  const openPicker = () => fileInputRef.current?.click();

  const addFiles = (files: readonly File[]) => {
    if (files.length === 0) {
      return;
    }

    const taken = attachments.add(files);

    if (taken < files.length) {
      toast({
        title: creating ? "A new thread takes one file" : `Up to ${MAX_FILES} files at a time`,
        description: creating
          ? "Add the others in a reply once the thread exists."
          : "Send these first, then add the rest.",
      });
    }

    focusInput();
  };

  const drop = useDropTarget(rootRef, addFiles, true);

  /** Posts the text with the first file, and each further file as its own message. */
  const deliver = (markdown: string, files: readonly TrayFile[]) => {
    if (threadId === null && store.getState().timelines[roomId]?.after != null) {
      void actions.jumpToPresent(roomId);
    }

    const [first, ...rest] = files;

    actions.send(roomId, markdown, {
      threadId,
      attachmentSignedId: first?.snapshot.signedId ?? null,
      attachment: first === undefined ? null : pendingAttachment(first),
    });

    for (const entry of rest) {
      actions.send(roomId, "", {
        threadId,
        attachmentSignedId: entry.snapshot.signedId,
        attachment: pendingAttachment(entry),
      });
    }

    attachments.clearSent();
    clear();
  };

  const showResult = (result: SlashCommandResult, typed: string) => {
    switch (result.status) {
      case "posted":
        if (result.notice !== null) {
          toast({ title: result.notice, tone: "success" });
        }

        return;
      case "ephemeral":
        toast({ title: result.message });

        return;
      case "error":
        update(typed);
        setRestore({ start: typed.length, end: typed.length });
        toast({ title: "That command didn't work", description: result.message, tone: "danger" });

        return;
      case "open_url":
        window.open(result.url, "_blank", "noopener");

        return;
      case "open_poll":
        toast({
          title: "Polls aren't in this app yet",
          description: "Create the poll from the classic view for now.",
        });

        return;
      case "start_huddle":
        toast({
          title: `Huddles in #${result.roomName} aren't in this app yet`,
          description: "Start it from the classic view for now.",
        });
    }
  };

  const runCommand = async (typed: string) => {
    const commands = await loadCommands(roomId, threadId).catch(() => []);
    const route = routeSlash(typed, commands);

    if (route.kind === "message") {
      deliver(route.markdown, []);

      return;
    }

    setRunning(true);
    actions.setTyping(roomId, false, threadId);
    clear();

    try {
      showResult(await composerActions.runSlashCommand(roomId, route.text, threadId), typed);
    } catch (error) {
      update(typed);
      toast({
        title: "That command didn't run",
        description:
          error instanceof Error ? error.message : "Check your connection and try again.",
        tone: "danger",
      });
    } finally {
      setRunning(false);
    }
  };

  const send = () => {
    if (!canSend) {
      return;
    }

    if (attachments.failed) {
      setWaiting(false);
      toast({
        title: "A file didn't upload",
        description: "Retry it or remove it, then send.",
        tone: "danger",
      });

      return;
    }

    if (!attachments.ready) {
      setWaiting(true);

      return;
    }

    setWaiting(false);

    const markdown = text.trimEnd();
    const files = attachments.files;

    actions.setTyping(roomId, false, threadId);

    if (onSubmit !== undefined) {
      void onSubmit({ markdown, attachmentSignedId: files[0]?.snapshot.signedId ?? null }).then(
        () => {
          attachments.clearSent();
          clear();
        },
        () => undefined,
      );

      return;
    }

    if (files.length === 0 && looksLikeCommand(markdown)) {
      void runCommand(markdown);

      return;
    }

    deliver(markdown, files);
  };

  // Enter while files upload: send as soon as they're all up (or stop if one fails).
  const sendRef = useRef(send);

  useLayoutEffect(() => {
    sendRef.current = send;
  });

  const uploadsSettled = attachments.ready || attachments.failed || !hasFiles;

  useEffect(() => {
    if (waiting && uploadsSettled) {
      sendRef.current();
    }
  }, [waiting, uploadsSettled]);

  const schedule = (at: Date): Promise<void> => {
    const markdown = text.trimEnd();

    return scheduled
      .create(roomId, {
        markdownSource: markdown,
        sendAt: at.toISOString(),
        threadId,
        replyToMessageId: null,
      })
      .then(() => {
        actions.setTyping(roomId, false, threadId);
        clear();
        toast({
          title: `Scheduled for ${sendAtLabel(at, new Date()).replace(/^T/, "t")}`,
          tone: "success",
        });
      });
  };

  const schedulePreset = (preset: SchedulePreset) => {
    schedule(preset.at).catch((error: Error) =>
      toast({ title: "Couldn't schedule it", description: error.message, tone: "danger" }),
    );
  };

  const pick = (index: number) => {
    const item = autocomplete.items[index];

    if (item === undefined || !selectable(item) || trigger === null || item.insert === null) {
      return;
    }

    if (item.kind === "command" && !item.command.takesArguments) {
      void runCommand(item.insert);

      return;
    }

    const completion = applyCompletion(text, trigger, item.insert);

    update(completion.value);
    setRestore({ start: completion.caret, end: completion.caret });
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.nativeEvent.isComposing) {
      return;
    }

    const plain = !event.metaKey && !event.ctrlKey && !event.altKey;

    if (autocomplete.open && plain) {
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        autocomplete.move(event.key === "ArrowDown" ? 1 : -1);

        return;
      }

      if ((event.key === "Enter" || event.key === "Tab") && !event.shiftKey) {
        event.preventDefault();
        pick(autocomplete.activeIndex);

        return;
      }

      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        autocomplete.dismiss();

        return;
      }
    }

    if (event.key === "Escape" && previewOpen) {
      event.preventDefault();
      event.stopPropagation();
      setPreviewOpen(false);

      return;
    }

    if (event.key === "Enter" && !event.shiftKey && !event.altKey) {
      event.preventDefault();
      send();

      return;
    }

    if (event.key === "ArrowUp" && plain && !event.shiftKey && text === "" && !hasFiles) {
      if (editLastOwnMessage(roomId, threadId)) {
        event.preventDefault();
      }

      return;
    }

    // ⌘K is the switcher everywhere: never handled here.
    if (event.metaKey || event.ctrlKey) {
      if (!event.shiftKey && !event.altKey && event.key.toLowerCase() === "u") {
        event.preventDefault();
        openPicker();

        return;
      }

      const marker = markerForChord(event.key, event.shiftKey);

      if (marker !== null) {
        event.preventDefault();
        format(marker);
      }
    }
  };

  const onPaste = (event: ClipboardEvent<HTMLTextAreaElement>) => {
    const files = [...event.clipboardData.files];

    if (files.length > 0) {
      event.preventDefault();
      addFiles(files.map((file) => pastedName(file)));

      return;
    }

    const link = linkOnPaste(currentEdit(), event.clipboardData.getData("text/plain"));

    if (link !== null) {
      event.preventDefault();
      apply(link);
    }
  };

  const syncCaret = () => {
    const textarea = textareaRef.current;

    if (textarea !== null) {
      setCaret({ start: textarea.selectionStart, end: textarea.selectionEnd });
    }
  };

  const scheduleBlocked = hasFiles ? "Files can't be scheduled" : null;

  const plusActions: PlusAction[] = [
    {
      id: "upload",
      label: "Upload a file",
      icon: "paperclip",
      shortcut: shortcutKeys("upload"),
      onSelect: openPicker,
    },
  ];

  if (!creating) {
    plusActions.push({
      id: "schedule",
      label: "Schedule message…",
      icon: "clock",
      disabled: !hasText || hasFiles,
      onSelect: () => setCustomOpen(true),
    });
  }

  plusActions.push({
    id: "preview",
    label: previewOpen ? "Hide preview" : "Preview message",
    icon: "eye",
    onSelect: () => {
      setPreviewOpen((open) => !open);
      focusInput();
    },
  });

  if (!creating) {
    plusActions.push({
      id: "command",
      label: "Run a command",
      icon: "square-slash",
      groupStart: true,
      disabled: hasText,
      onSelect: () => apply({ value: "/", start: 1, end: 1 }),
    });
  }

  plusActions.push({
    id: "mention",
    label: "Mention someone",
    icon: "at",
    groupStart: creating,
    onSelect: () => insertAtCaret("@"),
  });

  return (
    <div className="composer" ref={rootRef}>
      <Beam active={agentReplying} radius={12}>
        <AutocompleteList autocomplete={autocomplete} onPick={pick} />
        <div className="composer-card" data-drop={drop.active || undefined}>
          <PreviewPanel
            open={previewOpen}
            roomId={roomId}
            markdown={text}
            onClose={() => {
              setPreviewOpen(false);
              focusInput();
            }}
          />
          <AttachmentTray
            files={attachments.files}
            onRemove={attachments.remove}
            onRetry={attachments.retry}
          />
          {usage === null ? null : (
            <p className="composer-usage" aria-live="polite">
              <span className="composer-usage-line">{commandUsage(usage)}</span>
              <span className="composer-usage-text">{usage.description}</span>
            </p>
          )}
          <textarea
            ref={textareaRef}
            className="composer-input"
            value={text}
            placeholder={placeholder}
            aria-label={placeholder}
            aria-autocomplete="list"
            aria-controls={autocomplete.open ? autocomplete.listboxId : undefined}
            aria-activedescendant={
              autocomplete.open ? autocomplete.optionId(autocomplete.activeIndex) : undefined
            }
            rows={1}
            onChange={(event) => {
              update(event.target.value);
              setCaret({ start: event.target.selectionStart, end: event.target.selectionEnd });
            }}
            onSelect={syncCaret}
            onKeyDown={onKeyDown}
            onPaste={onPaste}
            onFocus={() => setFocused(true)}
            onBlur={() => {
              setFocused(false);
              actions.setTyping(roomId, false, threadId);
            }}
          />
          <div className="composer-toolbar">
            <PlusMenu actions={plusActions} />
            <span className="composer-divider" aria-hidden="true" />
            <div className="composer-formats">
              {FORMATS.map((entry) => (
                <IconButton
                  key={entry.id}
                  icon={entry.icon}
                  label={entry.label}
                  shortcut={shortcutKeys(entry.id)}
                  size="sm"
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => format(entry.marker)}
                />
              ))}
              <span className="composer-divider" aria-hidden="true" />
            </div>
            <ComposerEmojiButton onInsert={insertAtCaret} />
            <IconButton
              icon="at"
              label="Mention someone"
              size="sm"
              className="composer-mention"
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => insertAtCaret("@")}
            />
            <span className="composer-spacer" />
            <span className="composer-hint" data-visible={hasText || undefined} aria-hidden="true">
              <Kbd keys={shortcutKeys("newline")} /> new line
            </span>
            {creating ? null : <ScheduledPopover items={scheduledHere} />}
            <SendButton
              canSend={canSend}
              waiting={waiting || running}
              onSend={send}
              schedule={
                creating
                  ? null
                  : {
                      enabled: scheduleBlocked === null,
                      reason: scheduleBlocked,
                      onPreset: schedulePreset,
                      onCustom: () => setCustomOpen(true),
                    }
              }
            />
          </div>
        </div>
      </Beam>
      <TypingIndicator roomId={roomId} threadId={threadId} />
      <input
        ref={fileInputRef}
        type="file"
        multiple={!creating}
        hidden
        tabIndex={-1}
        aria-hidden="true"
        onChange={(event) => {
          addFiles([...(event.target.files ?? [])]);
          event.target.value = "";
        }}
      />
      <DropOverlay
        target={drop}
        label={dropLabelFor(conversation, threadId !== null || creating)}
      />
      {creating ? null : (
        <CustomTimeDialog
          open={customOpen}
          onOpenChange={setCustomOpen}
          title="Schedule message"
          confirmLabel="Schedule"
          onConfirm={schedule}
        />
      )}
    </div>
  );
}

/** The modifier key's label, re-exported for the composer's tests and tooltips. */
export const COMPOSER_MOD = MOD;
