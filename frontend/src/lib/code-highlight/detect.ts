import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import c from "highlight.js/lib/languages/c";
import cpp from "highlight.js/lib/languages/cpp";
import csharp from "highlight.js/lib/languages/csharp";
import css from "highlight.js/lib/languages/css";
import diff from "highlight.js/lib/languages/diff";
import dockerfile from "highlight.js/lib/languages/dockerfile";
import go from "highlight.js/lib/languages/go";
import java from "highlight.js/lib/languages/java";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import plaintext from "highlight.js/lib/languages/plaintext";
import powershell from "highlight.js/lib/languages/powershell";
import python from "highlight.js/lib/languages/python";
import ruby from "highlight.js/lib/languages/ruby";
import rust from "highlight.js/lib/languages/rust";
import sql from "highlight.js/lib/languages/sql";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import yaml from "highlight.js/lib/languages/yaml";

/**
 * Guesses an unlabelled block's language as classic does (web/script/code-highlighter/detector.mjs):
 * Highlight.js 11.9.0's auto-detection over the same twenty grammars. Highlight.js only names the
 * language; Shiki colours it. Loaded on its own, only for a block with no `language-` class.
 */
const registry = hljs.newInstance();

for (const [name, grammar] of [
  ["bash", bash],
  ["c", c],
  ["cpp", cpp],
  ["csharp", csharp],
  ["css", css],
  ["diff", diff],
  ["dockerfile", dockerfile],
  ["go", go],
  ["java", java],
  ["javascript", javascript],
  ["json", json],
  ["plaintext", plaintext],
  ["powershell", powershell],
  ["python", python],
  ["ruby", ruby],
  ["rust", rust],
  ["sql", sql],
  ["typescript", typescript],
  ["xml", xml],
  ["yaml", yaml],
] as const) {
  registry.registerLanguage(name, grammar);
}

/** Highlight.js's best guess (`"plaintext"` included), or undefined when nothing matches. */
export function detectLanguage(source: string): string | undefined {
  return registry.highlightAuto(source).language;
}
