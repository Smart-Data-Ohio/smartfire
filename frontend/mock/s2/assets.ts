/**
 * The files the seed attaches (a chart, a design mockup, a board deck, a source file), the
 * icon images, and reading pixel sizes out of image headers. Everything is drawn in code and
 * cached per process, so the mock needs no binary fixtures.
 */
import { adler32, crc32 } from "./digest.ts";

const encoder = new TextEncoder();

/** UTF-8 bytes of a string. */
export function utf8(text: string): Uint8Array {
  return encoder.encode(text);
}

function memo(build: () => Uint8Array): () => Uint8Array {
  let cached: Uint8Array | null = null;

  return () => {
    cached ??= build();

    return cached;
  };
}

// --- the #general chart: weekly signups, an SVG with real text ---

const WEEKS = [
  ["Jul 6", 182],
  ["Jul 13", 204],
  ["Jul 20", 199],
  ["Jul 27", 236],
  ["Aug 3", 251],
  ["Aug 10", 247],
  ["Aug 17", 289],
  ["Aug 24", 312],
  ["Aug 31", 305],
  ["Sep 7", 348],
  ["Sep 14", 371],
  ["Sep 21", 402],
  ["Sep 28", 436],
] as const;

/** `signups-by-week.svg`: 1200×675. */
export const signupsChartSvg = memo(() => {
  const [width, height] = [1200, 675];
  const [left, top, right, bottom] = [96, 150, 1140, 585];
  const max = 500;
  const step = (right - left) / WEEKS.length;
  const y = (value: number) => bottom - ((bottom - top) * value) / max;
  const parts: string[] = [];

  for (let value = 0; value <= max; value += 100) {
    parts.push(
      `<line x1="${left}" x2="${right}" y1="${y(value)}" y2="${y(value)}" stroke="#e7e5e4" stroke-width="2"/>`,
      `<text x="${left - 16}" y="${y(value) + 6}" text-anchor="end" font-size="18" fill="#78716c">${value}</text>`,
    );
  }

  WEEKS.forEach(([label, value], position) => {
    const x = left + position * step + step * 0.18;
    const barWidth = step * 0.64;
    const latest = position === WEEKS.length - 1;

    parts.push(
      `<rect x="${x.toFixed(1)}" y="${y(value).toFixed(1)}" width="${barWidth.toFixed(1)}" height="${(bottom - y(value)).toFixed(1)}" rx="8" fill="${latest ? "url(#hot)" : "url(#warm)"}"/>`,
      `<text x="${(x + barWidth / 2).toFixed(1)}" y="${bottom + 32}" text-anchor="middle" font-size="16" fill="#78716c">${label}</text>`,
    );

    if (latest) {
      parts.push(
        `<text x="${(x + barWidth / 2).toFixed(1)}" y="${(y(value) - 14).toFixed(1)}" text-anchor="middle" font-size="20" font-weight="700" fill="#c2410c">${value}</text>`,
      );
    }
  });

  return utf8(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" font-family="Inter, system-ui, sans-serif">` +
      `<defs><linearGradient id="warm" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fdba74"/><stop offset="1" stop-color="#fed7aa"/></linearGradient>` +
      `<linearGradient id="hot" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#f97316"/><stop offset="1" stop-color="#ea580c"/></linearGradient></defs>` +
      `<rect width="${width}" height="${height}" rx="24" fill="#fffbf7"/>` +
      `<text x="${left}" y="72" font-size="34" font-weight="700" fill="#1c1917">Weekly signups</text>` +
      `<text x="${left}" y="108" font-size="20" fill="#78716c">Self-serve workspaces, July to September · up 140% over the quarter</text>` +
      `${parts.join("")}</svg>`,
  );
});

// --- the active thread's funnel, a smaller SVG ---

