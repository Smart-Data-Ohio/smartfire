import { Schema } from "effect";
import tables from "../../crates/rails_compat/src/unicode_tables.json" with { type: "json" };

const downcase = new Map(
  Schema.decodeUnknownSync(Schema.Array(Schema.Tuple([Schema.Int, Schema.String])))(
    tables.maps.downcase,
  ),
);

function nameKey(name: string): number[] {
  const lowered = Array.from(
    name,
    (letter) => downcase.get(letter.codePointAt(0) ?? 0) ?? letter,
  ).join("");

  return Array.from(lowered, (letter) => letter.codePointAt(0) ?? 0);
}

export function sidebarDirectLabel(customName: string | null, names: readonly string[]): string {
  return directLabel(customName, names, (name) => name);
}

export function directRoomLabel(customName: string | null, names: readonly string[]): string {
  return directLabel(
    customName,
    names,
    (name) => name.split(/[ \t\n\r\v\f]/).find((part) => part !== "") ?? "",
  );
}

function directLabel(
  customName: string | null,
  names: readonly string[],
  labelName: (name: string) => string,
): string {
  if (customName !== null && customName.trim() !== "") return customName;

  const ordered = names
    .map((name) => ({ name, key: nameKey(name) }))
    .sort((a, b) => {
      for (let index = 0; index < Math.min(a.key.length, b.key.length); index++) {
        const difference = (a.key[index] ?? 0) - (b.key[index] ?? 0);

        if (difference !== 0) return difference;
      }

      return a.key.length - b.key.length;
    });

  const label = ordered
    .slice(0, 3)
    .map(({ name }) => labelName(name))
    .join(", ");

  return label + (ordered.length > 3 ? ` +${ordered.length - 3}` : "");
}
