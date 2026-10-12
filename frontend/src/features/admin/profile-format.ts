/** The images a workspace profile takes, as the server checks them. */
export const PROFILE_IMAGE_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp"] as const;

/** The largest workspace profile image the server accepts. */
export const PROFILE_IMAGE_MAX_BYTES = 10 * 1024 * 1024;

export type ProfileImageKind = "logo" | "banner" | "invite_splash";

/** What each slot is called and the size it asks for. */
export const PROFILE_SLOTS: Readonly<
  Record<ProfileImageKind, { readonly title: string; readonly noun: string; readonly hint: string }>
> = {
  invite_splash: {
    title: "Invite splash",
    noun: "invite splash",
    hint: "16:9, ideally 1920 × 1080, at most 4096 × 2304. A still background on join and About pages; phones crop the sides.",
  },
  logo: {
    title: "Icon",
    noun: "icon",
    hint: "Square is best, at least 512 × 512. Shown whole, never cropped to a circle.",
  },
  banner: {
    title: "Banner",
    noun: "banner",
    hint: "16:9, at least 960 × 540. Shown behind the workspace name; narrow sidebars crop the sides.",
  },
};

/** A person's own profile banner, in their settings. */
export const USER_BANNER_SLOT = {
  title: "Profile banner",
  noun: "banner",
  hint: "Wide, at least 960 × 320. Shown across the top of your profile; the sides crop on phones.",
} as const;

/** The file types and size, said once under the slots. */
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
