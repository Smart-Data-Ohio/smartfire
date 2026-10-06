import { IconButton } from "../../../ui/icon-button.tsx";
import { Icon } from "../../../ui/icons/icon.tsx";
import type { TrayFile } from "./use-attachments.ts";
import "./attachments.css";

interface AttachmentTrayProps {
  readonly files: readonly TrayFile[];
  readonly onRemove: (id: string) => void;
  readonly onRetry: (id: string) => void;
}

/** "1.2 MB", "820 KB", "12 bytes". */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} ${bytes === 1 ? "byte" : "bytes"}`;
  }

  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;

  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }

  return `${value >= 10 ? Math.round(value) : value.toFixed(1)} ${units[unit]}`;
}

/** "PDF", "ZIP", "PNG": the extension, else the MIME subtype. */
function kindLabel(file: File): string {
  const extension = /\.([a-z0-9]{1,5})$/i.exec(file.name)?.[1];

  return (extension ?? file.type.split("/")[1] ?? "file").toUpperCase();
}

/** 0..1 of the way there; hashing and starting count as a sliver so the ring shows life. */
function progressOf(entry: TrayFile): number {
  const { phase, loaded, total } = entry.snapshot;

  if (phase === "done") {
    return 1;
  }

  if (phase !== "uploading") {
    return 0.04;
  }

  return total > 0 ? Math.min(1, Math.max(0.04, loaded / total)) : 0.04;
}

const RING_RADIUS = 9;

const RING_LENGTH = 2 * Math.PI * RING_RADIUS;

/** A small determinate ring; it spins while the upload hasn't started sending bytes. */
function ProgressRing({ entry }: { readonly entry: TrayFile }) {
  const progress = progressOf(entry);
  const waiting = entry.snapshot.phase === "hashing" || entry.snapshot.phase === "starting";

  return (
    <svg
      className="tray-ring"
      data-waiting={waiting || undefined}
      viewBox="0 0 24 24"
      width={24}
      height={24}
      aria-hidden="true"
    >
      <circle className="tray-ring-track" cx="12" cy="12" r={RING_RADIUS} />
      <circle
        className="tray-ring-value"
        cx="12"
        cy="12"
        r={RING_RADIUS}
        strokeDasharray={RING_LENGTH}
        strokeDashoffset={waiting ? RING_LENGTH * 0.75 : RING_LENGTH * (1 - progress)}
      />
    </svg>
  );
}

function statusText(entry: TrayFile): string {
  const { phase, error } = entry.snapshot;

  switch (phase) {
    case "hashing":
    case "starting":
      return "Preparing…";
    case "uploading":
      return `Uploading ${Math.round(progressOf(entry) * 100)}%`;
    case "failed":
      return error ?? "Upload failed";
    case "cancelled":
      return "Cancelled";
    case "done":
      return `${kindLabel(entry.file)} · ${formatBytes(entry.file.size)}`;
  }
}

/**
 * The files waiting to go with the message: image thumbnails and file cards, each with its
 * upload progress, a remove button and (after a failure) retry. New chips pop in.
 */
export function AttachmentTray({ files, onRemove, onRetry }: AttachmentTrayProps) {
  if (files.length === 0) {
    return null;
  }

  return (
    <ul className="tray" aria-label="Attachments">
      {files.map((entry) => {
        const { phase } = entry.snapshot;
        const busy = phase === "hashing" || phase === "starting" || phase === "uploading";
        const status = statusText(entry);

        return (
          <li
            key={entry.id}
            className="tray-chip"
            data-kind={entry.previewUrl === null ? "file" : "image"}
            data-phase={phase}
            aria-label={`${entry.file.name}: ${status}`}
            aria-busy={busy || undefined}
          >
            {entry.previewUrl === null ? (
              <span className="tray-file-icon" aria-hidden="true">
                {busy ? <ProgressRing entry={entry} /> : <Icon name="file" size={18} />}
              </span>
            ) : (
              <span className="tray-thumb">
                <img src={entry.previewUrl} alt="" />
                {busy ? (
                  <span className="tray-thumb-veil">
                    <ProgressRing entry={entry} />
                  </span>
                ) : null}
              </span>
            )}
            {entry.previewUrl === null ? (
              <span className="tray-file-text">
                <span className="tray-file-name">{entry.file.name}</span>
                <span className="tray-file-status" role={phase === "failed" ? "alert" : undefined}>
                  {status}
                </span>
              </span>
            ) : null}
            {phase === "failed" ? (
              <span className="tray-failed">
                {entry.previewUrl === null ? null : (
                  <span className="tray-failed-badge" role="alert" title={status}>
                    <Icon name="alert" size={12} />
                  </span>
                )}
                <IconButton
                  icon="rotate-ccw"
                  label={`Retry ${entry.file.name}`}
                  size="sm"
                  className="tray-retry"
                  onClick={() => onRetry(entry.id)}
                />
              </span>
            ) : null}
            <IconButton
              icon="x"
              label={busy ? `Cancel ${entry.file.name}` : `Remove ${entry.file.name}`}
              size="sm"
              className="tray-remove"
              onClick={() => onRemove(entry.id)}
            />
          </li>
        );
      })}
    </ul>
  );
}
