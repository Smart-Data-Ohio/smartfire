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
import { uuid7 } from "../../lib/uuid7.ts";
import { store, useStore } from "../../store/store.ts";
import { composerActions } from "../../sync/composer-actions.ts";
import { actions } from "../../sync/runtime.ts";
import { Beam } from "../../ui/beam.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import type { IconName } from "../../ui/icons/icon.tsx";
import { Kbd } from "../../ui/kbd.tsx";
import { toast } from "../../ui/toast-store.ts";
import { LazyCreatePollDialog } from "../cards/lazy-create-poll-dialog.tsx";
import { startHuddleFromCommand } from "../huddle/slash-huddle.ts";
import { editLastOwnMessage } from "../messages/edit-last.ts";
import { notePosted } from "../room/follow-posted.ts";
import { AttachmentTray } from "./attachments/attachment-tray.tsx";
import { DropOverlay, useDropTarget } from "./attachments/drop-zone.tsx";
import {
  fileOptions,
  MAX_FILES,
  pastedName,
  type TrayFile,
  useAttachments,
} from "./attachments/use-attachments.ts";
import { AutocompleteList } from "./autocomplete/autocomplete-list.tsx";
import { loadCommands, selectable } from "./autocomplete/suggestions.ts";
import { applyCompletion, findTrigger } from "./autocomplete/trigger.ts";
import { useAutocomplete } from "./autocomplete/use-autocomplete.ts";
import { draftKey as conversationDraftKey, readDraft, writeDraft } from "./draft.ts";
import { attachDriveFile, type DrivePick } from "./drive-picker.ts";
import { DrivePendingChips, DrivePicker } from "./drive-picker.tsx";
import { ComposerEmojiButton } from "./emoji-button.tsx";
import { insertLink, markerForChord, type TextEdit, toggleWrap } from "./markdown-keys.ts";
import { type PlusAction, PlusMenu } from "./plus-menu/plus-menu.tsx";
import { LazyPreviewPanel } from "./preview/lazy-preview-panel.tsx";
import { ReplyChip } from "./reply-chip.tsx";
import {
  cancelReply,
  cancelReplyAt,
  type ReplySnapshot,
  setReplyNotify,
  snapshotReply,
  trackSentReply,
  useReplyTarget,
} from "./reply-store.ts";
import { LazyCustomTimeDialog } from "./schedule/lazy-custom-time-dialog.tsx";
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
  /** Drive files pinned on the message. Empty when there are none. */
  readonly driveFileIds: readonly string[];
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

