import type { GifItem } from "../../src/gen/GifItem.ts";

const decode = (encoded: string) =>
  Uint8Array.from(atob(encoded), (character) => character.charCodeAt(0));

export const GIF_BYTES = decode(
  "R0lGODlhAgADAIAAAAAAAP///yH5BAAKAAAALAAAAAACAAMAAAIFBEEQBFEAIfkEAAoAAAAsAAAAAAIAAwAAAgUMwzAMUwA7",
);

export const GIF_STILL_BYTES = decode(
  "iVBORw0KGgoAAAANSUhEUgAAAAIAAAADCAYAAAC56t6BAAAAEElEQVR4nGNgYGD4D8VoDABH0wX7kHb2kgAAAABJRU5ErkJggg==",
);

const mediaPath = (slug: string, extension: string) =>
  `/api/v1/gifs/media?u=${encodeURIComponent(`https://static.klipy.com/mock/${slug}.${extension}`)}`;

export const GIF_FIXTURES: readonly GifItem[] = [
  { slug: "hello-wave", klipyId: "9007199254740993", title: "Hello Wave" },
  { slug: "celebration-cheer", klipyId: "9007199254740994", title: "Celebration Cheer" },
].map((item) => ({
  ...item,
  attribution: "Powered by KLIPY",
  previewUrl: mediaPath(item.slug, "gif"),
  stillUrl: mediaPath(item.slug, "png"),
  width: 2,
  height: 3,
}));
