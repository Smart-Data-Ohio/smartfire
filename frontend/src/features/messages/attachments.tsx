import { Suspense, useState } from "react";
import type { Attachment } from "../../gen/Attachment.ts";
import { lazyForUpdate as lazy } from "../../service-worker/lazy.ts";
import type { PendingAttachment } from "../../store/model.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { FileSpoiler, SpoilerCover } from "./file-spoiler.tsx";
import { download, fileIcon, fileKind, fitWithin, formatBytes } from "./format.ts";

/** The lightbox loads the first time someone opens an image. */
const Lightbox = lazy(() => import("./lightbox.tsx"));

/** The largest an inline image or video is drawn (Slack's limits, about). */
const MEDIA_MAX = { width: 400, height: 300 } as const;

interface LightboxState {
  /** The lightbox has been opened once, so its chunk is loaded and it stays mounted. */
  readonly opened: boolean;
  readonly open: boolean;
  readonly index: number;
}

const CLOSED: LightboxState = { opened: false, open: false, index: 0 };

/** The lightbox over `images`, mounted from the first open on (its close animates out). */
function MessageLightbox({
  images,
  state,
  onChange,
}: {
  readonly images: readonly Attachment[];
  readonly state: LightboxState;
  readonly onChange: (state: LightboxState) => void;
}) {
  if (!state.opened) {
    return null;
  }

  return (
    <Suspense fallback={null}>
      <Lightbox
        images={images}
        index={state.index}
        onIndexChange={(index) => onChange({ ...state, index })}
        open={state.open}
        onOpenChange={(open) => onChange({ ...state, open })}
      />
    </Suspense>
  );
}

/**
 * The smallest box an inline image is drawn in (messages.css repeats it for images of unknown
 * size). It holds the action bubble, and keeps the bubble's top below the message bar, which covers
 * the top of the row. A smaller image sits at its own size inside the box.
 */
const IMAGE_FLOOR = 80;

/**
 * An image that opens the lightbox. Its action bubble sits beside the button, not inside it,
 * because a button can't hold another.
 */
function ImageFrame({
  attachment,
  tile,
  onOpen,
}: {
  readonly attachment: Attachment;
  readonly tile: boolean;
  readonly onOpen: () => void;
}) {
  const [loaded, setLoaded] = useState(false);
  const fitted = tile ? null : fitWithin(attachment.width, attachment.height, MEDIA_MAX);

  const box =
    fitted === null
      ? null
      : {
          width: Math.max(fitted.width, IMAGE_FLOOR),
          height: Math.max(fitted.height, IMAGE_FLOOR),
        };

  return (
    <div
      className={tile ? "attachment-tile attachment-image" : "attachment-image"}
      data-loaded={loaded || undefined}
      style={
        box === null ? undefined : { width: box.width, aspectRatio: `${box.width} / ${box.height}` }
      }
    >
      <button
        type="button"
        className="attachment-image-open"
        aria-label={`Open ${attachment.filename}`}
        onClick={onOpen}
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
      <div className="attachment-image-actions">
        <IconButton
          icon="download"
          label={`Download ${attachment.filename}`}
          size="sm"
          tooltipPlacement="bottom"
          onClick={() => download(attachment.downloadUrl, attachment.filename)}
        />
        <IconButton
          icon="external-link"
          label={`Open original ${attachment.filename}`}
          size="sm"
          tooltipPlacement="bottom"
          onClick={() => window.open(attachment.url, "_blank", "noopener")}
        />
      </div>
    </div>
  );
}