/** What one submit took, read as the author submits: only this is sent and then cleared. */
interface Submission {
  readonly snapshot: ReplySnapshot;
  /** The draft as submitted (untrimmed, to compare with the composer's text later). */
  readonly text: string;
  readonly fileIds: ReadonlySet<string>;
  readonly driveFileIds: ReadonlySet<string>;
}

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
  const textRef = useRef(text);
  /** Bumps on every edit and clear: a later outcome only puts back a draft nobody has touched. */
  const draftEdits = useRef(0);
  const [caret, setCaret] = useState(() => ({ start: text.length, end: text.length }));
  const [restore, setRestore] = useState<{ start: number; end: number } | null>(null);
  const [focused, setFocused] = useState(false);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [waiting, setWaiting] = useState(false);
  const [customOpen, setCustomOpen] = useState(false);

  const threadStatus = useStore((state) =>
    threadId === null ? null : (state.threads[threadId]?.status ?? null),
  );

  const canPoll = !creating && (threadId === null || threadStatus === "active");
  const [poll, setPoll] = useState({ open: false, question: "" });
  const [running, setRunning] = useState(false);
  const submitting = useRef(false);
  const [usage, setUsage] = useState<SlashCommand | null>(null);
  const rootRef = useRef<HTMLDivElement | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const conversation = useConversationName(roomId);
  const placeholder = placeholderOverride ?? placeholderFor(conversation);
  const agentReplying = useAgentReplying(roomId);
  const attachments = useAttachments(creating ? 1 : MAX_FILES);
  const [driveFiles, setDriveFiles] = useState<readonly DrivePick[]>([]);
  const [driveOpen, setDriveOpen] = useState(false);
  const scheduledHere = useScheduled(roomId, threadId, !creating);
  const trigger = focused ? findTrigger(text, caret.start, caret.end) : null;
  const autocomplete = useAutocomplete(roomId, threadId, trigger, !creating);
  const hasText = text.trim() !== "";
  const hasFiles = attachments.files.length > 0;
  const hasDrive = driveFiles.length > 0;
  const canSend = (hasText || hasFiles || hasDrive) && !running;
  // An inline reply (classic's Reply): a new thread's first message never carries one.
  const reply = useReplyTarget(creating ? null : key);

  const replyGone = useStore(
    (state) => reply !== null && state.messages[reply.messageId] === undefined,
  );

  const replySeq = reply?.seq ?? null;
  const seenReplySeq = useRef(replySeq);

  // The quoted message was deleted: there's nothing left to reply to.
  useEffect(() => {
    if (replyGone) {
      cancelReply(key);
    }
  }, [replyGone, key]);

  // A Reply picked on a message hands this composer the focus (as classic's does), but coming
  // back to a conversation with a reply already set doesn't.
  useEffect(() => {
    if (replySeq === null || replySeq === seenReplySeq.current) {
      return;
    }

    seenReplySeq.current = replySeq;
    textareaRef.current?.focus();
    requestAnimationFrame(() => textareaRef.current?.focus({ preventScroll: true }));
  }, [replySeq]);

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
    draftEdits.current += 1;
    textRef.current = next;
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
    draftEdits.current += 1;
    textRef.current = "";
    setText("");
    writeDraft(key, "");
    setPreviewOpen(false);
    focusInput();
  };

  /** Clears the draft only if it's still what was submitted; says whether it did. */
  const clearSubmitted = (submitted: string): boolean => {
    if (textRef.current !== submitted) {
      return false;
    }

    clear();

    return true;
  };

  const openPicker = () => fileInputRef.current?.click();

  const addFiles = (files: readonly File[]) => {
    if (files.length === 0) {
      return;
    }

    const overflow = attachments.add(files);

    if (overflow > 0) {
      toast({
        title: creating
          ? "A new thread takes one file"
          : `A message holds up to ${MAX_FILES} files`,
        description: creating
          ? "Add the others in a reply once the thread exists."
          : `${overflow === 1 ? "1 file wasn't" : `${overflow} files weren't`} added. Send these, then add the rest.`,
      });
    }

    focusInput();
  };

  const drop = useDropTarget(rootRef, addFiles, true);

  /** Takes the room's window to the present, where a message from here lands. */
  const toPresent = () => {
    if (threadId === null && store.getState().timelines[roomId]?.after != null) {
      return actions.jumpToPresent(roomId);
    }

    return Promise.resolve();
  };

  /**
   * Posts the text and every submitted file as one message, with the reply the submit took
   * (`snapshot`, read before any await), as classic's text and uploads do. The chip goes with the
   * send; a send that fails puts it back (`trackSentReply`). Only what the submit took is
   * cleared: text typed or files added while it was out stay.
   */
  const deliver = async (
    markdown: string,
    files: readonly TrayFile[],
    snapshot: ReplySnapshot,
    submitted: string,
    drive: readonly DrivePick[],
  ) => {
    if (submitting.current) return;
    submitting.current = true;
    setRunning(true);

    try {
      const latest = toPresent();

      // Sound broadcasts need the latest page; ordinary sends proceed while it loads.
      if (files.length === 0 && slashName(markdown.trim()) === "play") {
        await latest;
      } else {
        void latest;
      }

      const target = snapshot.target;

      const replying =
        target === null ? null : { messageId: target.messageId, notify: target.notify };

      const clientMessageId = uuid7(Date.now());
      const options = { ...fileOptions(files), threadId, reply: replying, clientMessageId };

      actions.send(
        roomId,
        markdown,
        drive.length === 0 ? options : { ...options, driveFileIds: drive.map((file) => file.id) },
      );

      attachments.clearSent(files);
      clearSentDrive(drive);
      trackSentReply(key, snapshot, [clientMessageId]);
      clearSubmitted(submitted);
    } finally {
      submitting.current = false;
      setRunning(false);
    }
  };

  const showResult = (
    result: SlashCommandResult,
    typed: string,
    revision: number,
    restoreTyped: () => void,
  ) => {
    // A command that ran consumes the draft and, as in classic, the reply with it (not one
    // picked while it ran).
    if (result.status !== "error") {
      cancelReplyAt(key, revision);
    }

    switch (result.status) {
      case "posted":
        // The server posted it with no pending row: go to it, as a send does.
        notePosted(threadId === null ? `room:${roomId}` : `thread:${threadId}`, result.messageId);
        void toPresent();

        if (result.notice !== null) {
          toast({ title: result.notice, tone: "success" });
        }

        return;
      case "ephemeral":
        toast({ title: result.message });

        return;
      case "error":
        restoreTyped();
        toast({ title: "That command didn't work", description: result.message, tone: "danger" });

        return;
      case "open_url":
        window.open(result.url, "_blank", "noopener");

        return;
      case "open_poll":
        if (canPoll) {
          setPoll({ open: true, question: typed.replace(/^\/poll\b\s*/i, "") });
        } else {
          toast({
            title: "This thread is closed",
            description: "Reopen it before creating a poll.",
          });
        }

        return;
      case "start_huddle":
        startHuddleFromCommand(result.roomId, result.roomName);
    }
  };

  // The reply and the draft are read before the command-list lookup: a reply picked or text
  // typed during it is newer input.
  const runCommand = async (
    typed: string,
    snapshot: ReplySnapshot = currentReply(),
    submitted: string = text,
  ) => {
    const commands = await loadCommands(roomId, threadId).catch(() => []);
    const route = routeSlash(typed, commands);

    if (route.kind === "message") {
      await deliver(route.markdown, [], snapshot, submitted, []);

      return;
    }

    setRunning(true);
    actions.setTyping(roomId, false, threadId);

    const cleared = clearSubmitted(submitted);
    const edits = draftEdits.current;

    // A command that fails puts its text back, unless the author has started something newer.
    const restoreTyped = () => {
      if (cleared && draftEdits.current === edits) {
        update(typed);
        setRestore({ start: typed.length, end: typed.length });
      }
    };

    try {
      const result = await composerActions.runSlashCommand(roomId, route.text, threadId);

      showResult(result, typed, snapshot.revision, restoreTyped);
    } catch (error) {
      restoreTyped();
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

  /** The reply a submit takes, read as the author submits; a new thread's never has one. */
  const currentReply = () => snapshotReply(creating ? null : key);

  /** What a send waiting on uploads took when Enter was pressed: it sends exactly that. */
  const waitingSubmission = useRef<Submission | null>(null);

  /** The submission's files still in the tray (one removed while it waited is dropped). */
  const submittedFiles = (submission: Submission) =>
    attachments.files.filter((entry) => submission.fileIds.has(entry.id));

  const submittedDrive = (submission: Submission) =>
    driveFiles.filter((file) => submission.driveFileIds.has(file.id));

  const clearSentDrive = (sent: readonly DrivePick[]) => {
    const ids = new Set(sent.map((file) => file.id));

    setDriveFiles((current) => current.filter((file) => !ids.has(file.id)));
  };

  const send = (pending?: Submission) => {
    if (pending === undefined && !canSend) {
      return;
    }

    const submission: Submission = pending ?? {
      snapshot: currentReply(),
      text,
      fileIds: new Set(attachments.files.map((entry) => entry.id)),
      driveFileIds: new Set(driveFiles.map((file) => file.id)),
    };

    const files = submittedFiles(submission);
    const drive = submittedDrive(submission);
    const markdown = submission.text.trimEnd();

    if (markdown === "" && files.length === 0 && drive.length === 0) {
      waitingSubmission.current = null;
      setWaiting(false);

      return;
    }

    if (files.some((entry) => entry.snapshot.phase === "failed")) {
      waitingSubmission.current = null;
      setWaiting(false);
      toast({
        title: "A file didn't upload",
        description: "Retry it or remove it, then send.",
        tone: "danger",
      });

      return;
    }

    if (!files.every((entry) => entry.snapshot.phase === "done")) {
      waitingSubmission.current = submission;
      setWaiting(true);

      return;
    }

    waitingSubmission.current = null;
    setWaiting(false);

    actions.setTyping(roomId, false, threadId);

    if (onSubmit !== undefined) {
      // One request at a time: a second Enter while the first is out would post twice.
      if (submitting.current) {
        return;
      }

      submitting.current = true;
      setRunning(true);

      void onSubmit({
        markdown,
        attachmentSignedId: files[0]?.snapshot.signedId ?? null,
        driveFileIds: drive.map((file) => file.id),
      })
        .then(
          () => {
            attachments.clearSent(files);
            clearSentDrive(drive);
            clearSubmitted(submission.text);
          },
          () => undefined,
        )
        .finally(() => {
          submitting.current = false;
          setRunning(false);
        });

      return;
    }

    if (files.length === 0 && drive.length === 0 && looksLikeCommand(markdown)) {
      void runCommand(markdown, submission.snapshot, submission.text);

      return;
    }

    void deliver(markdown, files, submission.snapshot, submission.text, drive);
  };

  // Enter while files upload: send as soon as the submitted ones are up (or stop if one fails).
  const sendRef = useRef(send);

  useLayoutEffect(() => {
    sendRef.current = send;
  });

  const waitingFor = waitingSubmission.current;

  const uploadsSettled =
    waitingFor === null ||
    submittedFiles(waitingFor).every(
      (entry) => entry.snapshot.phase === "done" || entry.snapshot.phase === "failed",
    );

  useEffect(() => {
    if (waiting && uploadsSettled) {
      sendRef.current(waitingSubmission.current ?? undefined);
    }
  }, [waiting, uploadsSettled]);

  const schedule = (at: Date): Promise<void> => {
    const submitted = text;
    const markdown = submitted.trimEnd();
    const taken = currentReply();
    const files = attachments.files;

    if (!attachments.ready) {
      return Promise.reject(new Error("Wait for the files to upload, or retry failed uploads."));
    }

    return scheduled
      .create(roomId, {
        markdownSource: markdown,
        sendAt: at.toISOString(),
        threadId,
        // Classic's schedule menu keeps the draft's reply target too.
        replyToMessageId: taken.target?.messageId ?? null,
        attachmentSignedIds: files.flatMap((entry) =>
          entry.snapshot.signedId === null ? [] : [entry.snapshot.signedId],
        ),
      })
      .then(() => {
        actions.setTyping(roomId, false, threadId);
        // The reply it took, not one picked while the request was out.
        cancelReplyAt(key, taken.revision);
        attachments.clearSent(files);
        clearSubmitted(submitted);
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

    if (event.key === "Escape" && reply !== null) {
      event.preventDefault();
      event.stopPropagation();
      cancelReply(key);

      return;
    }

    if (event.key === "Enter" && !event.shiftKey && !event.altKey) {
      event.preventDefault();
      send();

      return;
    }

    if (
      event.key === "ArrowUp" &&
      plain &&
      !event.shiftKey &&
      text === "" &&
      !hasFiles &&
      !hasDrive
    ) {
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

  const scheduleBlocked = hasDrive
    ? "Google Drive files can't be scheduled"
    : !attachments.ready
      ? "Wait for the files to upload"
      : null;

  const plusActions: PlusAction[] = [
    {
      id: "upload",
      label: "Upload a file",
      icon: "paperclip",
      shortcut: shortcutKeys("upload"),
      onSelect: openPicker,
    },
    {
      id: "drive",
      label: "From Google Drive",
      icon: "file-text",
      onSelect: () => setDriveOpen(true),
    },
  ];

  if (!creating) {
    plusActions.push({
      id: "schedule",
      label: "Schedule message…",
      icon: "clock",
      disabled: !canSend || scheduleBlocked !== null,
      onSelect: () => setCustomOpen(true),
    });
  }

  if (canPoll) {
    plusActions.push({
      id: "poll",
      label: "Create a poll",
      icon: "chart-bar",
      onSelect: () => setPoll({ open: true, question: "" }),
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
    <div
      className="composer"
      ref={rootRef}
      data-holding={hasText || hasFiles || hasDrive || undefined}
    >
      <Beam active={agentReplying} radius={12}>
        <AutocompleteList autocomplete={autocomplete} onPick={pick} />
        <div className="composer-card" data-drop={drop.active || undefined}>
          {reply === null ? null : (
            <ReplyChip
              target={reply}
              onNotifyChange={(notify) => setReplyNotify(key, notify)}
              onCancel={() => {
                cancelReply(key);
                focusInput();
              }}
            />
          )}
          <LazyPreviewPanel
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
          <DrivePendingChips
            files={driveFiles}
            onRemove={(id) => setDriveFiles((current) => current.filter((file) => file.id !== id))}
          />
          <DrivePicker
            roomId={roomId}
            attachedFileIds={driveFiles.map((file) => file.id)}
            open={driveOpen}
            onOpenChange={setDriveOpen}
            onAttach={(file) => {
              setDriveFiles((current) => {
                const result = attachDriveFile(current, file);

                if (result.status === "full") toast({ title: "Up to 10 Drive files per message" });

                return result.files;
              });
            }}
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
              onSend={() => send()}
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
        <LazyCustomTimeDialog
          open={customOpen}
          onOpenChange={setCustomOpen}
          title="Schedule message"
          confirmLabel="Schedule"
          onConfirm={schedule}
        />
      )}
      {canPoll ? (
        <LazyCreatePollDialog
          roomId={roomId}
          threadId={threadId}
          open={poll.open}
          initialQuestion={poll.question}
          onOpenChange={(open) => setPoll((current) => ({ ...current, open }))}
        />
      ) : null}
    </div>
  );
}

/** The modifier key's label, re-exported for the composer's tests and tooltips. */
export const COMPOSER_MOD = MOD;
