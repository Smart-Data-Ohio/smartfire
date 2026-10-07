/** The images a workspace profile takes, as the server checks them. */
export const PROFILE_IMAGE_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp"] as const;

/** The largest logo or banner the server accepts. */
export const PROFILE_IMAGE_MAX_BYTES = 10 * 1024 * 1024;

export type ProfileImageKind = "logo" | "banner";

/** What each slot is called and the size it asks for. */
export const PROFILE_SLOTS: Readonly<
  Record<ProfileImageKind, { readonly title: string; readonly noun: string; readonly hint: string }>
> = {
  logo: {
    title: "Icon",
    noun: "icon",
    hint: "Square, at least 512 × 512. Shown round in the rail, squaring off on hover.",
  },
  banner: {
    title: "Banner",
    noun: "banner",
    hint: "16:9, at least 960 × 540. Shown behind the workspace name; narrow sidebars crop the sides.",
  },
};

/** The file types and size, said once under both slots. */
export const PROFILE_FORMATS =
  "PNG, JPEG, GIF or WebP, up to 10 MB and 4096 pixels wide. GIF and WebP can be animated.";

/** Why `file` can't be a logo or banner, before it's uploaded; `null` when it can. */
export function imageProblem(file: Pick<File, "type" | "size">): string | null {
  if (!PROFILE_IMAGE_TYPES.some((type) => type === file.type)) {
    return "Choose a PNG, JPEG, GIF or WebP image.";
  }

  return file.size > PROFILE_IMAGE_MAX_BYTES ? "Choose an image of 10 MB or less." : null;
}

/** An upload's progress as a whole percentage, for its bar and label. */
export function uploadPercent(loaded: number, total: number): number {
  if (total <= 0) {
    return 0;
  }

  return Math.min(100, Math.round((loaded / total) * 100));
}
