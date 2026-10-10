export const DESCRIPTION_MAX_CHARS = 300;

const RESERVED_SLUGS = new Set([
  "app",
  "api",
  "rooms",
  "users",
  "session",
  "sessions",
  "join",
  "assets",
  "admin",
  "account",
  "accounts",
  "settings",
  "about",
  "privacy",
  "terms",
  "up",
  "health",
  "cable",
  "huddle",
  "uploads",
  "files",
  "blobs",
  "storage",
  "messages",
  "threads",
  "events",
  "boards",
  "bots",
  "agents",
  "integrations",
  "first_run",
  "first-run",
  "sudo",
  "two_factor",
  "two-factor",
  "two_factor_setup",
  "two-factor-setup",
  "pwa",
  "manifest",
  "service-worker",
  "favicon",
  "robots",
  "rails",
]);

export function descriptionError(value: string): string | undefined {
  const description = value.trim();

  if ([...description].length > DESCRIPTION_MAX_CHARS) return "Must be 300 characters or fewer.";

  const control = [...description].some(
    (character) =>
      /\p{Cc}/u.test(character) && character !== "\n" && character !== "\r" && character !== "\t",
  );

  if (/[<>]/u.test(description) || control) {
    return "Must be plain text without HTML or control characters.";
  }

  return undefined;
}

export function vanitySlugError(value: string): string | undefined {
  const slug = value.trim();

  if (slug === "") return undefined;

  if (!/^[a-z0-9][a-z0-9-]{1,30}[a-z0-9]$/.test(slug)) {
    return "Use 3–32 lowercase letters, digits or hyphens, without a leading or trailing hyphen.";
  }

  if (RESERVED_SLUGS.has(slug)) return "This slug is reserved. Choose another.";

  return undefined;
}
