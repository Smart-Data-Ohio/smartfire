/**
 * The composer's emoji button hands off to an emoji picker registered here (the messages slice
 * owns the picker). With none registered, the button starts a `:` autocomplete instead.
 */

/** Opens a picker anchored on `anchor`; `insert` puts the chosen `:name:` at the caret. */
export type EmojiPickerOpener = (anchor: HTMLElement, insert: (text: string) => void) => void;

let opener: EmojiPickerOpener | null = null;

/** Registers the picker; returns the unregister function. */
export function registerComposerEmojiPicker(open: EmojiPickerOpener): () => void {
  opener = open;

  return () => {
    if (opener === open) {
      opener = null;
    }
  };
}

/** The registered picker, or `null`. */
export function composerEmojiPicker(): EmojiPickerOpener | null {
  return opener;
}
