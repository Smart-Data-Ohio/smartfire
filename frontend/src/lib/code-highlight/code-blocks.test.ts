import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { COPIED_MS, codeLabel, enhanceCodeBlocks, paintCode } from "./code-blocks.ts";
import type { HighlightResult } from "./kinds.ts";
import { highlight } from "./tokenize.ts";
import { createTokenizer, type Highlight } from "./tokenizer.ts";

/** A body as the server renders a fenced block (crates/richtext): escaped text in `<code>`. */
const RUST = '<p>Look:</p><pre><code class="language-rust">fn main() { let x = 1; }</code></pre>';

const SCRIPT =
  '<pre><code class="language-html">&lt;script&gt;alert("hi")&lt;/script&gt;</code></pre>';

function body(html: string): HTMLElement {
  const root = document.createElement("div");

  root.className = "message-body";
  root.innerHTML = html;
  document.body.append(root);

  return root;
}

/** A tokenizer whose loads and calls are counted, colouring each block's first word a keyword. */
function counted() {
  const calls: string[] = [];

  const load = vi.fn(
    async (): Promise<Highlight> => async (source) => {
      calls.push(source);

      return {
        language: "rust",
        tokens: [{ offset: 0, length: source.indexOf(" "), kind: "keyword" }],
      };
    },
  );

  return { calls, load, tokenizer: createTokenizer(load) };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

afterEach(() => {
  document.body.replaceChildren();
  vi.useRealTimers();
  vi.unstubAllGlobals();

  for (const record of toastSnapshot()) {
    removeToast(record.id);
  }
});

describe("codeLabel", () => {
  it("reads a language- or lang- class from the code or its pre, as classic does", () => {
    const root = body(
      '<pre><code class="language-rust">a</code></pre><pre class="lang-py">b</pre><pre><code>c</code></pre>' +
        '<pre><code class="hljs language-c++ other">d</code></pre>',
    );

    const [rust, python, none, cpp] = [...root.querySelectorAll("pre")];

    expect(rust && codeLabel(rust)).toBe("rust");
    expect(python && codeLabel(python)).toBe("py");
    expect(none && codeLabel(none)).toBeUndefined();
    expect(cpp && codeLabel(cpp)).toBe("c++");
  });
});

describe("enhanceCodeBlocks", () => {
  it("loads no highlighter for a body without a code block", async () => {
    const { load, tokenizer } = counted();

    enhanceCodeBlocks(body("<p>No code here, just <code>inline</code>.</p>"), tokenizer);
    await settle();

    expect(load).not.toHaveBeenCalled();
  });

  it("loads the highlighter once, on the first block, and colours every block", async () => {
    const { calls, load, tokenizer } = counted();
    const root = body(`${RUST}<pre><code>let y = 2;</code></pre>`);

    enhanceCodeBlocks(root, tokenizer);
    await settle();

    expect(load).toHaveBeenCalledTimes(1);
    expect(calls).toEqual(["fn main() { let x = 1; }", "let y = 2;"]);
    expect(
      [...root.querySelectorAll(".code-token--keyword")].map((span) => span.textContent),
    ).toEqual(["fn", "let"]);
    expect(root.querySelector("pre")?.classList.contains("code-highlighted")).toBe(true);
    expect(root.querySelector("code")?.textContent).toBe("fn main() { let x = 1; }");
  });

  it("colours a mounted-again row from the cache before paint, without asking again", async () => {
    const { calls, tokenizer } = counted();

    enhanceCodeBlocks(body(RUST), tokenizer);
    await settle();
    document.body.replaceChildren();

    const again = body(RUST);

    enhanceCodeBlocks(again, tokenizer);

    expect(again.querySelector(".code-token--keyword")?.textContent).toBe("fn");
    expect(calls).toHaveLength(1);
  });

  it("drops a result that arrives after the row was unmounted or its body replaced", async () => {
    let finish: (result: HighlightResult) => void = () => undefined;

    const tokenizer = createTokenizer(
      async () => () => new Promise((resolve) => (finish = resolve)),
    );

    const root = body(RUST);
    const cleanup = enhanceCodeBlocks(root, tokenizer);

    await settle();
    cleanup();
    finish({ language: "rust", tokens: [{ offset: 0, length: 2, kind: "keyword" }] });
    await settle();

    expect(root.querySelector(".code-token")).toBeNull();
  });

  it("leaves a block with markup inside (a link) as the server rendered it", async () => {
    const { load, tokenizer } = counted();
    const root = body('<pre><code>see <a href="/x">this</a></code></pre>');

    enhanceCodeBlocks(root, tokenizer);
    await settle();

    expect(load).not.toHaveBeenCalled();
    expect(root.querySelector("pre a")?.textContent).toBe("this");
  });

  it("keeps a <script> in a code block as text, with the real highlighter", async () => {
    const root = body(SCRIPT);

    enhanceCodeBlocks(
      root,
      createTokenizer(async () => highlight),
    );

    await vi.waitFor(() => expect(root.querySelector(".code-token")).not.toBeNull());

    expect(root.querySelector("script")).toBeNull();
    expect(root.querySelector("code")?.textContent).toBe('<script>alert("hi")</script>');
    expect(root.querySelector("code")?.dataset.codeLanguage).toBe("html");
    expect(
      [...root.querySelectorAll("code *")].every(
        (element) =>
          element.tagName === "SPAN" && /^code-token code-token--[a-z]+$/.test(element.className),
      ),
    ).toBe(true);
  });
});

describe("paintCode", () => {
  it("refuses runs out of order, past the end or of an unknown kind", () => {
    const code = document.createElement("code");
    const source = "let x = 1;";

    code.textContent = source;

    for (const tokens of [
      [
        { offset: 4, length: 1, kind: "variable" },
        { offset: 0, length: 3, kind: "keyword" },
      ],
      [{ offset: 8, length: 9, kind: "number" }],
      [{ offset: 0, length: 3, kind: 'keyword" onclick="x' }],
    ]) {
      // Malformed worker output, typed only as the worker's message would be.
      const malformed: HighlightResult = JSON.parse(JSON.stringify({ language: "rust", tokens }));

      expect(paintCode(code, source, malformed)).toBe(false);
      expect(code.childNodes).toHaveLength(1);
    }
  });
});

describe("the Copy button", () => {
  let writeText: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    writeText = vi.fn(async (_text: string) => undefined);
    vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });
  });

  it("is a real button beside each block, named for screen readers", () => {
    const root = body(RUST);
    const { tokenizer } = counted();

    enhanceCodeBlocks(root, tokenizer);

    const button = root.querySelector<HTMLButtonElement>(".code-block > button.code-copy");

    expect(button?.type).toBe("button");
    expect(button?.textContent).toBe("Copy");
    expect(button?.getAttribute("aria-label")).toBe("Copy code");
    expect(button?.previousElementSibling?.tagName).toBe("PRE");
  });

  it("copies the block's text, says Copied, then goes back", async () => {
    const root = body(RUST);
    const { tokenizer } = counted();

    enhanceCodeBlocks(root, tokenizer);
    await settle();
    vi.useFakeTimers();

    const button = root.querySelector<HTMLButtonElement>("button.code-copy");

    button?.click();
    await vi.advanceTimersByTimeAsync(0);

    expect(writeText).toHaveBeenCalledWith("fn main() { let x = 1; }");
    expect(button?.textContent).toBe("Copied");
    expect(button?.hasAttribute("data-copied")).toBe(true);

    await vi.advanceTimersByTimeAsync(COPIED_MS);

    expect(button?.textContent).toBe("Copy");
    expect(button?.getAttribute("aria-label")).toBe("Copy code");
  });

  it("raises a toast when the clipboard refuses", async () => {
    writeText.mockRejectedValueOnce(new Error("denied"));

    const root = body(RUST);
    const { tokenizer } = counted();

    enhanceCodeBlocks(root, tokenizer);
    root.querySelector<HTMLButtonElement>("button.code-copy")?.click();
    await settle();

    expect(toastSnapshot().map((record) => record.title)).toEqual([
      "Couldn't copy to the clipboard",
    ]);
    expect(root.querySelector("button.code-copy")?.textContent).toBe("Copy");
  });

  it("stops answering clicks after cleanup", async () => {
    const root = body(RUST);
    const { tokenizer } = counted();

    enhanceCodeBlocks(root, tokenizer)();
    root.querySelector<HTMLButtonElement>("button.code-copy")?.click();
    await settle();

    expect(writeText).not.toHaveBeenCalled();
  });
});
