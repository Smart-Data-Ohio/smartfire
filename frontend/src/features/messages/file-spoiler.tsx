import { type ReactNode, useState } from "react";
import "./file-spoiler.css";

export function SpoilerCover({
  onReveal,
  tile = false,
}: {
  readonly onReveal: () => void;
  readonly tile?: boolean;
}) {
  return (
    <button
      type="button"
      className={`attachment-spoiler${tile ? " attachment-tile" : ""}`}
      aria-label="Spoiler, activate to reveal"
      onClick={(event) => {
        event.stopPropagation();
        onReveal();
      }}
    >
      Spoiler
    </button>
  );
}

export function FileSpoiler({
  spoiler,
  children,
  tile = false,
}: {
  readonly spoiler: boolean;
  readonly children: ReactNode;
  readonly tile?: boolean;
}) {
  const [revealed, setRevealed] = useState(false);

  return spoiler && !revealed ? (
    <SpoilerCover tile={tile} onReveal={() => setRevealed(true)} />
  ) : (
    children
  );
}