/** `signup-funnel.svg`: 800×480. */
export const funnelSvg = memo(() => {
  const stages = [
    ["Visited pricing", 12_480, "#7c3aed"],
    ["Started signup", 3_912, "#8b5cf6"],
    ["Verified email", 2_655, "#a78bfa"],
    ["Invited a teammate", 1_204, "#c4b5fd"],
  ] as const;

  const rows = stages
    .map(([label, value, color], position) => {
      const barWidth = Math.round((620 * value) / stages[0][1]);
      const top = 110 + position * 88;

      return (
        `<rect x="150" y="${top}" width="${barWidth}" height="60" rx="10" fill="${color}"/>` +
        `<text x="134" y="${top + 38}" text-anchor="end" font-size="18" fill="#44403c">${label}</text>` +
        `<text x="${160 + barWidth}" y="${top + 38}" font-size="18" font-weight="700" fill="#1c1917">${value.toLocaleString("en-US")}</text>`
      );
    })
    .join("");

  return utf8(
    `<svg xmlns="http://www.w3.org/2000/svg" width="800" height="480" viewBox="0 0 800 480" font-family="Inter, system-ui, sans-serif">` +
      `<rect width="800" height="480" rx="20" fill="#faf5ff"/>` +
      `<text x="40" y="62" font-size="28" font-weight="700" fill="#1c1917">Signup funnel, last 30 days</text>${rows}</svg>`,
  );
});

// --- the #design mockup: a palette PNG drawn pixel by pixel ---

const PALETTE = [
  [0xf5, 0xf3, 0xef], // 0 canvas
  [0xff, 0xff, 0xff], // 1 white
  [0x1f, 0x29, 0x37], // 2 sidebar
  [0xff, 0x6b, 0x35], // 3 accent
  [0xe5, 0xe7, 0xeb], // 4 hairline
  [0x9c, 0xa3, 0xaf], // 5 muted text
  [0x37, 0x41, 0x51], // 6 text
  [0xff, 0xd8, 0xc7], // 7 accent wash
  [0x2c, 0xb6, 0x7d], // 8 green
  [0x7f, 0x5a, 0xf0], // 9 violet
  [0x37, 0x41, 0x55], // 10 sidebar row
  [0xde, 0xd6, 0xfb], // 11 violet wash
] as const;

class Canvas {
  readonly width: number;
  readonly height: number;
  readonly pixels: Uint8Array;

  constructor(width: number, height: number) {
    this.width = width;
    this.height = height;
    this.pixels = new Uint8Array(width * height);
  }

  rect(x: number, y: number, w: number, h: number, color: number, radius = 0) {
    for (let row = Math.max(0, y); row < Math.min(this.height, y + h); row++) {
      for (let col = Math.max(0, x); col < Math.min(this.width, x + w); col++) {
        if (radius > 0 && !this.insideCorner(col - x, row - y, w, h, radius)) continue;
        this.pixels[row * this.width + col] = color;
      }
    }
  }

  circle(cx: number, cy: number, r: number, color: number) {
    for (let row = Math.max(0, cy - r); row <= Math.min(this.height - 1, cy + r); row++) {
      for (let col = Math.max(0, cx - r); col <= Math.min(this.width - 1, cx + r); col++) {
        if ((col - cx) ** 2 + (row - cy) ** 2 <= r * r) this.pixels[row * this.width + col] = color;
      }
    }
  }

  private insideCorner(dx: number, dy: number, w: number, h: number, r: number): boolean {
    const cx = dx < r ? r : dx >= w - r ? w - r - 1 : dx;
    const cy = dy < r ? r : dy >= h - r ? h - r - 1 : dy;

    return (dx - cx) ** 2 + (dy - cy) ** 2 <= r * r;
  }
}

function chunk(type: string, data: Uint8Array): Uint8Array {
  const out = new Uint8Array(12 + data.length);
  const view = new DataView(out.buffer);

  view.setUint32(0, data.length);
  out.set(utf8(type), 4);
  out.set(data, 8);
  view.setUint32(8 + data.length, crc32(out.subarray(4, 8 + data.length)));

  return out;
}

