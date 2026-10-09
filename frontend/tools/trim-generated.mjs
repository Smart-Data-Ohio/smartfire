import { readdirSync, readFileSync, writeFileSync } from "node:fs";

const directory = new URL("../src/gen/", import.meta.url);

// ts-rs leaves spaces before line breaks when a field has a doc comment.
for (const name of readdirSync(directory).filter((name) => name.endsWith(".ts"))) {
  const file = new URL(name, directory);
  const source = readFileSync(file, "utf8");
  const trimmed = source.replace(/[\t ]+$/gm, "");

  if (trimmed !== source) {
    writeFileSync(file, trimmed);
  }
}
