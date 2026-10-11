// Prints FOLD_EXCEPTIONS for src/lib/case-fold.ts: every code point whose full case folding (the
// `caseless` crate's CaseFolding.txt C+F table, as the server compares tag names) differs from
// what String.prototype.toLowerCase does to it alone. Run it on the crate's data file:
//   node tools/case-fold-table.mjs ~/.cargo/registry/src/*/caseless-<version>/src/case_folding_data.rs
import { readFileSync } from "node:fs";

const source = readFileSync(process.argv[2], "utf8");

const CHAR = String.raw`(\\u\{[0-9a-f]+\}|\\.|.)`;

const ENTRY = new RegExp(String.raw`\('${CHAR}', \['${CHAR}', '${CHAR}', '${CHAR}'\]\)`, "gu");

const text = (token) => {
  if (token === "\\0") return "";

  if (token.startsWith("\\u{"))
    return String.fromCodePoint(Number.parseInt(token.slice(3, -1), 16));

  return token.replace(/^\\(.)$/u, "$1");
};

const table = new Map();

for (const [, from, ...to] of source.matchAll(ENTRY)) {
  table.set(text(from).codePointAt(0), to.map(text).join(""));
}

const hex = (value) => value.toString(16);

const entries = [];

for (let point = 0; point <= 0x10ffff; point += 1) {
  if (point >= 0xd800 && point <= 0xdfff) continue;

  const char = String.fromCodePoint(point);
  const folded = table.get(point) ?? char;

  if (char.toLowerCase() !== folded) {
    entries.push(`${hex(point)}:${[...folded].map((part) => hex(part.codePointAt(0))).join(".")}`);
  }
}

const lines = [];

for (let index = 0; index < entries.length; index += 9) {
  lines.push(`  "${entries.slice(index, index + 9).join(" ")} " +`);
}

console.log(`const FOLD_EXCEPTIONS =\n${lines.join("\n").replace(/ " \+$/, '";')}`);