function ImageAttachment({ attachment }: { readonly attachment: Attachment }) {
  const [lightbox, setLightbox] = useState(CLOSED);

  return (
    <>
      <ImageFrame
        attachment={attachment}
        tile={false}
        onOpen={() => setLightbox({ opened: true, open: true, index: 0 })}
      />
      <MessageLightbox images={[attachment]} state={lightbox} onChange={setLightbox} />
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

function GalleryVideo({ attachment }: { readonly attachment: Attachment }) {
  return (
    // biome-ignore lint/a11y/useMediaCaption: user uploads carry no caption track
    <video
      className="attachment-tile attachment-video"
      src={attachment.url}
      poster={attachment.thumbnailUrl ?? undefined}
      controls
      preload="metadata"
      aria-label={attachment.filename}
    />
  );
}

/** Two and four tiles go two across (a pair, a square); everything else three across. */
function galleryColumns(count: number): number {
  return count === 2 || count === 4 ? 2 : 3;
}

/**
 * A message's files. One file (every legacy message) draws as it always has; several draw as a
 * gallery: images and videos tile a grid, the lightbox steps through the images, and other
 * files list underneath.
 */
export function AttachmentGallery({
  attachments,
}: {
  readonly attachments: readonly Attachment[];
}) {
  const [lightbox, setLightbox] = useState(CLOSED);
  const [revealed, setRevealed] = useState<ReadonlySet<number>>(new Set());
  const media = attachments.filter((attachment) => attachment.preview !== "file");
  const files = attachments.filter((attachment) => attachment.preview === "file");

  const images = media.filter(
    (attachment, index) =>
      attachment.preview === "image" && (!attachment.spoiler || revealed.has(index)),
  );

  if (attachments.length <= 1 || media.length <= 1) {
    return attachments.map((attachment, index) => (
      // biome-ignore lint/suspicious/noArrayIndexKey: attachment-id order is stable, including shared blobs
      <AttachmentView key={`${index}:${attachment.url}`} attachment={attachment} />
    ));
  }

  return (
    <div className="attachment-gallery">
      <ul
        className="attachment-grid"
        data-columns={galleryColumns(media.length)}
        aria-label={`${media.length} images and videos`}
      >
        {media.map((attachment, index) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: attachment-id order is stable, including shared blobs
          <li key={`${index}:${attachment.url}`} className="attachment-cell">
            {attachment.spoiler && !revealed.has(index) ? (
              <SpoilerCover
                tile
                onReveal={() => setRevealed((current) => new Set([...current, index]))}
              />
            ) : attachment.preview === "image" ? (
              <ImageFrame
                attachment={attachment}
                tile
                onOpen={() =>
                  setLightbox({ opened: true, open: true, index: images.indexOf(attachment) })
                }
              />
            ) : (
              <GalleryVideo attachment={attachment} />
            )}
          </li>
        ))}
      </ul>
      {files.map((attachment, index) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: attachment-id order is stable, including shared blobs
        <AttachmentView key={`${index}:${attachment.url}`} attachment={attachment} />
      ))}
      <MessageLightbox images={images} state={lightbox} onChange={setLightbox} />
    </div>
  );
}

/** A message's one file, drawn by its preview kind. */
export function AttachmentView({ attachment }: { readonly attachment: Attachment }) {
  return (
    <FileSpoiler spoiler={attachment.spoiler}>
      <RevealedAttachment attachment={attachment} />
    </FileSpoiler>
  );
}

function RevealedAttachment({ attachment }: { readonly attachment: Attachment }) {
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
export function PendingAttachmentView({
  attachment,
  tile = false,
}: {
  readonly attachment: PendingAttachment;
  /** Drawn as a square gallery tile. */
  readonly tile?: boolean;
}) {
  return (
    <FileSpoiler spoiler={attachment.spoiler ?? false} tile={tile}>
      <RevealedPendingAttachment attachment={attachment} tile={tile} />
    </FileSpoiler>
  );
}

function RevealedPendingAttachment({
  attachment,
  tile,
}: {
  readonly attachment: PendingAttachment;
  readonly tile: boolean;
}) {
  const [broken, setBroken] = useState(false);
  const image = attachment.contentType.startsWith("image/");

  if (image && attachment.previewUrl !== null && !broken) {
    return (
      <div
        className={`attachment-image attachment-pending${tile ? " attachment-tile" : ""}`}
        data-loaded
      >
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

/** Whether a pending file has a local image preview to tile. */
function previewable(attachment: PendingAttachment): boolean {
  return attachment.contentType.startsWith("image/") && attachment.previewUrl !== null;
}

/** The files of a grouped message on its way, laid out as its gallery will be. */
export function PendingGallery({
  attachments,
}: {
  readonly attachments: readonly PendingAttachment[];
}) {
  const images = attachments.filter(previewable);
  const others = attachments.filter((attachment) => !previewable(attachment));

  if (images.length <= 1) {
    return attachments.map((attachment, index) => (
      // biome-ignore lint/suspicious/noArrayIndexKey: the pending list never reorders
      <PendingAttachmentView key={index} attachment={attachment} />
    ));
  }

  return (
    <div className="attachment-gallery">
      <ul
        className="attachment-grid"
        data-columns={galleryColumns(images.length)}
        aria-label={`${images.length} images`}
      >
        {images.map((attachment, index) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: the pending list never reorders
          <li key={index} className="attachment-cell">
            <PendingAttachmentView attachment={attachment} tile />
          </li>
        ))}
      </ul>
      {others.map((attachment, index) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: the pending list never reorders
        <PendingAttachmentView key={index} attachment={attachment} />
      ))}
    </div>
  );
}
