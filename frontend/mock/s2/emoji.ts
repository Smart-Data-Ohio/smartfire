/**
 * Emoji, brand and workspace icons: what counts as a reaction, its tooltip, and the `:name:`
 * lookups behind icon autocomplete. Emoji come from emojibase-data (the gemoji aliases are its
 * GitHub shortcodes); the brand and workspace icons are a small made-up set the Vite plugin
 * draws as SVGs.
 */
import emojiData from "emojibase-data/en/compact.json" with { type: "json" };
import githubShortcodes from "emojibase-data/en/shortcodes/github.json" with { type: "json" };
import type { Icon } from "../../src/gen/Icon.ts";

/** `EmojiHelper::REACTIONS`: the quick reactions, as character and title, in order. */
export const QUICK_REACTIONS: readonly (readonly [string, string])[] = [
  ["👍", "Thumbs up"],
  ["👏", "Clapping"],
  ["👋", "Waving hand"],
  ["💪", "Muscle"],
  ["❤️", "Red heart"],
  ["😂", "Face with tears of joy"],
  ["🎉", "Party popper"],
  ["🔥", "Fire"],
];

/** A brand or workspace icon. */
interface ImageIcon {
  readonly name: string;
  readonly title: string;
}

/** Built-in brand logos (a few of `vendor/icons.yml`), served at `/assets/icons/brands/:name.svg` as the server's asset
 * pipeline does (crates/app/src/icons.rs, less the digest). */
export const BRAND_ICONS: readonly ImageIcon[] = [
  { name: "anthropic", title: "Anthropic" },
  { name: "claude", title: "Claude" },
  { name: "docker", title: "Docker" },
  { name: "figma", title: "Figma" },
  { name: "github", title: "GitHub" },
  { name: "linear", title: "Linear" },
  { name: "openai", title: "OpenAI" },
  { name: "slack", title: "Slack" },
];

/** The workspace's uploaded icons, served at `/icons/:name`. */
export const CUSTOM_ICONS: readonly ImageIcon[] = [
  { name: "lgtm", title: "LGTM" },
  { name: "ohio", title: "Ohio" },
  { name: "partyparrot", title: "Party parrot" },
  { name: "shipit", title: "Ship it" },
  { name: "smartfire", title: "Smartfire" },
];

/** The image URL of a brand or workspace icon. */
export function iconImageUrl(kind: "brand" | "custom", name: string): string {
  return kind === "brand" ? `/assets/icons/brands/${name}.svg` : `/icons/${name}`;
}

const VARIATION_SELECTOR = /️/g;

const SKIN_TONE = /[\u{1F3FB}-\u{1F3FF}]/gu;

interface EmojiIndex {
  /** Every emoji character (skin tones included, variation selectors stripped) to its hexcode. */
  readonly hexcodes: ReadonlyMap<string, string>;
  /** Hexcode to character, in the stored form (see `canonical`). */
  readonly characters: ReadonlyMap<string, string>;
  /** Gemoji alias to character. */
  readonly byAlias: ReadonlyMap<string, string>;
  /** Hexcode to its aliases. */
  readonly aliases: ReadonlyMap<string, readonly string[]>;
  /** Every alias usable as `:name:`, sorted. */
  readonly names: readonly string[];
}

let index: EmojiIndex | null = null;

/**
 * The form gemoji stores: emoji outside the Basic Multilingual Plane already default to emoji
 * presentation, so they go without U+FE0F (👍); older symbols keep it (❤️, #️⃣).
 */
function canonical(unicode: string): string {
  return (unicode.codePointAt(0) ?? 0) >= 0x1f000
    ? unicode.replace(VARIATION_SELECTOR, "")
    : unicode;
}

function emojiIndex(): EmojiIndex {
  if (index !== null) return index;

  const hexcodes = new Map<string, string>();
  const characters = new Map<string, string>();

  for (const emoji of emojiData) {
    hexcodes.set(emoji.unicode.replace(VARIATION_SELECTOR, ""), emoji.hexcode);
    characters.set(emoji.hexcode, canonical(emoji.unicode));

    for (const skin of emoji.skins ?? []) {
      hexcodes.set(skin.unicode.replace(VARIATION_SELECTOR, ""), skin.hexcode);
      characters.set(skin.hexcode, canonical(skin.unicode));
    }
  }

  const byAlias = new Map<string, string>();
  const aliases = new Map<string, readonly string[]>();

  for (const [hexcode, value] of Object.entries(githubShortcodes)) {
    const character = characters.get(hexcode);

    const names = (Array.isArray(value) ? value : [value]).filter((name) =>
      /^[a-z0-9_]+$/.test(name),
    );

    if (character === undefined || names.length === 0) continue;

    aliases.set(hexcode, names);

    for (const name of names) byAlias.set(name, character);
  }

  index = { hexcodes, characters, byAlias, aliases, names: [...byAlias.keys()].sort() };

  return index;
}