/** A zlib stream of stored (uncompressed) deflate blocks. */
function zlibStored(raw: Uint8Array): Uint8Array {
  const blocks = Math.max(1, Math.ceil(raw.length / 65_535));
  const out = new Uint8Array(2 + raw.length + 5 * blocks + 4);
  const view = new DataView(out.buffer);
  let at = 2;

  out[0] = 0x78;
  out[1] = 0x01;

  for (let block = 0; block < blocks; block++) {
    const part = raw.subarray(block * 65_535, Math.min(raw.length, (block + 1) * 65_535));

    out[at] = block === blocks - 1 ? 1 : 0;
    view.setUint16(at + 1, part.length, true);
    view.setUint16(at + 3, ~part.length & 0xffff, true);
    out.set(part, at + 5);
    at += 5 + part.length;
  }

  view.setUint32(at, adler32(raw));

  return out;
}

/** An 8-bit palette PNG of the canvas. */
function encodePng(canvas: Canvas): Uint8Array {
  const header = new Uint8Array(13);
  const headerView = new DataView(header.buffer);

  headerView.setUint32(0, canvas.width);
  headerView.setUint32(4, canvas.height);
  header.set([8, 3, 0, 0, 0], 8);

  const raw = new Uint8Array(canvas.height * (canvas.width + 1));

  for (let row = 0; row < canvas.height; row++) {
    raw.set(
      canvas.pixels.subarray(row * canvas.width, (row + 1) * canvas.width),
      row * (canvas.width + 1) + 1,
    );
  }

  const parts = [
    new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("PLTE", new Uint8Array(PALETTE.flat())),
    chunk("IDAT", zlibStored(raw)),
    chunk("IEND", new Uint8Array()),
  ];

  const out = new Uint8Array(parts.reduce((total, part) => total + part.length, 0));
  let at = 0;

  for (const part of parts) {
    out.set(part, at);
    at += part.length;
  }

  return out;
}

/** The palette entries of the mockup's three progress steps: done, current, to do. */
const STEP_COLORS = [3, 7, 4] as const;

/** `onboarding-v3.png`: 1200×750, the new onboarding screen as a flat wireframe. */
export const onboardingMockupPng = memo(() => {
  const canvas = new Canvas(1200, 750);

  canvas.rect(0, 0, 1200, 750, 0);
  canvas.rect(24, 24, 1152, 702, 1, 18);
  canvas.rect(24, 24, 260, 702, 2, 18);
  canvas.rect(266, 24, 18, 702, 2);

  for (let row = 0; row < 9; row++) {
    canvas.rect(
      52,
      110 + row * 52,
      row === 2 ? 204 : 150 + ((row * 37) % 50),
      22,
      row === 2 ? 3 : 10,
      8,
    );
  }

  canvas.circle(68, 66, 18, 3);
  canvas.rect(96, 56, 120, 20, 10, 8);
  canvas.rect(284, 24, 892, 72, 1);
  canvas.rect(284, 95, 892, 2, 4);
  canvas.rect(316, 46, 220, 26, 6, 8);

  [0, 1, 2].forEach((step) => {
    canvas.circle(530 + step * 120, 160, 18, STEP_COLORS[step] ?? 4);

    if (step < 2) canvas.rect(552 + step * 120, 158, 76, 4, step === 0 ? 3 : 4);
  });

  canvas.rect(380, 220, 520, 420, 1, 20);
  canvas.rect(380, 220, 520, 420, 4, 20);
  canvas.rect(382, 222, 516, 416, 1, 19);
  canvas.rect(420, 262, 300, 30, 6, 10);
  canvas.rect(420, 306, 420, 16, 5, 8);
  canvas.rect(420, 332, 360, 16, 5, 8);

  [0, 1].forEach((field) => {
    canvas.rect(420, 380 + field * 84, 100, 14, 5, 6);
    canvas.rect(420, 402 + field * 84, 440, 44, 4, 10);
    canvas.rect(422, 404 + field * 84, 436, 40, 1, 9);
    canvas.rect(438, 418 + field * 84, 180 + field * 60, 12, 4, 6);
  });

  canvas.rect(420, 570, 440, 48, 3, 12);
  canvas.rect(580, 588, 120, 12, 1, 6);
  canvas.rect(940, 220, 200, 200, 11, 24);
  canvas.circle(1040, 300, 52, 9);
  canvas.circle(1040, 300, 26, 1);
  canvas.rect(940, 440, 200, 120, 7, 24);
  canvas.circle(990, 500, 22, 8);
  canvas.rect(1024, 486, 90, 12, 1, 6);
  canvas.rect(1024, 508, 60, 10, 1, 5);

  return encodePng(canvas);
});

