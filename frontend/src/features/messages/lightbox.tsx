import { useEffect } from "react";
import type { Attachment } from "../../gen/Attachment.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { download, formatBytes } from "./format.ts";

interface LightboxProps {
  /** The message's images, in order; the lightbox steps between them. */
  readonly images: readonly Attachment[];
  /** Which image is showing. */
  readonly index: number;
  readonly onIndexChange: (index: number) => void;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

/**
 * An image at full size in a modal, with its name, size, and download and open actions. A message
 * with several images gets previous and next buttons and the arrow keys, wrapping at the ends.
 */
export default function Lightbox({
  images,
  index,
  onIndexChange,
  open,
  onOpenChange,
}: LightboxProps) {
  const count = images.length;
  const current = Math.min(Math.max(index, 0), count - 1);
  const attachment = images[current];
  const several = count > 1;

  const step = (by: number) => onIndexChange((current + by + count) % count);

  useEffect(() => {
    if (!open || count < 2) {
      return;
    }

    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || event.metaKey || event.ctrlKey) {
        return;
      }

      if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
        event.preventDefault();
        onIndexChange((current + (event.key === "ArrowLeft" ? -1 : 1) + count) % count);
      }
    };

    document.addEventListener("keydown", onKey);

    return () => document.removeEventListener("keydown", onKey);
  }, [open, count, current, onIndexChange]);

  if (attachment === undefined) {
    return null;
  }

  const dimensions =
    attachment.width === null || attachment.height === null
      ? null
      : `${attachment.width} × ${attachment.height}`;

  const position = several ? `${current + 1} of ${count}` : null;

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={attachment.filename}
      description={[position, dimensions, formatBytes(attachment.byteSize)]
        .filter((part) => part !== null)
        .join(" · ")}
      footer={
        <>
          <Button
            variant="ghost"
            icon="external-link"
            onClick={() => window.open(attachment.url, "_blank", "noopener")}
          >
            Open original
          </Button>
          <Button
            variant="primary"
            icon="download"
            onClick={() => download(attachment.downloadUrl, attachment.filename)}
          >
            Download
          </Button>
        </>
      }
    >
      <div className="lightbox" data-several={several || undefined}>
        <img
          key={attachment.url}
          className="lightbox-image"
          src={attachment.url}
          alt={attachment.filename}
          width={attachment.width ?? undefined}
          height={attachment.height ?? undefined}
          draggable={false}
        />
        {several ? (
          <>
            <IconButton
              icon="chevron-left"
              label="Previous image"
              className="lightbox-step"
              data-side="previous"
              onClick={() => step(-1)}
            />
            <IconButton
              icon="chevron-right"
              label="Next image"
              className="lightbox-step"
              data-side="next"
              onClick={() => step(1)}
            />
          </>
        ) : null}
      </div>
    </Dialog>
  );
}
