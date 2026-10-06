import { lazy, Suspense, useState } from "react";
import type { Attachment } from "../../gen/Attachment.ts";
import type { PendingAttachment } from "../../store/model.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { download, fileIcon, fileKind, fitWithin, formatBytes } from "./format.ts";

/** The lightbox loads the first time someone opens an image. */
const Lightbox = lazy(() => import("./lightbox.tsx"));

/** The largest an inline image or video is drawn (Slack's limits, about). */
const MEDIA_MAX = { width: 400, height: 300 } as const;

function ImageAttachment({ attachment }: { readonly attachment: Attachment }) {
  const [open, setOpen] = useState(false);
  const [opened, setOpened] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const size = fitWithin(attachment.width, attachment.height, MEDIA_MAX);

  return (
    <>
      <button
        type="button"
        className="attachment-image"
        data-loaded={loaded || undefined}
        style={
          size === null
            ? undefined
            : { width: size.width, aspectRatio: `${size.width} / ${size.height}` }
        }
        aria-label={`Open ${attachment.filename}`}
        onClick={() => {
          setOpened(true);
          setOpen(true);
        }}
      >
        <img
          src={attachment.thumbnailUrl ?? attachment.url}
          alt={attachment.filename}
          loading="lazy"
          decoding="async"
          draggable={false}
          onLoad={() => setLoaded(true)}
        />
      </button>
      {opened ? (
        <Suspense fallback={null}>
          <Lightbox attachment={attachment} open={open} onOpenChange={setOpen} />
        </Suspense>
      ) : null}
    </>
  );
}

function VideoAttachment({ attachment }: { readonly attachment: Attachment }) {
  const size = fitWithin(attachment.width, attachment.height, MEDIA_MAX);

  return (
    // biome-ignore lint/a11y/useMediaCaption: user uploads carry no caption track
    <video
      className="attachment-video"
      src={attachment.url}
      poster={attachment.thumbnailUrl ?? undefined}
      controls
      preload="metadata"
      style={
        size === null
          ? undefined
          : { width: size.width, aspectRatio: `${size.width} / ${size.height}` }
      }
      aria-label={attachment.filename}
    />
  );
}

/** Any other file: an icon by type, the name (opens it), size and kind, and a download button. */
function FileAttachment({ attachment }: { readonly attachment: Attachment }) {
  const audio = attachment.contentType.startsWith("audio/");

  return (
    <div className="attachment-file" data-audio={audio || undefined}>
      <div className="attachment-file-row">
        <span className="attachment-file-icon" data-kind={fileIcon(attachment.contentType)}>
          <Icon name={fileIcon(attachment.contentType)} size={20} />
        </span>
        <span className="attachment-file-text">
          <a
            className="attachment-file-name"
            href={attachment.url}
            target="_blank"
            rel="noopener noreferrer"
            title={attachment.filename}
          >
            {attachment.filename}
          </a>
          <span className="attachment-file-meta">
            {fileKind(attachment.filename, attachment.contentType)} ·{" "}
            {formatBytes(attachment.byteSize)}
          </span>
        </span>
        <IconButton
          icon="download"
          label={`Download ${attachment.filename}`}
          size="sm"
          onClick={() => download(attachment.downloadUrl, attachment.filename)}
        />
      </div>
      {audio ? (
        // biome-ignore lint/a11y/useMediaCaption: user uploads carry no caption track
        <audio className="attachment-audio" src={attachment.url} controls preload="none" />
      ) : null}
    </div>
  );
}

/** A message's one file, drawn by its preview kind. */
export function AttachmentView({ attachment }: { readonly attachment: Attachment }) {
  switch (attachment.preview) {
    case "image":
      return <ImageAttachment attachment={attachment} />;
    case "video":
      return <VideoAttachment attachment={attachment} />;
    case "file":
      return <FileAttachment attachment={attachment} />;
  }
}

/**
 * The file a message on its way carries: the local preview for an image (until its URL is
 * released), otherwise an inert chip with the name, kind and size.
 */
export function PendingAttachmentView({ attachment }: { readonly attachment: PendingAttachment }) {
  const [broken, setBroken] = useState(false);
  const image = attachment.contentType.startsWith("image/");

  if (image && attachment.previewUrl !== null && !broken) {
    return (
      <div className="attachment-image attachment-pending" data-loaded>
        <img
          src={attachment.previewUrl}
          alt={attachment.filename}
          decoding="async"
          draggable={false}
          onError={() => setBroken(true)}
        />
      </div>
    );
  }

  return (
    <div className="attachment-file attachment-pending">
      <div className="attachment-file-row">
        <span className="attachment-file-icon" data-kind={fileIcon(attachment.contentType)}>
          <Icon name={fileIcon(attachment.contentType)} size={20} />
        </span>
        <span className="attachment-file-text">
          <span className="attachment-file-name" title={attachment.filename}>
            {attachment.filename}
          </span>
          <span className="attachment-file-meta">
            {fileKind(attachment.filename, attachment.contentType)} ·{" "}
            {formatBytes(attachment.byteSize)}
          </span>
        </span>
      </div>
    </div>
  );
}
