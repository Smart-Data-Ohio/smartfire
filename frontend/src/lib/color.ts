/**
 * OKLCH colour maths: parsing the `oklch()` values the design tokens use, converting them to sRGB
 * and measuring WCAG 2 contrast. The design-token test uses it to hold every text token to 4.5:1,
 * and the wrappers around Jakub Antalik's canvas libraries use it to hand them a token colour in
 * the `rgb()` form those libraries accept.
 */

export interface Oklch {
  readonly l: number;
  readonly c: number;
  readonly h: number;
  readonly alpha: number;
}

export interface Rgb {
  readonly r: number;
  readonly g: number;
  readonly b: number;
}

const OKLCH_RE =
  /^oklch\(\s*([\d.]+)(%?)\s+([\d.]+)\s+([\d.]+)(?:deg)?\s*(?:\/\s*([\d.]+)(%?))?\s*\)$/;

/** Parses `oklch(L% C H)` or `oklch(L C H / A)`; null for anything else. */
export function parseOklch(value: string): Oklch | null {
  const match = OKLCH_RE.exec(value.trim());

  if (match === null) {
    return null;
  }

  const [, lRaw = "0", lPercent, cRaw = "0", hRaw = "0", aRaw, aPercent] = match;
  const l = lPercent === "%" ? Number(lRaw) / 100 : Number(lRaw);
  const alphaRaw = aRaw === undefined ? 1 : Number(aRaw);
  const alpha = aPercent === "%" ? alphaRaw / 100 : alphaRaw;

  return { l, c: Number(cRaw), h: Number(hRaw), alpha };
}

/** Linear-light sRGB (0..1, unclamped) for an OKLCH colour, per CSS Color 4. */
function oklchToLinearSrgb(color: Oklch): Rgb {
  const hue = (color.h * Math.PI) / 180;
  const a = color.c * Math.cos(hue);
  const b = color.c * Math.sin(hue);
  const l1 = color.l + 0.3963377774 * a + 0.2158037573 * b;
  const m1 = color.l - 0.1055613458 * a - 0.0638541728 * b;
  const s1 = color.l - 0.0894841775 * a - 1.291485548 * b;
  const l3 = l1 ** 3;
  const m3 = m1 ** 3;
  const s3 = s1 ** 3;

  return {
    r: 4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3,
    g: -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3,
    b: -0.0041960863 * l3 - 0.7034186147 * m3 + 1.707614701 * s3,
  };
}

function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value));
}

function gammaEncode(channel: number): number {
  const c = clamp01(channel);

  return c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055;
}

/** Whether an OKLCH colour lies inside sRGB (a hair of tolerance for float error). */
export function inGamut(color: Oklch): boolean {
  const linear = oklchToLinearSrgb(color);

  return [linear.r, linear.g, linear.b].every((channel) => channel >= -1e-4 && channel <= 1 + 1e-4);
}

/**
 * The colour with its chroma reduced (lightness and hue kept) until it fits sRGB, so a derived
 * colour paints as computed rather than clipped channel by channel.
 */
export function fitGamut(color: Oklch): Oklch {
  if (inGamut(color)) {
    return color;
  }

  let low = 0;
  let high = color.c;

  for (let step = 0; step < 20; step += 1) {
    const middle = (low + high) / 2;

    if (inGamut({ ...color, c: middle })) {
      low = middle;
    } else {
      high = middle;
    }
  }

  return { ...color, c: low };
}

/** sRGB in 0..255, clipped to the gamut (what a browser paints on an sRGB display). */
export function oklchToRgb(color: Oklch): Rgb {
  const linear = oklchToLinearSrgb(color);

  return {
    r: Math.round(gammaEncode(linear.r) * 255),
    g: Math.round(gammaEncode(linear.g) * 255),
    b: Math.round(gammaEncode(linear.b) * 255),
  };
}

/** `rgb(r, g, b)`: the form the Antalik libraries' `color` props parse. */
export function toRgbString(color: Rgb): string {
  return `rgb(${color.r}, ${color.g}, ${color.b})`;
}

function linearize(channel255: number): number {
  const c = channel255 / 255;

  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

/** WCAG 2 relative luminance of an sRGB colour. */
export function relativeLuminance(color: Rgb): number {
  return 0.2126 * linearize(color.r) + 0.7152 * linearize(color.g) + 0.0722 * linearize(color.b);
}

/** Composites a translucent foreground over an opaque background, in gamma-encoded sRGB. */
export function composite(foreground: Rgb, alpha: number, background: Rgb): Rgb {
  const mix = (front: number, back: number) => Math.round(front * alpha + back * (1 - alpha));

  return {
    r: mix(foreground.r, background.r),
    g: mix(foreground.g, background.g),
    b: mix(foreground.b, background.b),
  };
}

/** WCAG 2 contrast ratio between two opaque sRGB colours (1..21). */
export function contrastRatio(first: Rgb, second: Rgb): number {
  const a = relativeLuminance(first);
  const b = relativeLuminance(second);
  const [light, dark] = a > b ? [a, b] : [b, a];

  return (light + 0.05) / (dark + 0.05);
}

/**
 * Converts a resolved CSS colour token to `rgb()`. OKLCH values are converted; hex and rgb pass
 * through; anything else falls back.
 */
export function tokenToRgbString(value: string, fallback: string): string {
  const trimmed = value.trim();
  const oklch = parseOklch(trimmed);

  if (oklch !== null) {
    return toRgbString(oklchToRgb(oklch));
  }

  if (/^#[\da-f]{3,8}$/i.test(trimmed) || /^rgba?\(/.test(trimmed)) {
    return trimmed;
  }

  return fallback;
}
