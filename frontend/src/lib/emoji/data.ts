import { loadForUpdate } from "../../service-worker/update-required.ts";
/**
 * The emoji catalogue: the repo's gemoji file (`web/app/assets/emoji/emoji.json`, the same
 * shortcodes the server expands), copied here and loaded on demand so the 100 KB never lands in
 * the initial bundle. Each entry is `[character, "alias other_alias", name, "key words"]`.
 */

/** The picker's category ids, in the server's order (`EMOJI_PICKER_TABS`). */
export type EmojiGroupId =
  | "smileys"
  | "people"
  | "nature"
  | "food"
  | "activities"
  | "travel"
  | "objects"
  | "symbols"
  | "flags";

export interface Emoji {
  readonly char: string;
  /** Shortcodes without colons, the first one canonical (`+1`, `thumbsup`). */
  readonly aliases: readonly string[];
  /** The Unicode name, lowercase ("thumbs up"). */
  readonly name: string;
  readonly keywords: readonly string[];
  readonly group: EmojiGroupId;
}

export interface EmojiGroup {
  readonly id: EmojiGroupId;
  readonly label: string;
  /** The tab's glyph. */
  readonly glyph: string;
  readonly emoji: readonly Emoji[];
}

export interface EmojiData {
  readonly groups: readonly EmojiGroup[];
  readonly all: readonly Emoji[];
  readonly byChar: ReadonlyMap<string, Emoji>;
  readonly byAlias: ReadonlyMap<string, Emoji>;
}

/** The file's own layout. */
export interface RawEmojiFile {
  readonly groups: readonly {
    readonly id: string;
    readonly emoji: readonly (readonly string[])[];
  }[];
}

export const GROUP_LABELS: Readonly<Record<EmojiGroupId, { label: string; glyph: string }>> = {
  smileys: { label: "Smileys", glyph: "😀" },
  people: { label: "People", glyph: "👋" },
  nature: { label: "Nature", glyph: "🐻" },
  food: { label: "Food", glyph: "🍔" },
  activities: { label: "Activities", glyph: "⚽" },
  travel: { label: "Travel", glyph: "✈️" },
  objects: { label: "Objects", glyph: "💡" },
  symbols: { label: "Symbols", glyph: "🔣" },
  flags: { label: "Flags", glyph: "🏳️" },
};

function isGroupId(id: string): id is EmojiGroupId {
  return Object.hasOwn(GROUP_LABELS, id);
}

function words(text: string | undefined): string[] {
  return (text ?? "").split(" ").filter((word) => word !== "");
}

/** Builds the lookup tables from the file's layout. Unknown groups and empty rows are skipped. */
export function buildEmojiData(file: RawEmojiFile): EmojiData {
  const groups: EmojiGroup[] = [];
  const byChar = new Map<string, Emoji>();
  const byAlias = new Map<string, Emoji>();

  for (const group of file.groups) {
    if (!isGroupId(group.id)) {
      continue;
    }

    const groupId = group.id;

    const emoji = group.emoji.flatMap(([char, aliases, name, keywords]): Emoji[] =>
      char === undefined || char === ""
        ? []
        : [
            {
              char,
              aliases: words(aliases),
              name: name ?? "",
              keywords: words(keywords),
              group: groupId,
            },
          ],
    );

    for (const entry of emoji) {
      byChar.set(entry.char, entry);

      for (const alias of entry.aliases) {
        byAlias.set(alias, entry);
      }
    }

    groups.push({ id: groupId, ...GROUP_LABELS[groupId], emoji });
  }

  return { groups, all: groups.flatMap((group) => group.emoji), byChar, byAlias };
}

let loading: Promise<EmojiData> | null = null;

/** The catalogue, fetched as its own chunk the first time anything asks, then kept. */
export function loadEmojiData(): Promise<EmojiData> {
  loading ??= loadForUpdate(() => import("./emoji.json")).then(
    (module) => buildEmojiData(module.default),
    (error: Error) => {
      loading = null;

      throw error;
    },
  );

  return loading;
}

/** "thumbs up" -> "Thumbs up": the title the server gives an emoji reaction. */
export function emojiTitle(emoji: Emoji): string {
  return emoji.name.charAt(0).toUpperCase() + emoji.name.slice(1);
}
