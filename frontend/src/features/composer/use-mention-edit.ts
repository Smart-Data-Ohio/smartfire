import { type RefObject, useLayoutEffect, useRef } from "react";
import { editMentions, type MentionEdit, type MentionText } from "./mentions.ts";

const HISTORY_LIMIT = 100;

interface PendingEdit extends MentionEdit {
  readonly inputType: string;
}

function sameSnapshot(left: MentionText, right: MentionText): boolean {
  return (
    left.text === right.text &&
    left.mentions.length === right.mentions.length &&
    left.mentions.every((mention, index) => {
      const other = right.mentions[index];

      return (
        other !== undefined &&
        mention.start === other.start &&
        mention.text === other.text &&
        mention.token === other.token
      );
    })
  );
}

/** Capture selections and identities before browser edits, including history and composition. */
export function useMentionEdit(
  ref: RefObject<HTMLTextAreaElement | null>,
  draft: MentionText | null,
) {
  const pending = useRef<PendingEdit | null>(null);
  const history = useRef<{ snapshots: MentionText[]; index: number }>({ snapshots: [], index: -1 });

  const rememberSnapshot = (snapshot: MentionText) => {
    const state = history.current;
    const current = state.snapshots[state.index];

    if (current !== undefined && sameSnapshot(current, snapshot)) return;

    state.snapshots.splice(state.index + 1);
    state.snapshots.push(snapshot);

    if (state.snapshots.length > HISTORY_LIMIT) state.snapshots.shift();
    state.index = state.snapshots.length - 1;
  };

  useLayoutEffect(() => {
    if (draft !== null) rememberSnapshot(draft);

    const textarea = ref.current;

    if (textarea === null) return;

    const remember = (event: InputEvent) => {
      pending.current = {
        inputType: event.inputType,
        start: textarea.selectionStart,
        end: textarea.selectionEnd,
      };
    };

    textarea.addEventListener("beforeinput", remember);

    return () => textarea.removeEventListener("beforeinput", remember);
  });

  return (current: MentionText, text: string, caret: number): MentionText => {
    const selection = pending.current;
    pending.current = null;
    rememberSnapshot(current);

    const inputType = selection?.inputType ?? "";

    if (inputType.startsWith("history") || inputType.includes("Composition")) {
      const state = history.current;
      const direction = inputType === "historyRedo" ? 1 : -1;

      for (
        let index = state.index + direction;
        index >= 0 && index < state.snapshots.length;
        index += direction
      ) {
        const snapshot = state.snapshots[index];

        if (snapshot?.text === text) {
          state.index = index;

          return snapshot;
        }
      }
    }

    // History has no replaced selection. A collapsed deletion removes text beside the old caret.
    const replaced = inputType.startsWith("history")
      ? undefined
      : selection !== null && selection.start === selection.end && text.length < current.text.length
        ? { start: caret, end: caret + current.text.length - text.length }
        : (selection ?? undefined);

    const next = editMentions(current, text, replaced);
    rememberSnapshot(next);

    return next;
  };
}