// --- the board deck: a one-page PDF ---

/** `Q3-board-update.pdf`: a valid one-page PDF, padded to a realistic size. */
export const boardDeckPdf = memo(() => {
  const lines = [
    ["F2 28", "Q3 Board Update"],
    ["F1 14", "Smart Data - prepared for the October board meeting"],
    ["F1 12", ""],
    ["F2 16", "Highlights"],
    ["F1 12", "- Self-serve signups up 140% over the quarter (436 in the last week)"],
    ["F1 12", "- Smartfire now runs on the Rust backend in production"],
    ["F1 12", "- Net revenue retention at 118%; two enterprise pilots converted"],
    ["F1 12", ""],
    ["F2 16", "Asks"],
    ["F1 12", "- Approve the Q4 hiring plan (two engineers, one designer)"],
    ["F1 12", "- Feedback on the self-hosted pricing tier"],
  ] as const;

  const text = lines
    .map(([font, line], position) => `BT /${font} Tf 72 ${720 - position * 28} Td (${line}) Tj ET`)
    .join("\n");

  const filler = Array.from({ length: 2900 }, (_, row) =>
    `% ${((row * 2_654_435_761) >>> 0).toString(16).padStart(8, "0")}`.padEnd(63, "."),
  ).join("\n");

  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> /Contents 6 0 R >>",
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>",
    `<< /Length ${text.length} >>\nstream\n${text}\nendstream`,
    `<< /Length ${filler.length} >>\nstream\n${filler}\nendstream`,
  ];

  let body = "%PDF-1.4\n";
  const offsets: number[] = [];

  objects.forEach((object, position) => {
    offsets.push(body.length);
    body += `${position + 1} 0 obj\n${object}\nendobj\n`;
  });

  const xref = body.length;

  const entries = offsets
    .map((offset) => `${String(offset).padStart(10, "0")} 00000 n \n`)
    .join("");

  body += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n${entries}`;
  body += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;

  return utf8(body);
});

// --- the #engineering source file ---

/** `rate_limiter.rs`. */
export const rateLimiterSource = memo(() =>
  utf8(`//! A token bucket per account, refilled continuously.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How many requests an account may burst, and how fast the bucket refills.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub burst: u32,
    pub per_second: f64,
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    updated: Instant,
}

/// Decides whether a request may go ahead.
#[derive(Debug)]
pub struct RateLimiter {
    limits: Limits,
    buckets: HashMap<i64, Bucket>,
}

impl RateLimiter {
    pub fn new(limits: Limits) -> Self {
        Self { limits, buckets: HashMap::new() }
    }

    /// Takes one token for \`account_id\`; \`Err\` says how long to wait.
    pub fn check(&mut self, account_id: i64, now: Instant) -> Result<(), Duration> {
        let limits = self.limits;
        let bucket = self.buckets.entry(account_id).or_insert(Bucket {
            tokens: f64::from(limits.burst),
            updated: now,
        });

        let elapsed = now.saturating_duration_since(bucket.updated).as_secs_f64();

        bucket.tokens = (bucket.tokens + elapsed * limits.per_second).min(f64::from(limits.burst));
        bucket.updated = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            return Ok(());
        }