/** Whether `content` is a single emoji (skin tones and variation selectors allowed). */
export function isEmoji(content: string): boolean {
  return emojiIndex().hexcodes.has(content.replace(VARIATION_SELECTOR, ""));
}

/** The stored form of an emoji character (`👍️` is stored as `👍`); `null` if it isn't one. */
export function canonicalEmoji(content: string): string | null {
  const { hexcodes, characters } = emojiIndex();
  const hexcode = hexcodes.get(content.replace(VARIATION_SELECTOR, ""));

  return hexcode === undefined ? null : (characters.get(hexcode) ?? null);
}

/** The gemoji alias of an emoji character, e.g. `rocket` for 🚀. */
export function emojiAlias(character: string): string | null {
  const { hexcodes, aliases } = emojiIndex();
  const bare = character.replace(VARIATION_SELECTOR, "");
  const hexcode = hexcodes.get(bare) ?? hexcodes.get(bare.replace(SKIN_TONE, ""));

  return hexcode === undefined ? null : (aliases.get(hexcode)?.[0] ?? null);
}

/** An alias with spaces, capitalised: `party_popper` reads "Party popper". */
function aliasTitle(alias: string): string {
  const spaced = alias.replace(/_/g, " ");

  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

function emojiIcon(name: string, character: string): Icon {
  return { name, title: aliasTitle(name), kind: "emoji", character, imageUrl: null };
}

function imageIcon(kind: "brand" | "custom", icon: ImageIcon): Icon {
  return {
    name: icon.name,
    title: icon.title,
    kind,
    character: null,
    imageUrl: iconImageUrl(kind, icon.name),
  };
}

/** `Icons::lookup`: brand, then custom, then emoji. */
export function lookupIcon(name: string): Icon | null {
  const brand = BRAND_ICONS.find((icon) => icon.name === name);

  if (brand !== undefined) return imageIcon("brand", brand);

  const custom = CUSTOM_ICONS.find((icon) => icon.name === name);

  if (custom !== undefined) return imageIcon("custom", custom);

  const character = emojiIndex().byAlias.get(name);

  return character === undefined ? null : emojiIcon(name, character);
}

/** `GET /icons`: every brand and workspace icon, by kind then name. */
export function imageIcons(): Icon[] {
  const byName = (a: ImageIcon, b: ImageIcon) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0);

  return [
    ...[...BRAND_ICONS].sort(byName).map((icon) => imageIcon("brand", icon)),
    ...[...CUSTOM_ICONS].sort(byName).map((icon) => imageIcon("custom", icon)),
  ];
}

/** At most this many icon suggestions. */
export const ICON_SUGGESTIONS = 8;

/**
 * `autocompletable/icons`: exact, then prefix, then substring matches; non-emoji before emoji;
 * then by name. Empty for an empty query.
 */
export function searchIcons(rawQuery: string): Icon[] {
  const query = rawQuery.trim().replace(/^:|:$/g, "").toLowerCase();

  if (query === "") return [];

  const candidates: Icon[] = imageIcons();

  for (const name of emojiIndex().names) {
    if (!name.includes(query)) continue;

    const character = emojiIndex().byAlias.get(name);

    if (character !== undefined) candidates.push(emojiIcon(name, character));
  }

  const rank = (icon: Icon) =>
    icon.name === query ? 0 : icon.name.startsWith(query) ? 1 : icon.name.includes(query) ? 2 : 3;

  return candidates
    .filter((icon) => rank(icon) < 3)
    .sort(
      (a, b) =>
        rank(a) - rank(b) ||
        Number(a.kind === "emoji") - Number(b.kind === "emoji") ||
        (a.name < b.name ? -1 : a.name > b.name ? 1 : 0),
    )
    .slice(0, ICON_SUGGESTIONS);
}

/** What a reaction stores and shows. */
export interface ReactionContent {
  /** The canonical content: the emoji character, or `:name:` for an icon. */
  readonly content: string;
  readonly title: string;
  readonly imageUrl: string | null;
}

/**
 * `Boost.reaction?` and `resolve_content`: a single emoji or a known `:shortcode:` is a
 * reaction (shortcodes of emoji are stored as the emoji); anything else is a free-text boost
 * (`null`).
 */
export function reactionContent(raw: string): ReactionContent | null {
  const content = raw.trim();
  const shortcode = /^:([a-z0-9_]+):$/.exec(content);

  if (shortcode !== null) {
    const icon = lookupIcon(shortcode[1] ?? "");

    if (icon === null) return null;

    if (icon.character !== null) return emojiReaction(icon.character);

    return { content: `:${icon.name}:`, title: icon.title, imageUrl: icon.imageUrl };
  }

  const character = canonicalEmoji(content);

  return character === null ? null : emojiReaction(character);
}

function emojiReaction(character: string): ReactionContent {
  const quick = QUICK_REACTIONS.find(([emoji]) => emoji === character);

  return {
    content: character,
    title: quick?.[1] ?? emojiAlias(character) ?? character,
    imageUrl: null,
  };
}
