// @vitest-environment node
import { describe, expect, it } from "vitest";
import { PREVIEW_POLICY, previewPolicy } from "./preview-policy.ts";

const INLINE = '\n      document.documentElement.dataset.theme = "dark";\n    ';

describe("the preview server's content security policy", () => {
  it("allows each inline script by the hash of its exact text", () => {
    const page = `<head><script>${INLINE}</script><script type="module" src="/app/assets/index-abcd1234.js"></script></head>`;

    expect(previewPolicy([page])).toContain(
      "script-src 'self' 'wasm-unsafe-eval' 'sha256-moyVHVXUnglmiARexqUMpHF2ztchXHd/hq163Oonq60=';",
    );
  });

  it("allows nothing beyond its own origin for pages whose scripts are all files", () => {
    const page = '<script type="module" src="/app/assets/offline-abcd1234.js"></script>';

    expect(previewPolicy([page])).toBe(PREVIEW_POLICY);
  });

  it("lists a script both pages share once, and never allows inline scripts wholesale", () => {
    const page = `<script>${INLINE}</script>`;
    const policy = previewPolicy([page, page]);

    const scripts = policy.split("; ").find((directive) => directive.startsWith("script-src"));

    expect(policy.match(/'sha256-/g)).toHaveLength(1);
    expect(scripts).not.toContain("unsafe-inline");
  });
});