        let missing = 1.0 - bucket.tokens;
        Err(Duration::from_secs_f64(missing / limits.per_second))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_after_the_burst_and_recovers() {
        let mut limiter = RateLimiter::new(Limits { burst: 2, per_second: 1.0 });
        let start = Instant::now();

        assert!(limiter.check(1, start).is_ok());
        assert!(limiter.check(1, start).is_ok());
        assert!(limiter.check(1, start).is_err());
        assert!(limiter.check(1, start + Duration::from_secs(1)).is_ok());
    }
}
`),
);

// --- icons ---

/** Each icon's background and ink colours. */
const ICON_COLORS = new Map<string, readonly [string, string]>([
  ["anthropic", ["#d4a27f", "#191919"]],
  ["claude", ["#d97757", "#ffffff"]],
  ["docker", ["#2496ed", "#ffffff"]],
  ["figma", ["#a259ff", "#ffffff"]],
  ["github", ["#181717", "#ffffff"]],
  ["linear", ["#5e6ad2", "#ffffff"]],
  ["openai", ["#10a37f", "#ffffff"]],
  ["slack", ["#4a154b", "#ffffff"]],
  ["lgtm", ["#2cb67d", "#ffffff"]],
  ["ohio", ["#bb0000", "#ffffff"]],
  ["partyparrot", ["#ff6b35", "#ffffff"]],
  ["shipit", ["#0f766e", "#ffffff"]],
  ["smartfire", ["#f97316", "#fff7ed"]],
]);

/** What a workspace icon's badge says (brands show their initial). */
const ICON_GLYPHS = new Map([
  ["lgtm", "LGTM"],
  ["shipit", "🚢"],
  ["partyparrot", "🦜"],
  ["smartfire", "🔥"],
  ["ohio", "OH"],
]);

/** A 64×64 badge for a brand or workspace icon. */
export function iconSvg(name: string): Uint8Array {
  const [fill, ink] = ICON_COLORS.get(name) ?? ["#57534e", "#ffffff"];
  const glyph = ICON_GLYPHS.get(name) ?? name.charAt(0).toUpperCase();
  const size = [...glyph].length > 2 ? 18 : [...glyph].length > 1 ? 24 : 32;

  return utf8(
    `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><rect width="64" height="64" rx="14" fill="${fill}"/><text x="32" y="${32 + size * 0.36}" text-anchor="middle" font-family="Inter, system-ui, sans-serif" font-weight="800" font-size="${size}" fill="${ink}">${glyph}</text></svg>`,
  );
}

// --- image sizes ---

/** Pixel size from a PNG, GIF, JPEG or SVG header; `null` for anything else or unreadable. */
export function imageSize(
  bytes: Uint8Array,
  contentType: string,
): readonly [number, number] | null {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);

  if (bytes.length >= 24 && bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4e) {
    return [view.getUint32(16), view.getUint32(20)];
  }

  if (bytes.length >= 10 && bytes[0] === 0x47 && bytes[1] === 0x49 && bytes[2] === 0x46) {
    return [view.getUint16(6, true), view.getUint16(8, true)];
  }

  if (bytes.length >= 4 && bytes[0] === 0xff && bytes[1] === 0xd8) return jpegSize(view);

  if (contentType === "image/svg+xml")
    return svgSize(new TextDecoder().decode(bytes.subarray(0, 4096)));

  return null;
}

function jpegSize(view: DataView): readonly [number, number] | null {
  let at = 2;

  while (at + 9 < view.byteLength) {
    if (view.getUint8(at) !== 0xff) return null;

    const marker = view.getUint8(at + 1);
    const length = view.getUint16(at + 2);
    const startOfFrame = marker >= 0xc0 && marker <= 0xcf && ![0xc4, 0xc8, 0xcc].includes(marker);

    if (startOfFrame) return [view.getUint16(at + 7), view.getUint16(at + 5)];

    at += 2 + length;
  }

  return null;
}

function svgSize(text: string): readonly [number, number] | null {
  const tag = /<svg\b[^>]*>/i.exec(text)?.[0];

  if (tag === undefined) return null;

  const attribute = (name: string) => {
    const value = new RegExp(`\\s${name}\\s*=\\s*["']\\s*([\\d.]+)(px)?\\s*["']`, "i").exec(tag);

    return value === null ? null : Math.round(Number(value[1]));
  };

  const [width, height] = [attribute("width"), attribute("height")];

  if (width !== null && height !== null) return [width, height];

  const viewBox = /\sviewBox\s*=\s*["']\s*[-\d.]+[\s,]+[-\d.]+[\s,]+([\d.]+)[\s,]+([\d.]+)/i.exec(
    tag,
  );

  return viewBox === null ? null : [Math.round(Number(viewBox[1])), Math.round(Number(viewBox[2]))];
}
