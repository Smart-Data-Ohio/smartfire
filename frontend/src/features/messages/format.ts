import type { IconName } from "../../ui/icons/icon.tsx";

const UNITS = ["KB", "MB", "GB", "TB"] as const;

/** "512 bytes", "1.2 MB", "34 MB": one decimal below 10, none above (1 KB = 1024 bytes). */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} ${bytes === 1 ? "byte" : "bytes"}`;
  }

  let value = bytes / 1024;
  let unit = 0;

  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }

  const rounded = value < 10 ? Math.round(value * 10) / 10 : Math.round(value);

  return `${rounded} ${UNITS[unit]}`;
}

/** The file card's icon for a MIME type. */
export function fileIcon(contentType: string): IconName {
  const type = contentType.toLowerCase();

  if (type.startsWith("image/")) return "image";

  if (type.startsWith("video/")) return "film";

  if (type.startsWith("audio/")) return "music";

  if (/zip|tar|gzip|x-7z|rar|compressed/.test(type)) return "file-archive";

  if (type === "application/pdf" || type.startsWith("text/") || /word|document|rtf/.test(type)) {
    return "file-text";
  }

  return "file";
}

/** A short type label for the card: "PDF", "ZIP", "PNG", else the file's extension. */
export function fileKind(filename: string, contentType: string): string {
  const extension = /\.([a-z0-9]{1,8})$/i.exec(filename)?.[1];

  if (extension !== undefined) {
    return extension.toUpperCase();
  }

  const subtype = contentType.split("/")[1] ?? "";

  return subtype === "" || subtype === "octet-stream" ? "File" : subtype.toUpperCase();
}

/** Display size for an image or video: its own size, scaled down to fit `max` and never up. */
export function fitWithin(
  width: number | null,
  height: number | null,
  max: { readonly width: number; readonly height: number },
): { readonly width: number; readonly height: number } | null {
  if (width === null || height === null || width <= 0 || height <= 0) {
    return null;
  }

  const scale = Math.min(1, max.width / width, max.height / height);

  return { width: Math.round(width * scale), height: Math.round(height * scale) };
}

/** Downloads through a throwaway link, so the browser saves instead of navigating. */
export function download(url: string, filename: string): void {
  const link = document.createElement("a");

  link.href = url;
  link.download = filename;
  link.rel = "noopener";
  document.body.append(link);
  link.click();
  link.remove();
}
