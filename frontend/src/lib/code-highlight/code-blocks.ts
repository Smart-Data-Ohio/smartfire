import { toast } from "../../ui/toast-store.ts";
import { type HighlightResult, isTokenKind } from "./kinds.ts";
import { tokenizer as defaultTokenizer, type Tokenizer } from "./tokenizer.ts";

/**
 * Code blocks in a server-rendered message body (classic's models/message_formatter.js and
 * models/code_highlighter.js): each gets a Copy button and is coloured once its tokens arrive.
 *
 * The body's HTML is the server's sanitizer output and is never re-parsed here: the coloured
 * markup is built from the block's own text, every run a text node or a `<span>` whose class is
 * one of the fixed token kinds, so a `<script>` in a code block stays text.
 */

/** A `lang-` or `language-` class on the `<code>` or its `<pre>` (classic's pattern). */
const LABEL = /\blang(?:uage)?-([\w+#.-]+)(?=\s|$)/i;

/** How long the button says "Copied" (classic's 1.6 s). */
export const COPIED_MS = 1600;

const COPY_LABEL = "Copy";

const COPY_NAME = "Copy code";

/** The block's `<code>`, or the `<pre>` itself when a rich-text body has no `<code>` in it. */
function codeOf(pre: HTMLElement): HTMLElement {
  return pre.querySelector<HTMLElement>(":scope > code") ?? pre;
}

/** The block's fence label, as classic reads it; undefined when it has none. */
export function codeLabel(pre: HTMLElement): string | undefined {
  return LABEL.exec(`${codeOf(pre).className} ${pre.className}`)?.[1];
}

/**
 * Replaces `code`'s text with coloured runs of `source`. Refuses (returns false) unless the runs
 * fall in order inside the source, so the block's text is exactly what it was.
 */
export function paintCode(code: HTMLElement, source: string, result: HighlightResult): boolean {
  const fragment = document.createDocumentFragment();
  let offset = 0;

  for (const token of result.tokens) {
    const end = token.offset + token.length;

    if (!isTokenKind(token.kind) || token.offset < offset || end > source.length) {
      return false;
    }

    if (token.offset > offset) {
      fragment.append(source.slice(offset, token.offset));
    }

    const span = document.createElement("span");

    span.className = `code-token code-token--${token.kind}`;
    span.textContent = source.slice(token.offset, end);
    fragment.append(span);
    offset = end;
  }

  if (offset < source.length) {
    fragment.append(source.slice(offset));
  }

  code.replaceChildren(fragment);

  return true;
}

function colour(pre: HTMLElement, tokenizer: Tokenizer, live: () => boolean): void {
  const code = codeOf(pre);

  // Already coloured, or markup inside (links, mentions) that colouring would flatten.
  if (
    code.dataset.highlighted !== undefined ||
    ![...code.childNodes].every((node) => node.nodeType === Node.TEXT_NODE)
  ) {
    return;
  }

  const source = code.textContent;

  if (source.trim() === "") {
    return;
  }

  const label = codeLabel(pre);

  const apply = (result: HighlightResult) => {
    // The row may have been unmounted, or its body replaced by an edit, while the work ran.
    if (!live() || !code.isConnected || code.textContent !== source) {
      return;
    }

    code.dataset.highlighted = "";
    code.dataset.codeLanguage = result.language;

    if (result.tokens.length > 0 && paintCode(code, source, result)) {
      pre.classList.add("code-highlighted");
    }
  };

  const cached = tokenizer.cached(source, label);

  if (cached === undefined) {
    // A block that can't be coloured stays plain; its Copy button still works.
    tokenizer.tokenize(source, label).then(apply, () => undefined);
  } else {
    apply(cached);
  }
}

/** Wraps the block so its Copy button sits over the corner, not inside the scrolling `<pre>`. */
function addCopyButton(pre: HTMLElement): void {
  if (pre.parentElement?.classList.contains("code-block") ?? false) {
    return;
  }

  const block = document.createElement("div");
  const button = document.createElement("button");

  block.className = "code-block";
  button.type = "button";
  button.className = "code-copy";
  button.textContent = COPY_LABEL;
  button.setAttribute("aria-label", COPY_NAME);
  pre.replaceWith(block);
  block.append(pre, button);
}

/**
 * Enhances every code block under `root` and handles their Copy buttons; returns the cleanup,
 * after which late results are dropped. A body without a `<pre>` loads nothing.
 */
export function enhanceCodeBlocks(
  root: HTMLElement,
  tokenizer: Tokenizer = defaultTokenizer,
): () => void {
  let live = true;
  const resets = new Map<HTMLButtonElement, number>();

  const resetCopy = (button: HTMLButtonElement) => {
    window.clearTimeout(resets.get(button));
    resets.delete(button);
    button.textContent = COPY_LABEL;
    button.setAttribute("aria-label", COPY_NAME);
    delete button.dataset.copied;
  };

  const showCopied = (button: HTMLButtonElement) => {
    window.clearTimeout(resets.get(button));
    button.textContent = "Copied";
    button.setAttribute("aria-label", "Copied");
    button.dataset.copied = "";
    resets.set(
      button,
      window.setTimeout(() => resetCopy(button), COPIED_MS),
    );
  };

  const onClick = (event: MouseEvent) => {
    const button =
      event.target instanceof Element
        ? event.target.closest<HTMLButtonElement>("button.code-copy")
        : null;

    const pre = button?.parentElement?.querySelector<HTMLElement>(":scope > pre");

    if (button === null || button === undefined || pre === null || pre === undefined) {
      return;
    }

    event.preventDefault();

    const text = codeOf(pre).textContent;

    // Through a promise, so a page without a clipboard (an insecure origin) reaches the toast too.
    Promise.resolve(text)
      .then((source) => navigator.clipboard.writeText(source))
      .then(
        () => {
          if (live) {
            showCopied(button);
          }
        },
        () => toast({ title: "Couldn't copy to the clipboard", tone: "danger" }),
      );
  };

  for (const pre of root.querySelectorAll<HTMLElement>("pre")) {
    addCopyButton(pre);
    colour(pre, tokenizer, () => live);
  }

  root.addEventListener("click", onClick);

  return () => {
    live = false;
    root.removeEventListener("click", onClick);

    for (const button of [...resets.keys()]) {
      resetCopy(button);
    }
  };
}
