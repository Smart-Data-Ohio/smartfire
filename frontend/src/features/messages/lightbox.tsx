import type { Attachment } from "../../gen/Attachment.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { download, formatBytes } from "./format.ts";

interface LightboxProps {
  readonly attachment: Attachment;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

/** An image at full size in a modal, with its name, size, and download and open actions. */
export default function Lightbox({ attachment, open, onOpenChange }: LightboxProps) {
  const dimensions =
    attachment.width === null || attachment.height === null
      ? null
      : `${attachment.width} × ${attachment.height}`;

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={attachment.filename}
      description={[dimensions, formatBytes(attachment.byteSize)]
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
      <div className="lightbox">
        <img
          className="lightbox-image"
          src={attachment.url}
          alt={attachment.filename}
          width={attachment.width ?? undefined}
          height={attachment.height ?? undefined}
          draggable={false}
        />
      </div>
    </Dialog>
  );
}
