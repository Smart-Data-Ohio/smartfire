import { useEffect, useState } from "react";

/** A GIF or WebP may move; other image types never do. */
export function mayAnimate(file: File): boolean {
  return file.type === "image/gif" || file.type === "image/webp";
}

/**
 * A still of `file`'s first frame as an object URL, while `enabled`: an image bitmap of an animated
 * file is its first frame, drawn once to a canvas. `null` while it's drawn, when disabled, or when
 * the browser can't read the file.
 */
export function useFirstFrame(file: File | null, enabled: boolean): string | null {
  const [still, setStill] = useState<string | null>(null);

  useEffect(() => {
    if (file === null || !enabled) {
      setStill(null);

      return;
    }

    let live = true;
    let url: string | null = null;

    createImageBitmap(file)
      .then(
        (bitmap) =>
          new Promise<Blob | null>((resolve) => {
            const canvas = document.createElement("canvas");

            canvas.width = bitmap.width;
            canvas.height = bitmap.height;
            canvas.getContext("2d")?.drawImage(bitmap, 0, 0);
            bitmap.close();
            canvas.toBlob(resolve, "image/png");
          }),
      )
      .then(
        (blob) => {
          if (!live || blob === null) return;

          url = URL.createObjectURL(blob);
          setStill(url);
        },
        () => {
          if (live) setStill(null);
        },
      );

    return () => {
      live = false;

      if (url !== null) URL.revokeObjectURL(url);
    };
  }, [file, enabled]);

  return still;
}
