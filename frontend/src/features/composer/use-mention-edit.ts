import { type RefObject, useLayoutEffect, useRef } from "react";
import { editMentions, type MentionEdit, type MentionText } from "./mentions.ts";

/** Capture the selection before the browser changes it, including paste, cut and composition. */
export function useMentionEdit(ref: RefObject<HTMLTextAreaElement | null>) {
  const pending = useRef<MentionEdit | null>(null);

  useLayoutEffect(() => {
    const textarea = ref.current;

    if (textarea === null) {
      return;
    }

    const remember = (event: InputEvent) => {
      pending.current = event.inputType.startsWith("history")
        ? null
        : { start: textarea.selectionStart, end: textarea.selectionEnd };
    };

    textarea.addEventListener("beforeinput", remember);

    return () => textarea.removeEventListener("beforeinput", remember);
  });

  return (current: MentionText, text: string, caret: number): MentionText => {
    const selection = pending.current;

    pending.current = null;

    // A collapsed deletion removes text beside the old caret. The new caret locates that cut.
    const replaced =
      selection !== null && selection.start === selection.end && text.length < current.text.length
        ? { start: caret, end: caret + current.text.length - text.length }
        : selection;

    return editMentions(current, text, replaced ?? undefined);
  };
}
