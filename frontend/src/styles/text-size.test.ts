// @vitest-environment node

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { TEXT_SIZE_CHOICES } from "../features/settings/settings-format.ts";

const typography = readFileSync(new URL("./typography.css", import.meta.url), "utf8");

const indexHtml = readFileSync(new URL("../../index.html", import.meta.url), "utf8");

/** The root font-size `typography.css` gives `<html data-text-size="size">`, in percent. */
function rootPercent(size: string): number {
  if (size === "default") {
    return 100;
  }

  const rule = new RegExp(
    `:root\\[data-text-size="${size}"\\]\\s*\\{\\s*font-size:\\s*([\\d.]+)%;`,
  );

  const match = typography.match(rule);

  if (match === null) {
    throw new Error(`no root font-size for the "${size}" text size`);
  }

  return Number(match[1]);
}

describe("text sizes", () => {
  it("grow in the order Settings lists them, up to at least half as big again", () => {
    const percents = TEXT_SIZE_CHOICES.map((choice) => rootPercent(choice.value));

    expect(percents).toEqual([...percents].sort((a, b) => a - b));
    expect(new Set(percents).size).toBe(percents.length);
    expect(percents.at(-1)).toBeGreaterThanOrEqual(150);
  });

  it("are all painted by index.html's first-paint script", () => {
    const allowed = indexHtml.match(/root\.dataset\.textSize = (\[[^\]]*\])/)?.[1];
    const painted: unknown = JSON.parse(allowed ?? "[]");

    expect(painted).toEqual(
      TEXT_SIZE_CHOICES.map((choice) => choice.value).filter((size) => size !== "default"),
    );
  });
});
