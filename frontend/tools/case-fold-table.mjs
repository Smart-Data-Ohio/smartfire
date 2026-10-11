// Prints FOLDS for src/lib/case-fold.ts: the whole full case folding table (CaseFolding.txt C+F)
// of the `caseless` crate the server compares tag names with, so the SPA folds the same way on
// any JavaScript runtime, whatever Unicode version its own toLowerCase knows. Run it on the
// crate's data file:
//   node tools/case-fold-table.mjs ~/.cargo/registry/src/*/caseless-<version>/src/case_folding_data.rs
import { readFileSync } from "node:fs";

const source = readFileSync(process.argv[2], "utf8");

const version = /UNICODE_VERSION: \(u64, u64, u64\) = \((\d+), (\d+), (\d+)\)/u.exec(source);

const CHAR = String.raw`(\\u\{[0-9a-f]+\}|\\.|.)`;

const ENTRY = new RegExp(String.raw`\('${CHAR}', \['${CHAR}', '${CHAR}', '${CHAR}'\]\)`, "gu");

const point = (token) => {
  if (token === "\\0") return undefined;

  if (token.startsWith("\\u{")) return Number.parseInt(token.slice(3, -1), 16);

  return token.replace(/^\\(.)$/u, "$1").codePointAt(0);
};

const entries = [...source.matchAll(ENTRY)]
  .map(([, from, ...to]) => ({
    from: point(from),
    to: to.map(point).filter((part) => part !== undefined),
  }))
  .sort((left, right) => left.from - right.from);

const hex = (value) => (value < 0 ? `-${(-value).toString(16)}` : value.toString(16));

const single = (entry) => entry.to.length === 1;

const delta = (entry) => entry.to[0] - entry.from;

const tokens = [];

let index = 0;

while (index < entries.length) {
  const first = entries[index];
  const next = entries[index + 1];
  const step = next === undefined ? 0 : next.from - first.from;
  let last = index;

  if (single(first) && (step === 1 || step === 2)) {
    while (
      entries[last + 1] !== undefined &&
      single(entries[last + 1]) &&
      entries[last + 1].from - entries[last].from === step &&
      delta(entries[last + 1]) === delta(first)
    ) {
      last += 1;
    }
  }

  if (last === index) {
    tokens.push(`${hex(first.from)}:${first.to.map(hex).join(".")}`);
  } else {
    const span = `${hex(first.from)}-${hex(entries[last].from)}`;

    tokens.push(`${span}${step === 2 ? "/2" : ""}:${hex(delta(first))}`);
  }

  index = last + 1;
}

const lines = [];

for (let at = 0; at < tokens.length; at += 8) {
  lines.push(`  "${tokens.slice(at, at + 8).join(" ")} " +`);
}

console.log(`// Unicode ${version?.slice(1).join(".")}, ${entries.length} code points.`);

console.log(`const FOLDS =\n${lines.join("\n").replace(/ " \+$/, '";')}`);
