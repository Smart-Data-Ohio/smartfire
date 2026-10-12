import type { ScheduledAttachment } from "../../gen/ScheduledAttachment.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { FileSpoiler } from "../messages/file-spoiler.tsx";

export function ScheduledFiles({
  files,
  onRemove,
  onToggleSpoiler,
}: {
  readonly files: readonly ScheduledAttachment[];
  readonly onRemove?: (signedId: string) => void;
  readonly onToggleSpoiler?: (signedId: string) => void;
}) {
  if (files.length === 0) return null;

  return (
    <ul className="scheduled-files" aria-label="Scheduled attachments">
      {files.map(({ attachment: file, signedId }) => (
        <li key={signedId} className="scheduled-file">
          <FileSpoiler spoiler={file.spoiler}>
            {file.preview === "image" || file.thumbnailUrl !== null ? (
              <img
                src={file.thumbnailUrl ?? file.url}
                alt={file.filename}
                className="scheduled-file-thumb"
              />
            ) : (
              <Icon name="file" size={24} />
            )}
          </FileSpoiler>
          <span className="scheduled-file-name">{file.filename}</span>
          {onToggleSpoiler === undefined ? null : (
            <button
              type="button"
              aria-label={`Mark ${file.filename} as spoiler`}
              aria-pressed={file.spoiler}
              onClick={() => onToggleSpoiler(signedId)}
            >
              {file.spoiler ? "Spoiler" : "Mark as spoiler"}
            </button>
          )}
          {onRemove === undefined ? null : (
            <IconButton
              icon="x"
              label={`Remove ${file.filename}`}
              size="sm"
              onClick={() => onRemove(signedId)}
            />
          )}
        </li>
      ))}
    </ul>
  );
}
