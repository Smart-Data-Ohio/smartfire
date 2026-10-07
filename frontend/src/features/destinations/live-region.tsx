import { type ReactElement, useState } from "react";

interface Announcer {
  /** Says `text` politely; the same text twice is said twice. */
  readonly announce: (text: string) => void;
  /** The page's one visually hidden live region: render it once, anywhere on the page. */
  readonly region: ReactElement;
}

/**
 * A polite live region for what a key or menu just did to a row ("Marked handled", "Removed"),
 * since the row itself goes and focus moves on to its neighbour. Each message is a fresh node, so
 * a repeat is announced again.
 */
export function useAnnouncer(): Announcer {
  const [said, setSaid] = useState<{ readonly text: string; readonly count: number }>({
    text: "",
    count: 0,
  });

  const announce = (text: string) => setSaid((last) => ({ text, count: last.count + 1 }));

  const region = (
    <div className="visually-hidden" role="status" aria-live="polite" aria-atomic="true">
      {said.text === "" ? null : <span key={said.count}>{said.text}</span>}
    </div>
  );

  return { announce, region };
}
