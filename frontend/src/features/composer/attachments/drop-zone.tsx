import { type RefObject, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { usePresence } from "../../../motion/presence.ts";
import { Icon } from "../../../ui/icons/icon.tsx";

/**
 * The pane a composer accepts drops for: an element marked `data-composer-drop-target`, a thread
 * pane, or the room's conversation section, whichever is closest; else the composer's parent.
 */
const DROP_TARGET = "[data-composer-drop-target], .thread-pane, .room";

function hasFiles(event: DragEvent): boolean {
  return event.dataTransfer?.types.includes("Files") ?? false;
}

export interface DropTarget {
  readonly active: boolean;
  /** The pane's box while a drag is over it, for the overlay. */
  readonly rect: DOMRect | null;
}

/**
 * Listens for files dragged over the composer's pane (found from the composer itself, so the
 * pane's own code needn't know): the pane shows the drop overlay, and a drop hands the files over.
 */
export function useDropTarget(
  anchorRef: RefObject<HTMLElement | null>,
  onFiles: (files: readonly File[]) => void,
  enabled: boolean,
): DropTarget {
  const [rect, setRect] = useState<DOMRect | null>(null);
  const onFilesRef = useRef(onFiles);

  useEffect(() => {
    onFilesRef.current = onFiles;
  });

  useEffect(() => {
    const anchor = anchorRef.current;
    const pane = anchor?.closest<HTMLElement>(DROP_TARGET) ?? anchor?.parentElement ?? null;

    if (!enabled || pane === null) {
      return;
    }

    // dragenter/dragleave fire for every child crossed: count them to know when it really left.
    let depth = 0;

    const onEnter = (event: DragEvent) => {
      if (!hasFiles(event)) {
        return;
      }

      event.preventDefault();
      depth += 1;
      setRect(pane.getBoundingClientRect());
    };

    const onOver = (event: DragEvent) => {
      if (!hasFiles(event)) {
        return;
      }

      event.preventDefault();

      if (event.dataTransfer !== null) {
        event.dataTransfer.dropEffect = "copy";
      }
    };

    const onLeave = (event: DragEvent) => {
      if (!hasFiles(event)) {
        return;
      }

      depth = Math.max(0, depth - 1);

      if (depth === 0) {
        setRect(null);
      }
    };

    const onDrop = (event: DragEvent) => {
      if (!hasFiles(event)) {
        return;
      }

      event.preventDefault();
      depth = 0;
      setRect(null);

      const files = [...(event.dataTransfer?.files ?? [])];

      if (files.length > 0) {
        onFilesRef.current(files);
      }
    };

    // A drag that ends anywhere (Esc, dropped outside the window) clears the overlay.
    const onEnd = () => {
      depth = 0;
      setRect(null);
    };

    pane.addEventListener("dragenter", onEnter);
    pane.addEventListener("dragover", onOver);
    pane.addEventListener("dragleave", onLeave);
    pane.addEventListener("drop", onDrop);
    window.addEventListener("dragend", onEnd);
    window.addEventListener("drop", onEnd);

    return () => {
      pane.removeEventListener("dragenter", onEnter);
      pane.removeEventListener("dragover", onOver);
      pane.removeEventListener("dragleave", onLeave);
      pane.removeEventListener("drop", onDrop);
      window.removeEventListener("dragend", onEnd);
      window.removeEventListener("drop", onEnd);
    };
  }, [anchorRef, enabled]);

  return { active: rect !== null, rect };
}

interface DropOverlayProps {
  readonly target: DropTarget;
  /** "Drop to share in #general". */
  readonly label: string;
}

/**
 * A soft accent wash over the pane with a dashed inner frame and a lifted card, fading in as the
 * drag arrives. It ignores the pointer, so the drop lands on the pane underneath.
 */
export function DropOverlay({ target, label }: DropOverlayProps) {
  const presence = usePresence<HTMLDivElement>(target.active);
  const [rect, setRect] = useState<DOMRect | null>(null);

  if (target.rect !== null && target.rect !== rect) {
    setRect(target.rect);
  }

  if (!presence.mounted || rect === null) {
    return null;
  }

  return createPortal(
    <div
      ref={presence.ref}
      className="drop-overlay"
      data-state={presence.state}
      style={{ top: rect.top, left: rect.left, width: rect.width, height: rect.height }}
      aria-hidden="true"
    >
      <div className="drop-overlay-frame">
        <div className="drop-overlay-card">
          <span className="drop-overlay-icon">
            <Icon name="cloud-upload" size={28} />
          </span>
          <span className="drop-overlay-title">Drop files to upload</span>
          <span className="drop-overlay-label">{label}</span>
        </div>
      </div>
    </div>,
    document.body,
  );
}
