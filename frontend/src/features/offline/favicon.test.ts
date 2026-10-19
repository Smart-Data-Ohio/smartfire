// @vitest-environment node

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const read = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");

describe("the offline page's icon", () => {
  it("is the server's mark, which crates/spa/favicon/render.py copies here", () => {
    expect(read("./favicon.svg")).toBe(read("../../../../crates/spa/favicon/favicon.svg"));
    expect(read("../../../offline.html")).toContain(
      '<link rel="icon" type="image/svg+xml" href="/src/features/offline/favicon.svg" />',
    );
  });
});
