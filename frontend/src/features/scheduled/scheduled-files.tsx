import type { ScheduledAttachment } from "../../gen/ScheduledAttachment.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";

export function ScheduledFiles({
  files,
  onRemove,
}: {
  readonly files: readonly ScheduledAttachment[];
  readonly onRemove?: (signedId: string) => void;
}) {
  if (files.length === 0) return null;

  return (
    <ul className="scheduled-files" aria-label="Scheduled attachments">
      {files.map(({ attachment: file, signedId }) => (
        <li key={signedId} className="scheduled-file">
          {file.preview === "image" || file.thumbnailUrl !== null ? (
            <img
              src={file.thumbnailUrl ?? file.url}
              alt={file.filename}
              className="scheduled-file-thumb"
            />
          ) : (
            <Icon name="file" size={24} />
          )}
          <span className="scheduled-file-name">{file.filename}</span>
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
