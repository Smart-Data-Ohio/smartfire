/**
 * Hues for people's initials tiles, in OKLCH degrees. Every tile in a theme shares one lightness
 * and chroma (avatar.css), so no one's tile shouts louder than another's. Two bands are left out:
 * violet (roughly 260-330) belongs to agents, so a person can never be mistaken for one, and
 * yellow-olive (roughly 70-130) turns muddy at the mid-tone lightness the dark theme uses.
 */
export const AVATAR_HUES = [350, 22, 55, 140, 165, 190, 215, 240] as const;

/** Coprime with the palette size, so consecutive numeric ids walk the whole palette, 3 apart. */
const ID_STEP = 3;

function fnv1a(value: string): number {
  let hash = 2166136261;

  for (const character of value) {
    hash ^= character.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16777619);
  }

  return hash >>> 0;
}

/** murmur3's finaliser: flips about half the output bits for any one-bit change in the input. */
function mix(value: number): number {
  let hash = value;

  hash ^= hash >>> 16;
  hash = Math.imul(hash, 0x85ebca6b);
  hash ^= hash >>> 13;
  hash = Math.imul(hash, 0xc2b2ae35);
  hash ^= hash >>> 16;

  return hash >>> 0;
}

/**
 * The tile hue for a user. Numeric ids (database ids, the common case) step through the palette
 * so neighbours such as 41 and 42 never share a colour; any other key (a uuid, a name) is hashed
 * with full avalanche, so near-identical strings still land far apart.
 */
export function avatarHue(key: string | number): number {
  const text = String(key);
  const count = AVATAR_HUES.length;

  const index = /^\d{1,15}$/.test(text)
    ? (Number(text) * ID_STEP) % count
    : mix(fnv1a(text)) % count;

  return AVATAR_HUES[index] ?? AVATAR_HUES[0];
}
