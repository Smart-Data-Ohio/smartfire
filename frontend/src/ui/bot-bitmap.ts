import type { BotAvatarDrawConfig, BotAvatarType } from "bot-avatars";
import * as bots from "bot-avatars";
import type { BotAvatarSpec } from "../gen/BotAvatarSpec.ts";
import { colorOf, fitOf, wearOf } from "./bot-avatar-spec.ts";

/** One data URL per look, size, theme and pixel ratio, for the life of the page. */
const cache = new Map<string, Promise<string>>();

/** The still poses a bitmap shows: awake, or asleep for an agent that can't act. */
export type BotPose = "default" | "sleeping";

function hashString(value: string): number {
  let hash = 2166136261;

  for (const character of value) {
    hash ^= character.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16777619);
  }

  return hash >>> 0;
}

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));

// The library's vivid push (MIT, Jakub Antalik's bot-avatars 0.2.2, not exported): the colour
// moved along its own hue in OKLab to the most saturated shade the screen can show, deepening by
// at most 15 %. Ported so a bitmap matches the live component colour for colour.
const toLinear = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);

const toGamma = (c: number) => (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);

function toOklab(r: number, g: number, b: number): [number, number, number] {
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);

  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}

function fromOklab(lightness: number, a: number, b: number): [number, number, number] {
  const l = (lightness + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (lightness - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (lightness - 0.0894841775 * a - 1.291485548 * b) ** 3;

  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
}

const inGamut = (rgb: number[]) => rgb.every((c) => c >= -1e-4 && c <= 1 + 1e-4);

export function vivid(color: string, amount: number): string {
  const rgb = bots.parseColor(color);

  if (rgb === null || !(amount > 0)) {
    return color;
  }

  const [lightness, a, b] = toOklab(
    toLinear(rgb[0] / 255),
    toLinear(rgb[1] / 255),
    toLinear(rgb[2] / 255),
  );

  const chroma = Math.hypot(a, b);

  if (chroma < 1e-4) {
    return color;
  }

  const hueA = a / chroma;
  const hueB = b / chroma;
  const target = chroma * (1 + 0.5 * amount);
  let best: [number, number, number] | null = null;
  let reached = chroma;

  for (let step = 0; step <= 6; step++) {
    const deeper = lightness * (1 - (0.15 * amount * step) / 6);
    let low = 0;
    let high = target;

    for (let round = 0; round < 18; round++) {
      const mid = (low + high) / 2;

      if (inGamut(fromOklab(deeper, hueA * mid, hueB * mid))) {
        low = mid;
      } else {
        high = mid;
      }
    }

    if (low > reached + 4e-3 * step) {
      reached = low;
      best = fromOklab(deeper, hueA * low, hueB * low);
    }

    if (low >= target - 1e-4) {
      break;
    }
  }

  if (best === null) {
    return color;
  }

  const [r, g, b2] = best.map((c) => Math.round(255 * toGamma(clamp(c, 0, 1))));

  return `rgb(${r} ${g} ${b2})`;
}

/** The library component's defaults for fabric, the shading it draws when given none. */
const FABRIC = { shadow: 1.15, highlight: 1.45, light: 295, rim: 0.6, spread: 1.6, depth: 0.65 };

const FUR = { length: 1, density: 1.6, fuzz: 0.9, clumps: 0.4, curl: 0.7, gravity: 0.9 };

const ACCESSORY_COLOR = "#27272b";

function partsOf(type: BotAvatarType): Path2D[] | undefined {
  const source = bots.botAvatarParts[type];

  return source === undefined
    ? undefined
    : source
        .split(/(?=M)/)
        .filter((part) => part.trim() !== "")
        .map((part) => new Path2D(part));
}

/** A generated agent's look: its seed's body in its own colour, plastic, as before. */
function seedConfig(seed: string, box: number, dpr: number, theme: "light" | "dark") {
  const type = bots.botAvatarTypes[hashString(seed) % bots.botAvatarTypes.length] ?? "clover";
  const preset = bots.botAvatarPresets[type];
  const path = new Path2D(bots.botAvatarShapes[type]);

  bots.warmBotAvatarPlastic(type, path, box * dpr);

  const config: BotAvatarDrawConfig = {
    path,
    face: preset.face,
    faceX: preset.faceX,
    faceY: preset.faceY,
    faceScale: preset.faceScale,
    color: preset.color,
    ink: bots.autoInk(preset.color),
    shading: "plastic",
    typeKey: type,
    dpr,
    theme,
    still: true,
  };

  const parts = partsOf(type);

  if (parts !== undefined) {
    config.parts = parts;
  }

  return config;
}

/**
 * A picked avatar's look, configured as the library's `<BotAvatar>` configures itself for the
 * same props: the type's brightness and saturation applied to the colour, fabric shading.
 */
function specConfig(spec: BotAvatarSpec, box: number, dpr: number, theme: "light" | "dark") {
  const preset = bots.botAvatarPresets[spec.type];
  const brightness = preset.brightness ?? 1;
  const saturation = preset.saturation ?? 1.5;
  const push = clamp(saturation, 1.5, 2.5) - 1.5;
  const color = colorOf(spec);

  const shaded =
    brightness === 1 && saturation === 1
      ? color
      : bots.shade(
          color,
          (clamp(brightness, 0, 2) - 1) * 0.35,
          (clamp(saturation, 0, 1.5) - 1) * 0.5,
        );

  const tint = vivid(shaded, push);
  const path = new Path2D(bots.botAvatarShapes[spec.type]);

  bots.warmBotAvatarPlastic(spec.type, path, box * dpr, FABRIC.depth, true, FUR, FABRIC.light);

  const config: BotAvatarDrawConfig = {
    path,
    face: spec.face,
    faceX: preset.faceX,
    faceY: preset.faceY,
    faceScale: preset.faceScale,
    color: tint,
    ink: bots.autoInk(tint),
    shading: "fabric",
    shadow: FABRIC.shadow,
    highlight: FABRIC.highlight,
    depth: FABRIC.depth,
    roundness: 1,
    fur: FUR,
    vivid: push,
    light: FABRIC.light,
    rim: FABRIC.rim,
    spread: FABRIC.spread,
    typeKey: spec.type,
    still: true,
    furOnIdle: false,
    dpr,
    theme,
    wear: { ...wearOf(spec.accessory), color: ACCESSORY_COLOR, fabric: true },
  };

  const parts = partsOf(spec.type);

  if (parts !== undefined) {
    config.parts = parts;
  }

  return config;
}

type Look = { kind: "seed"; seed: string } | { kind: "spec"; spec: BotAvatarSpec };

/**
 * The toy drawn into a `size` square: it fills 80 % of it (90 % at sidebar sizes, where every
 * pixel counts; less, and lower, under a hat — see `fitOf`); the library draws into a canvas OVERSCAN times the toy, with the toy RISE of its
 * size below centre (room for hats and bounce), cropped back to the square here.
 */
function draw(look: Look, size: number, theme: "light" | "dark", dpr: number, pose: BotPose) {
  const fit = look.kind === "spec" ? fitOf(look.spec.accessory, size) : fitOf(null, size);
  const box = size * fit.scale;
  const work = document.createElement("canvas");
  const side = Math.round(box * bots.BOT_AVATAR_OVERSCAN * dpr);

  work.width = side;
  work.height = side;

  const context = work.getContext("2d");

  if (context === null) {
    return null;
  }

  context.setTransform(dpr, 0, 0, dpr, 0, 0);

  const config =
    look.kind === "seed"
      ? seedConfig(look.seed, box, dpr, theme)
      : specConfig(look.spec, box, dpr, theme);

  bots.drawBotAvatarFrame(context, box, bots.restPose(pose), config);

  const tile = document.createElement("canvas");
  const tileSide = Math.round(size * dpr);

  tile.width = tileSide;
  tile.height = tileSide;

  const tileContext = tile.getContext("2d");

  if (tileContext === null) {
    return null;
  }

  const inset = (size - box) / 2;
  const spill = (bots.BOT_AVATAR_OVERSCAN - 1) / 2;

  tileContext.drawImage(
    work,
    (inset - spill * box) * dpr,
    (inset + fit.drop * size - (spill + bots.BOT_AVATAR_RISE) * box) * dpr,
  );

  return tile;
}

function whenIdle(): Promise<void> {
  return new Promise((resolve) => {
    if ("requestIdleCallback" in window) {
      window.requestIdleCallback(() => resolve(), { timeout: 500 });
    } else {
      setTimeout(resolve, 0);
    }
  });
}

function cached(key: string, look: Look, size: number, theme: "light" | "dark", pose: BotPose) {
  const dpr = Math.min(2, window.devicePixelRatio || 1);
  const full = `${key}|${size}|${theme}|${dpr}|${pose}`;
  const hit = cache.get(full);

  if (hit !== undefined) {
    return hit;
  }

  const bitmap = whenIdle().then(
    () => draw(look, size, theme, dpr, pose)?.toDataURL("image/png") ?? "",
  );

  cache.set(full, bitmap);

  return bitmap;
}

/** The agent's generated bot, drawn once off the main path and cached as a PNG data URL. */
export function botBitmap(
  seed: string,
  size: number,
  theme: "light" | "dark",
  pose: BotPose = "default",
): Promise<string> {
  return cached(`seed:${seed}`, { kind: "seed", seed }, size, theme, pose);
}

/** A picked avatar, drawn once off the main path and cached as a PNG data URL. */
export function specBitmap(
  spec: BotAvatarSpec,
  size: number,
  theme: "light" | "dark",
  pose: BotPose = "default",
): Promise<string> {
  const key = `spec:${spec.type}|${colorOf(spec)}|${spec.face}|${spec.accessory ?? ""}`;

  return cached(key, { kind: "spec", spec }, size, theme, pose);
}

/**
 * A picked avatar's still pose as a transparent PNG, `side` pixels square: the picture a bot
 * keeps for every surface that can't draw it live (notifications, the API, link previews).
 */
export function specPng(spec: BotAvatarSpec, side = 256): Promise<Blob | null> {
  const tile = draw({ kind: "spec", spec }, side, "light", 1, "default");

  return new Promise((resolve) => {
    if (tile === null) {
      resolve(null);
    } else {
      tile.toBlob(resolve, "image/png");
    }
  });
}
