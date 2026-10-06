import type { BotAvatarDrawConfig } from "bot-avatars";
import * as bots from "bot-avatars";

/** One data URL per seed, size, theme and pixel ratio, for the life of the page. */
const cache = new Map<string, Promise<string>>();

function hashString(value: string): number {
  let hash = 2166136261;

  for (const character of value) {
    hash ^= character.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16777619);
  }

  return hash >>> 0;
}

function render(seed: string, size: number, theme: "light" | "dark", dpr: number): string {
  const type = bots.botAvatarTypes[hashString(seed) % bots.botAvatarTypes.length] ?? "clover";
  const preset = bots.botAvatarPresets[type];
  // The toy fills 80 % of the tile (90 % at sidebar sizes, where every pixel counts); the library
  // draws into a canvas OVERSCAN times the toy, with the toy RISE of its size below centre (room
  // for hats and bounce).
  const box = size * (size <= 24 ? 0.9 : 0.8);
  const path = new Path2D(bots.botAvatarShapes[type]);
  const partsSource = bots.botAvatarParts[type];
  const work = document.createElement("canvas");
  const side = Math.round(box * bots.BOT_AVATAR_OVERSCAN * dpr);

  work.width = side;
  work.height = side;

  const context = work.getContext("2d");

  if (context === null) {
    return "";
  }

  bots.warmBotAvatarPlastic(type, path, box * dpr);
  context.setTransform(dpr, 0, 0, dpr, 0, 0);

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

  if (partsSource !== undefined) {
    config.parts = partsSource
      .split(/(?=M)/)
      .filter((part) => part.trim() !== "")
      .map((part) => new Path2D(part));
  }

  bots.drawBotAvatarFrame(context, box, bots.restPose("default"), config);

  const tile = document.createElement("canvas");
  const tileSide = Math.round(size * dpr);

  tile.width = tileSide;
  tile.height = tileSide;

  const tileContext = tile.getContext("2d");

  if (tileContext === null) {
    return "";
  }

  const inset = (size - box) / 2;
  const spill = (bots.BOT_AVATAR_OVERSCAN - 1) / 2;

  tileContext.drawImage(
    work,
    (inset - spill * box) * dpr,
    (inset - (spill + bots.BOT_AVATAR_RISE) * box) * dpr,
  );

  return tile.toDataURL("image/png");
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

/** The agent's bot, drawn once off the main path and cached as a PNG data URL. */
export function botBitmap(seed: string, size: number, theme: "light" | "dark"): Promise<string> {
  const dpr = Math.min(2, window.devicePixelRatio || 1);
  const key = `${seed}|${size}|${theme}|${dpr}`;
  const cached = cache.get(key);

  if (cached !== undefined) {
    return cached;
  }

  const bitmap = whenIdle().then(() => render(seed, size, theme, dpr));

  cache.set(key, bitmap);

  return bitmap;
}
