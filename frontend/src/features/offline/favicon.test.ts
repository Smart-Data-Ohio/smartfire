// @vitest-environment node

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const read = (path: string) => readFileSync(new URL(path, import.meta.url));

describe("the offline page's icons", () => {
  it.each(["favicon.svg", "favicon.png", "apple-touch-icon.png"])(
    "%s is the server's mark, which crates/spa/favicon/render.py copies here",
    (name) => {
      expect(read(`./${name}`).equals(read(`../../../../crates/spa/favicon/${name}`))).toBe(true);
    },
  );

  it("links each copy as the shell links the server's", () => {
    expect(read("../../../offline.html").toString()).toContain(
      [
        '<link rel="icon" type="image/png" sizes="32x32" href="/src/features/offline/favicon.png" />',
        '<link rel="icon" type="image/svg+xml" href="/src/features/offline/favicon.svg" />',
        '<link rel="apple-touch-icon" href="/src/features/offline/apple-touch-icon.png" />',
      ].join("\n    "),
    );
  });
});
